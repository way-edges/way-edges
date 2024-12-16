use std::rc::Rc;

use super::display::grid::BoxedWidgetRc;
use super::expose::BoxExpose;
use super::{BoxCtxRc, MousePosition};
use gtk::DrawingArea;

use crate::ui::draws::mouse_state::{
    new_mouse_event_func, new_mouse_state, MouseEvent, MouseState, MouseStateRc,
};
use crate::ui::draws::transition_state::TransitionStateRc;

/// last hover widget, for trigger mouse leave option for that widget.
struct LastWidget {
    press_lock: bool,
    current_widget: Option<BoxedWidgetRc>,
}
impl LastWidget {
    fn new() -> Self {
        Self {
            press_lock: false,
            current_widget: None,
        }
    }

    fn set_current(&mut self, w: BoxedWidgetRc, pos: (f64, f64)) {
        if self.press_lock {
            return;
        }

        if let Some(last) = self.current_widget.take() {
            if Rc::ptr_eq(&last, &w) {
                // if same widget
                w.borrow_mut().on_mouse_event(MouseEvent::Motion(pos));
            } else {
                // not same widget
                last.borrow_mut().on_mouse_event(MouseEvent::Leave);
                w.borrow_mut().on_mouse_event(MouseEvent::Enter(pos));
            }
        } else {
            // if no last widget
            w.borrow_mut().on_mouse_event(MouseEvent::Enter(pos));
        }
        self.current_widget = Some(w);
    }

    fn dispose_current(&mut self) {
        // here we trust that press_lock from MouseState works fine:
        // won't trigger `leave` while pressing
        if self.press_lock {
            return;
        }

        if let Some(last) = self.current_widget.take() {
            last.borrow_mut().on_mouse_event(MouseEvent::Leave);
        }
    }

    fn set_press_lock(&mut self, press_lock: bool) {
        self.press_lock = press_lock
    }
}

pub fn event_handle(
    darea: &DrawingArea,
    expose: BoxExpose,
    ts: TransitionStateRc,
    box_ctx: BoxCtxRc,
) -> MouseStateRc {
    let ms = new_mouse_state(darea, MouseState::new(true, false, true, ts.clone()));

    // last hover widget, for trigger mouse leave option for that widget.
    let mut last_widget = LastWidget::new();

    // because mouse leave event is before release,
    // we need to check if unpress is right behind leave
    let mut leave_box_state = false;

    let cb = {
        let f = expose.update_func();
        new_mouse_event_func(move |e| {
            let mut should_redraw = false;

            match e {
                MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                    let matched = match_item(&box_ctx, pos);

                    if let Some((widget, pos)) = matched {
                        last_widget.set_current(widget, pos);
                    } else {
                        last_widget.dispose_current();
                    }

                    if let MouseEvent::Enter(_) = e {
                        // show box
                        should_redraw = true;
                    }

                    leave_box_state = false;
                }
                MouseEvent::Leave => {
                    last_widget.dispose_current();
                    leave_box_state = true;

                    // hide box
                    should_redraw = true;
                }
                MouseEvent::Press(pos, k) => {
                    let matched = match_item(&box_ctx, pos);
                    if let Some((widget, pos)) = matched {
                        last_widget.set_press_lock(true);
                        widget
                            .borrow_mut()
                            .on_mouse_event(MouseEvent::Press(pos, k));
                    }

                    // hide box
                    should_redraw = true;
                }
                MouseEvent::Release(pos, k) => {
                    last_widget.set_press_lock(false);

                    let matched = match_item(&box_ctx, pos);
                    if let Some((widget, pos)) = matched {
                        widget
                            .borrow_mut()
                            .on_mouse_event(MouseEvent::Release(pos, k));
                    } else if leave_box_state {
                        leave_box_state = false;
                        if let Some(last) = last_widget.current_widget.take() {
                            let mut last = last.borrow_mut();
                            last.on_mouse_event(MouseEvent::Leave);
                            last.on_mouse_event(MouseEvent::Release(pos, k));
                        }
                        last_widget.dispose_current();
                    }

                    // hide box
                    should_redraw = true;
                }
                // pin/unpin pop/unpop
                _ => {
                    should_redraw = true;
                }
            };

            if should_redraw {
                f();
            }
        })
    };
    ms.borrow_mut().set_event_cb(cb);
    ms
}

fn match_item(box_ctx: &BoxCtxRc, pos: (f64, f64)) -> Option<(BoxedWidgetRc, MousePosition)> {
    let box_ctx = box_ctx.borrow();

    let pos = {
        let rectint = box_ctx.input_region; //input_region.as_ref().clone().into_inner();
        let pos = (pos.0 - rectint.x() as f64, pos.1 - rectint.y() as f64);
        box_ctx.outlook.transform_mouse_pos(pos)
    };

    box_ctx
        .grid_box
        .position_map
        .as_ref()
        .unwrap()
        .match_item(pos)
        .map(|(widget, pos)| (widget.clone(), pos))
}
