use std::rc::Rc;

use super::display::grid::BoxedWidgetRc;
use super::expose::BoxExposeRc;
use super::BoxCtxRc;
use gtk::DrawingArea;

use crate::ui::draws::mouse_state::{
    new_mouse_event_func, new_mouse_state, MouseEvent, MouseState, MouseStateRc,
};
use crate::ui::draws::transition_state::TransitionStateRc;

pub fn event_handle(
    darea: &DrawingArea,
    expose: BoxExposeRc,
    ts: TransitionStateRc,
    box_ctx: BoxCtxRc,
) -> MouseStateRc {
    let ms = new_mouse_state(darea, MouseState::new(true, false, true, ts.clone()));
    let mut last_widget: Option<BoxedWidgetRc> = None;
    let cb = {
        let f = expose.borrow().update_func();
        new_mouse_event_func(move |e| {
            match e {
                MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                    let box_ctx = box_ctx.borrow();

                    let pos = {
                        let rectint = box_ctx.rec_int; //input_region.as_ref().clone().into_inner();
                        let pos = (pos.0 - rectint.x() as f64, pos.1 - rectint.y() as f64);
                        box_ctx.outlook.transform_mouse_pos(pos)
                    };

                    let matched = box_ctx.item_map.match_item(pos);
                    // unsafe { filtered_grid_item_map.as_ptr().as_ref().unwrap() }.match_item(pos);
                    if let Some((widget, pos)) = matched {
                        if let Some(last) = last_widget.take() {
                            if Rc::ptr_eq(&last, &widget) {
                                widget.borrow_mut().on_mouse_event(MouseEvent::Motion(pos));
                            } else {
                                last.borrow_mut().on_mouse_event(MouseEvent::Leave);
                                widget.borrow_mut().on_mouse_event(MouseEvent::Enter(pos));
                            }
                        } else {
                            widget.borrow_mut().on_mouse_event(MouseEvent::Enter(pos));
                        }
                        last_widget = Some(widget);
                    } else {
                        if let Some(last) = last_widget.take() {
                            last.borrow_mut().on_mouse_event(MouseEvent::Leave);
                        }
                    }
                    if let MouseEvent::Enter(_) = e {
                        f();
                    }
                }
                MouseEvent::Leave => {
                    last_widget = None;
                    f();
                }
                _ => {}
            };
        })
    };
    ms.borrow_mut().set_event_cb(cb);
    ms
}
