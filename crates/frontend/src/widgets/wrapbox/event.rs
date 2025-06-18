use util::Or;

use super::{
    box_traits::{BoxedWidgetCtxRc, BoxedWidgetCtxRcWeak, BoxedWidgetGrid},
    outlook::OutlookDraw,
    BoxContext,
};
use crate::mouse_state::MouseEvent;

/// last hover widget, for trigger mouse leave option for that widget.
#[derive(Debug)]
pub struct LastWidget {
    press_lock: bool,
    current_widget: Option<BoxedWidgetCtxRcWeak>,
}
impl LastWidget {
    pub fn new() -> Self {
        Self {
            press_lock: false,
            current_widget: None,
        }
    }

    fn set_current(&mut self, w: BoxedWidgetCtxRc, pos: (f64, f64)) -> bool {
        if self.press_lock {
            return false;
        }

        let mut redraw = Or(false);

        if let Some(last) = self.current_widget.take() {
            let last = last.upgrade().unwrap();
            if last == w {
                // if same widget
                redraw.or(w.borrow_mut().on_mouse_event(MouseEvent::Motion(pos)));
            } else {
                // not same widget
                redraw.or(last.borrow_mut().on_mouse_event(MouseEvent::Leave));
                redraw.or(w.borrow_mut().on_mouse_event(MouseEvent::Enter(pos)));
            }
        } else {
            // if no last widget
            redraw.or(w.borrow_mut().on_mouse_event(MouseEvent::Enter(pos)));
        }
        self.current_widget = Some(w.downgrade());

        redraw.res()
    }

    fn dispose_current(&mut self) -> bool {
        // here we trust that press_lock from MouseState works fine:
        // won't trigger `leave` while pressing
        if self.press_lock {
            return false;
        }

        if let Some(last) = self.current_widget.take() {
            last.upgrade()
                .unwrap()
                .borrow_mut()
                .on_mouse_event(MouseEvent::Leave)
        } else {
            false
        }
    }

    fn lock_press(&mut self) {
        self.press_lock = true
    }
    fn release_press(&mut self) {
        self.press_lock = false
    }
    fn take_current(&mut self) -> Option<BoxedWidgetCtxRc> {
        self.current_widget.take().map(|w| w.upgrade().unwrap())
    }
}

fn match_item(
    grid_box: &BoxedWidgetGrid,
    outlook_mouse_pos: &dyn OutlookDraw,
    pos: (f64, f64),
) -> Option<(BoxedWidgetCtxRc, (f64, f64))> {
    let pos = outlook_mouse_pos.translate_mouse_position(pos);

    grid_box
        .match_item(pos)
        .map(|(widget, pos)| (widget.clone(), pos))
}

pub fn on_mouse_event(event: MouseEvent, ctx: &mut BoxContext) -> bool {
    let mut redraw = Or(false);

    match event {
        MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
            let matched = match_item(&ctx.grid_box, ctx.outlook_draw_conf.as_ref(), pos);

            if let Some((widget, pos)) = matched {
                redraw.or(ctx.last_widget.set_current(widget, pos));
            } else {
                redraw.or(ctx.last_widget.dispose_current());
            }

            ctx.leave_box_state = false;
        }
        MouseEvent::Leave => {
            redraw.or(ctx.last_widget.dispose_current());
            ctx.leave_box_state = true;
        }
        MouseEvent::Press(pos, k) => {
            let matched = match_item(&ctx.grid_box, ctx.outlook_draw_conf.as_ref(), pos);
            if let Some((widget, pos)) = matched {
                ctx.last_widget.lock_press();
                redraw.or(widget
                    .borrow_mut()
                    .on_mouse_event(MouseEvent::Press(pos, k)));
            }
        }
        MouseEvent::Release(pos, k) => {
            ctx.last_widget.release_press();

            let matched = match_item(&ctx.grid_box, ctx.outlook_draw_conf.as_ref(), pos);
            if let Some((widget, pos)) = matched {
                redraw.or(widget
                    .borrow_mut()
                    .on_mouse_event(MouseEvent::Release(pos, k)));
            } else if ctx.leave_box_state {
                ctx.leave_box_state = false;
                if let Some(last) = ctx.last_widget.take_current() {
                    let mut last = last.borrow_mut();
                    redraw.or(last.on_mouse_event(MouseEvent::Leave));
                    redraw.or(last.on_mouse_event(MouseEvent::Release(pos, k)));
                }
            }
        }
        common_event => {
            if let Some(last) = ctx.last_widget.take_current() {
                redraw.or(last.borrow_mut().on_mouse_event(common_event));
            }
        }
    };

    redraw.res()
}
