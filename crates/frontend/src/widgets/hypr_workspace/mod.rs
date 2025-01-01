mod draw;
mod event;

use std::{cell::Cell, rc::Rc};

use crate::window::WindowContext;
use backend::hypr_workspace::HyprGlobalData;
use config::{widgets::hypr_workspace::HyprWorkspaceConfig, Config};
use event::HoverData;
use gtk::{gdk::Monitor, glib, prelude::MonitorExt};

pub fn init_widget(
    window: &mut WindowContext,
    monitor: &Monitor,
    conf: Config,
    mut w_conf: HyprWorkspaceConfig,
) {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());
    w_conf.size.calculate_relative(size, conf.edge);

    let workspace_transition = window.new_animation(w_conf.workspace_transition_duration);
    let draw_func = draw::make_draw_func(&w_conf, conf.edge, workspace_transition.clone());

    let hypr_data = Rc::new(Cell::new(HyprGlobalData::default()));
    let hover_data = HoverData::new(conf.edge, w_conf.invert_direction).make_rc();
    window.set_draw_func(Some(glib::clone!(
        #[weak]
        hypr_data,
        #[weak]
        hover_data,
        #[upgrade_or]
        None,
        move || {
            let img = draw_func(hypr_data.get(), hover_data.clone());
            Some(img)
        }
    )));

    let redraw_signal = window.make_redraw_notifier();
    let window_pop_state = &window.window_pop_state;
    let backend_id = backend::hypr_workspace::register_hypr_event_callback(glib::clone!(
        #[weak]
        window_pop_state,
        move |data| {
            hypr_data.set(*data);
            workspace_transition.borrow_mut().flip();
            window_pop_state.pop();
            redraw_signal(None)
        }
    ));

    struct HyprWorkspaceCtx(u32);
    impl Drop for HyprWorkspaceCtx {
        fn drop(&mut self) {
            backend::hypr_workspace::unregister_hypr_event_callback(self.0);
        }
    }
    window.bind_context(HyprWorkspaceCtx(backend_id));

    event::setup_event(window, hover_data);
}
