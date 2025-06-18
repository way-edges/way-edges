mod draw;
mod event;

use std::{cell::Cell, rc::Rc};

use crate::{
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
};
use backend::workspace::{
    hypr::{register_hypr_event_callback, HyprConf},
    niri::register_niri_event_callback,
    WorkspaceCB, WorkspaceData, WorkspaceHandler,
};
use config::widgets::workspace::{WorkspaceConfig, WorkspacePreset};
use draw::DrawConf;
use event::HoverData;
use smithay_client_toolkit::{output::OutputInfo, seat::pointer::BTN_LEFT};

use super::WidgetContext;

pub fn init_widget(
    builder: &mut WidgetBuilder,
    size: (i32, i32),
    mut w_conf: WorkspaceConfig,
    output: &OutputInfo,
) -> impl WidgetContext {
    let edge = builder.common_config.edge;
    w_conf.size.calculate_relative(size, edge);
    if w_conf.output_name.is_none() {
        w_conf.output_name = output.name.clone();
    }

    let workspace_transition = builder.new_animation(
        w_conf.workspace_transition_duration,
        w_conf.workspace_animation_curve,
    );

    let draw_conf = DrawConf::new(&w_conf, workspace_transition.clone(), edge);

    let workspace_data = Rc::new(Cell::new((
        WorkspaceData::default(),
        WorkspaceData::default(),
    )));
    let hover_data = HoverData::new(edge, w_conf.invert_direction);

    let workspace_data_weak = Rc::downgrade(&workspace_data);
    let workspace_transition_weak = workspace_transition.downgrade();
    let pop_signal_sender = builder.make_pop_channel(w_conf.pop_duration, move |_, msg| {
        let Some(workspace_data) = workspace_data_weak.upgrade() else {
            return;
        };
        let Some(workspace_transition) = workspace_transition_weak.upgrade() else {
            return;
        };
        let mut old = workspace_data.get();
        if old.0 != msg {
            old.1 = old.0;
            old.0 = msg;
            workspace_data.set(old);
            workspace_transition.borrow_mut().flip();
        }
    });

    macro_rules! wp_cb {
        ($s:expr, $c:expr, $d:expr) => {
            WorkspaceCB {
                sender: $s,
                output: $c.output_name.take().unwrap(),
                data: $d,
                focused_only: $c.focused_only,
            }
        };
    }

    let workspace_handler = match w_conf.preset.clone() {
        WorkspacePreset::Hyprland => {
            register_hypr_event_callback(wp_cb!(pop_signal_sender, w_conf, HyprConf))
        }
        WorkspacePreset::Niri(niri_conf) => {
            register_niri_event_callback(wp_cb!(pop_signal_sender, w_conf, niri_conf))
        }
    };

    WorkspaceCtx {
        workspace_handler,
        draw_conf,
        workspace_data,
        hover_data,
    }
}

#[derive(Debug)]
pub struct WorkspaceCtx {
    workspace_handler: WorkspaceHandler,
    draw_conf: DrawConf,
    workspace_data: Rc<Cell<(WorkspaceData, WorkspaceData)>>,
    hover_data: HoverData,
}
impl WidgetContext for WorkspaceCtx {
    fn redraw(&mut self) -> cairo::ImageSurface {
        let d = self.workspace_data.get();
        self.draw_conf.draw(d.0, d.1, &mut self.hover_data)
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
                if key == BTN_LEFT {
                    should_redraw = hhh!(self.hover_data, pos);
                    let id = self.hover_data.hover_id;
                    if id >= 0 {
                        self.workspace_handler.change_to_workspace(id as usize);
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
            MouseEvent::Scroll(_, v) => {
                let (d, _) = self.workspace_data.get();
                let id = d.active;
                let new_id = id + (v / v.abs()).ceil() as i32;
                if new_id == new_id.clamp(0, d.workspace_count - 1) {
                    self.workspace_handler.change_to_workspace(new_id as usize);
                    should_redraw = true;
                }
            }
            _ => {}
        };
        should_redraw
    }
}
