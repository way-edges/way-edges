use crate::config::widgets::common::EventMap;
use crate::ui::draws::mouse_state::new_mouse_state;
use crate::ui::draws::mouse_state::new_translate_mouse_state;
use crate::ui::draws::mouse_state::MouseState;
use crate::ui::draws::mouse_state::MouseStateCbs;
use crate::ui::draws::transition_state::TransitionStateRc;

use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn setup_event(
    darea: &DrawingArea,
    mut event_map: EventMap,
    ts: TransitionStateRc,
) -> Rc<RefCell<MouseState>> {
    let ms = new_mouse_state(darea);
    let mut cbs = MouseStateCbs::new();
    cbs.set_unpress_cb(glib::clone!(
        #[weak]
        darea,
        move |_, k| {
            if let Some(cb) = event_map.get_mut(&k) {
                cb();
            };
            darea.queue_draw();
        }
    ));
    cbs.set_hover_enter_cb(glib::clone!(
        #[weak]
        darea,
        move |_| {
            darea.queue_draw();
        }
    ));
    cbs.set_hover_leave_cb(glib::clone!(
        #[weak]
        darea,
        move || {
            darea.queue_draw();
        }
    ));
    let mut cbs = new_translate_mouse_state(ts, ms.clone(), Some(cbs), true);
    cbs.set_press_cb(glib::clone!(
        #[weak]
        darea,
        move |_, _| {
            darea.queue_draw();
        }
    ));
    ms.borrow_mut().set_cbs(cbs);
    ms
}
