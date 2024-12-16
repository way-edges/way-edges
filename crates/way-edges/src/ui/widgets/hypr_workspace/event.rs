use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::WidgetExt;
use gtk::DrawingArea;
use gtk::{gdk::BUTTON_PRIMARY, glib};

use crate::ui::draws::mouse_state::{MouseState, MouseStateRc};
use crate::ui::draws::{
    mouse_state::{new_mouse_event_func, new_mouse_state, MouseEvent},
    transition_state::TransitionStateRc,
};
use backend::hypr_workspace::change_to_workspace;

use super::draw::HoverData;

pub fn setup_event(
    pop_ts: &TransitionStateRc,
    darea: &DrawingArea,
    hover_data: &Rc<RefCell<HoverData>>,
) -> MouseStateRc {
    let mouse_state = new_mouse_state(darea, MouseState::new(true, true, true, pop_ts.clone()));

    let cb = new_mouse_event_func(glib::clone!(
        #[weak]
        darea,
        #[weak]
        hover_data,
        move |e| {
            match e {
                MouseEvent::Press(_, _) => return,
                MouseEvent::Release(_, key) => {
                    if key == BUTTON_PRIMARY {
                        let id = hover_data.borrow().hover_id;
                        // set workspace
                        if id > 0 {
                            change_to_workspace(id as i32);
                        }
                    };
                }
                MouseEvent::Enter(pos) => {
                    hover_data
                        .borrow_mut()
                        .update_hover_id_with_mouse_position(pos);
                    darea.queue_draw();
                }
                MouseEvent::Motion(pos) => {
                    let mut h = hover_data.borrow_mut();
                    let old = h.hover_id;
                    if h.update_hover_id_with_mouse_position(pos) != old {
                        darea.queue_draw();
                    }
                }
                MouseEvent::Leave => {
                    hover_data.borrow_mut().force_update_hover_id(-1);
                    darea.queue_draw();
                }
                _ => {
                    // pin || unpin || pop || unpop
                    darea.queue_draw();
                }
            };
        }
    ));
    mouse_state.borrow_mut().set_event_cb(cb);

    mouse_state
}
