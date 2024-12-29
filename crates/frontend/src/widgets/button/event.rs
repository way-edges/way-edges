use crate::mouse_state::MouseEvent;
use crate::window::WindowContext;
use cairo::ImageSurface;
use config::widgets::button::BtnConfig;
use config::Config;

use super::draw::make_draw_func;

pub(super) fn setup_event(window: &mut WindowContext, conf: &Config, btn_conf: &mut BtnConfig) {
    let event_map = std::mem::take(&mut btn_conf.event_map);

    // NOTE: THIS TYPE ANNOTATION IS WEIRD
    window.set_draw_func(None::<fn() -> Option<ImageSurface>>);

    let darw_func = make_draw_func(btn_conf, conf.edge);
    let trigger_redraw = window.make_redraw_notifier();

    // initial content
    trigger_redraw(Some(darw_func(false)));

    let mut pressing_state_cache = false;
    window.setup_mouse_event_callback(move |data, event| {
        let new_pressing_state = data.pressing.is_some();
        if new_pressing_state != pressing_state_cache {
            pressing_state_cache = new_pressing_state;
            trigger_redraw(Some(darw_func(new_pressing_state)));
        }

        if let MouseEvent::Release(_, k) = event {
            event_map.call(k);
        }
        false
    });
}
