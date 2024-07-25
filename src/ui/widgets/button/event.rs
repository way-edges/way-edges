use crate::config::widgets::common::EventMap;
use crate::ui::draws::mouse_state::new_mouse_event_func;
use crate::ui::draws::mouse_state::new_mouse_state;
use crate::ui::draws::mouse_state::new_translate_mouse_state;
use crate::ui::draws::mouse_state::MouseEvent;
use crate::ui::draws::mouse_state::MouseState;
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
    let cb = new_mouse_event_func(glib::clone!(
        #[weak]
        darea,
        move |e| {
            match e {
                MouseEvent::Release(_, k) => {
                    if let Some(cb) = event_map.get_mut(&k) {
                        cb();
                    };
                }
                _ => {}
            };
            darea.queue_draw();
        }
    ));
    let cb = new_translate_mouse_state(ts, ms.clone(), Some(cb), true);
    ms.borrow_mut().set_event_cb(cb);
    ms
}
