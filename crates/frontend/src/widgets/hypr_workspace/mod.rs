mod draw;
mod event;

use std::{cell::Cell, rc::Rc};

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
    window::WidgetContext,
};
use backend::hypr_workspace::{change_to_workspace, HyprGlobalData};
use config::{widgets::hypr_workspace::HyprWorkspaceConfig, Config};
use draw::DrawConf;
use event::HoverData;
use glib::clone::{Downgrade, Upgrade};
use gtk::{gdk::BUTTON_PRIMARY, glib};

pub fn init_widget(
    builder: &mut WidgetBuilder,
    size: (i32, i32),
    conf: &Config,
    mut w_conf: HyprWorkspaceConfig,
) -> impl WidgetContext {
    w_conf.size.calculate_relative(size, conf.edge);

    let workspace_transition = builder.new_animation(w_conf.workspace_transition_duration);

    let draw_conf = DrawConf::new(&w_conf, workspace_transition.clone(), conf.edge);

    let hypr_data = Rc::new(Cell::new(HyprGlobalData::default()));
    let hover_data = HoverData::new(conf.edge, w_conf.invert_direction);

    let hypr_data_weak = Rc::downgrade(&hypr_data);
    let workspace_transition_weak = workspace_transition.downgrade();
    let pop_signal_sender = builder.make_pop_channel(w_conf.pop_duration, move |_, msg| {
        let Some(hypr_data) = hypr_data_weak.upgrade() else {
            return;
        };
        let Some(workspace_transition) = workspace_transition_weak.upgrade() else {
            return;
        };
        hypr_data.set(msg);
        workspace_transition.borrow_mut().flip();
    });
    let backend_id = backend::hypr_workspace::register_hypr_event_callback(pop_signal_sender);

    HyprWorkspaceCtx {
        backend_id,
        draw_conf,
        hypr_data,
        hover_data,
    }
}

pub struct HyprWorkspaceCtx {
    #[allow(dead_code)]
    backend_id: u32,
    draw_conf: DrawConf,
    hypr_data: Rc<Cell<HyprGlobalData>>,
    hover_data: HoverData,
}
impl WidgetContext for HyprWorkspaceCtx {
    fn redraw(&mut self) -> cairo::ImageSurface {
        self.draw_conf
            .draw(&self.hypr_data.get(), &mut self.hover_data)
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        let mut should_redraw = false;
        macro_rules! hhh {
            ($h:expr, $pos:expr) => {{
                let old = $h.hover_id;
                $h.update_hover_id_with_mouse_position($pos) != old
            }};
        }
        match event {
            MouseEvent::Release(pos, key) => {
                if key == BUTTON_PRIMARY {
                    should_redraw = hhh!(self.hover_data, pos);
                    let id = self.hover_data.hover_id;
                    if id > 0 {
                        change_to_workspace(id as i32);
                    }
                };
            }
            MouseEvent::Enter(pos) => {
                should_redraw = hhh!(self.hover_data, pos);
            }
            MouseEvent::Motion(pos) => {
                should_redraw = hhh!(self.hover_data, pos);
            }
            MouseEvent::Leave => {
                let old = self.hover_data.hover_id;
                if old != -1 {
                    self.hover_data.force_update_hover_id(-1);
                    should_redraw = true;
                }
            }
            _ => {}
        };
        should_redraw
    }
}
