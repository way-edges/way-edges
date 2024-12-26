use std::rc::Rc;

use crate::mouse_state::MouseEvent;
use crate::window::WindowContext;
use cairo::ImageSurface;
use config::widgets::button::BtnConfig;
use config::widgets::slide::base::SlideConfig;
use config::Config;
use gtk4_layer_shell::Edge;

fn make_translate_func(w_conf: &SlideConfig, edge: Edge) -> Fn((f64, f64)) -> f64 {
    fn lef_or_right(size: (i32, i32), pos: (f64, f64)) -> f64 {}
}

pub(super) fn setup_event(
    window: &mut WindowContext,
    conf: &Config,
    w_conf: &mut SlideConfig,
    draw_func: Rc<impl Fn(f64) -> ImageSurface>,
) {
    let mut event_map = std::mem::take(&mut w_conf.event_map);

    // NOTE: THIS TYPE ANNOTATION IS WEIRD
    window.set_draw_func(None::<fn() -> Option<ImageSurface>>);

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
            if let Some(cb) = event_map.get_mut(&k) {
                cb();
            };
        }
        false
    });
}
