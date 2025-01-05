mod draw;
mod event;

use std::{cell::Cell, rc::Rc};

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    window::{WidgetContext, WindowContextBuilder},
};
use backend::hypr_workspace::{change_to_workspace, HyprGlobalData};
use config::{widgets::hypr_workspace::HyprWorkspaceConfig, Config};
use draw::DrawConf;
use event::HoverData;
use gtk::{
    gdk::{Monitor, BUTTON_PRIMARY},
    glib,
    prelude::MonitorExt,
};

pub fn init_widget(
    window: &mut WindowContextBuilder,
    monitor: &Monitor,
    conf: Config,
    mut w_conf: HyprWorkspaceConfig,
) -> impl WidgetContext {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    w_conf.size.calculate_relative(size, conf.edge);

    let workspace_transition = window.new_animation(w_conf.workspace_transition_duration);

    let draw_conf = DrawConf::new(&w_conf, workspace_transition.clone(), conf.edge);

    let hypr_data = Rc::new(Cell::new(HyprGlobalData::default()));
    let hover_data = HoverData::new(conf.edge, w_conf.invert_direction);

    let pop_func = window.make_pop_func();
    let backend_id = backend::hypr_workspace::register_hypr_event_callback(glib::clone!(
        #[weak]
        hypr_data,
        #[weak]
        workspace_transition,
        move |data| {
            hypr_data.set(*data);
            workspace_transition.borrow_mut().flip();
            pop_func()
        }
    ));

    HyprWorkspaceCtx {
        backend_id,
        draw_conf,
        hypr_data,
        hover_data,
    }
}

pub struct HyprWorkspaceCtx {
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
