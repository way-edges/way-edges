use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::WidgetExt;
use gtk::DrawingArea;
use gtk::{gdk::BUTTON_PRIMARY, glib};

use crate::plug::hypr_workspace::change_to_workspace;
use crate::ui::draws::mouse_state::{MouseStateRc, TranslateStateRc};
use crate::ui::draws::{
    mouse_state::{new_mouse_event_func, new_mouse_state, new_translate_mouse_state, MouseEvent},
    transition_state::TransitionStateRc,
};

use super::draw::DrawData;

pub fn setup_event(
    pop_ts: &TransitionStateRc,
    darea: &DrawingArea,
    workspace_draw_data: &Rc<Cell<DrawData>>,
    hover_id: &Rc<Cell<isize>>,
) -> (MouseStateRc, TranslateStateRc) {
    let mouse_state = new_mouse_state(darea);

    let a = new_mouse_event_func(glib::clone!(
        #[weak]
        darea,
        #[weak]
        workspace_draw_data,
        #[weak]
        hover_id,
        move |e| {
            fn get_pos(workspace_draw_data: &Rc<Cell<DrawData>>, pos: (f64, f64)) -> isize {
                unsafe {
                    workspace_draw_data
                        .as_ptr()
                        .as_ref()
                        .unwrap()
                        .match_workspace(pos)
                        + 1
                }
            }
            match e {
                MouseEvent::Press(_, _) => return,
                MouseEvent::Release(pos, key) => {
                    if key == BUTTON_PRIMARY {
                        let pos = get_pos(&workspace_draw_data, pos);
                        // set workspace
                        if pos > 0 {
                            change_to_workspace(pos as i32);
                        }
                    };
                }
                MouseEvent::Enter(pos) => {
                    hover_id.set(get_pos(&workspace_draw_data, pos));
                    darea.queue_draw();
                }
                MouseEvent::Motion(pos) => {
                    let pos = get_pos(&workspace_draw_data, pos);
                    if hover_id.get() != pos {
                        hover_id.set(pos);
                        darea.queue_draw();
                    };
                }
                MouseEvent::Leave => {
                    hover_id.set(-1);
                    darea.queue_draw();
                }
            };
        }
    ));
    let (cb, translate_state) =
        new_translate_mouse_state(pop_ts.clone(), mouse_state.clone(), Some(a), false);
    mouse_state.borrow_mut().set_event_cb(cb);

    (mouse_state, translate_state)
}
