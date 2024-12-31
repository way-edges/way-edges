use std::{cell::RefCell, rc::Rc};

use crate::{mouse_state::MouseEvent, window::WindowContext};

use super::{
    box_traits::{BoxedWidgetGrid, BoxedWidgetRc},
    outlook::OutlookMousePositionTranslateion,
};

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

    fn lock_press(&mut self) {
        self.press_lock = true
    }
    fn release_press(&mut self) {
        self.press_lock = false
    }
}

pub fn event_handle(
    window: &mut WindowContext,
    grid_box: &Rc<RefCell<BoxedWidgetGrid>>,
    outlook_mouse_pos: impl OutlookMousePositionTranslateion + 'static,
) {
    // last hover widget, for trigger mouse leave option for that widget.
    let mut last_widget = LastWidget::new();

    // because mouse leave event is before release,
    // we need to check if unpress is right behind leave
    let mut leave_box_state = false;

    use gtk::glib;
    window.setup_mouse_event_callback(glib::clone!(
        #[weak]
        grid_box,
        #[upgrade_or]
        false,
        move |_, e| {
            match e {
                MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                    let matched = match_item(&grid_box, &outlook_mouse_pos, pos);

                    if let Some((widget, pos)) = matched {
                        last_widget.set_current(widget, pos);
                    } else {
                        last_widget.dispose_current();
                    }

                    leave_box_state = false;
                }
                MouseEvent::Leave => {
                    last_widget.dispose_current();
                    leave_box_state = true;
                }
                MouseEvent::Press(pos, k) => {
                    let matched = match_item(&grid_box, &outlook_mouse_pos, pos);
                    if let Some((widget, pos)) = matched {
                        last_widget.lock_press();
                        widget
                            .borrow_mut()
                            .on_mouse_event(MouseEvent::Press(pos, k));
                    }
                }
                MouseEvent::Release(pos, k) => {
                    last_widget.release_press();

                    let matched = match_item(&grid_box, &outlook_mouse_pos, pos);
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
                }
            };

            false
        }
    ));
}

fn match_item(
    grid_box: &Rc<RefCell<BoxedWidgetGrid>>,
    outlook_mouse_pos: &impl OutlookMousePositionTranslateion,
    pos: (f64, f64),
) -> Option<(BoxedWidgetRc, (f64, f64))> {
    let box_ctx = grid_box.borrow();

    let pos = outlook_mouse_pos.translate_mouse_position(pos);

    box_ctx
        .match_item(pos)
        .map(|(widget, pos)| (widget.ctx.clone(), pos))
}
