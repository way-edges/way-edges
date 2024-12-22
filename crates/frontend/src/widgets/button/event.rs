use crate::mouse_state::MouseEvent;
use crate::window::WindowContext;
use config::widgets::button::BtnConfig;

use std::cell::Cell;
use std::rc::Rc;

pub(super) fn setup_event(
    window: &mut WindowContext,
    pressing_state: Rc<Cell<bool>>,
    btn_conf: &mut BtnConfig,
) {
    let mut event_map = std::mem::take(&mut btn_conf.event_map);

    window.setup_mouse_event_callback(move |data, event| {
        let mut redraw_true = false;
        let new_pressing_state = data.pressing.is_some();
        if new_pressing_state != pressing_state.get() {
            pressing_state.set(new_pressing_state);
            redraw_true = true;
        }
        if let MouseEvent::Release(_, k) = event {
            if let Some(cb) = event_map.get_mut(&k) {
                cb();
            };
        }
        redraw_true
    });
}
