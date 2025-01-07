use educe::Educe;
use gtk::{glib, prelude::WidgetExt, DrawingArea};
use std::{
    cell::{Cell, RefCell, UnsafeCell},
    rc::{Rc, Weak},
    time::Duration,
};
use way_edges_derive::wrap_rc;

use crate::{
    animation::{ToggleAnimationRc, ToggleDirection},
    mouse_state::{MouseEvent, MouseStateData, MouseStateRc},
};

use super::WidgetContext;

use gtk::gdk::BUTTON_MIDDLE;

type PopStateGuard = Rc<()>;

#[wrap_rc(rc = "pub", normal = "pub(super)")]
#[derive(Educe)]
#[educe(Debug)]
pub struct WindowPopState {
    pin_state: bool,
    pop_state: Rc<UnsafeCell<Option<PopStateGuard>>>,
    pop_animation: ToggleAnimationRc,
    pin_key: u32,
    pop_duration: Duration,
}
impl WindowPopState {
    pub(super) fn new(ani: ToggleAnimationRc, pop_state: Rc<UnsafeCell<Option<Rc<()>>>>) -> Self {
        Self {
            pin_state: false,
            pop_state,
            pop_animation: ani,
            pin_key: BUTTON_MIDDLE,
            pop_duration: Duration::from_secs(1),
        }
    }
    fn invalidate_pop(&mut self) {
        unsafe { drop(self.pop_state.get().as_mut().unwrap().take()) };
    }
    pub fn toggle_pin(&mut self, is_hovering: bool) {
        self.invalidate_pop();
        let state = !self.pin_state;
        self.pin_state = state;
        if is_hovering {
            return;
        }
        self.pop_animation.borrow_mut().set_direction(state.into());
    }
    fn enter(&mut self) {
        self.invalidate_pop();
        if self.pin_state {
            return;
        }
        self.pop_animation
            .borrow_mut()
            .set_direction(ToggleDirection::Forward);
    }
    fn leave(&mut self) {
        self.invalidate_pop();
        if self.pin_state {
            return;
        }
        self.pop_animation
            .borrow_mut()
            .set_direction(ToggleDirection::Backward);
    }
}

pub fn setup_mouse_event_callback(
    darea: &DrawingArea,
    start_pos: &Rc<Cell<(i32, i32)>>,
    mouse_state: &MouseStateRc,
    window_pop_state: &WindowPopStateRc,

    widget: Weak<RefCell<dyn WidgetContext>>,
) {
    let redraw_func = glib::clone!(
        #[weak]
        darea,
        move || {
            darea.queue_draw();
        }
    );

    let cb = glib::clone!(
        #[weak]
        start_pos,
        #[weak]
        window_pop_state,
        move |data: &mut MouseStateData, mut event: MouseEvent| {
            let Some(w) = widget.upgrade() else {
                return;
            };

            let mut trigger_redraw = false;
            let mut do_redraw = || {
                if !trigger_redraw {
                    trigger_redraw = true;
                }
            };

            fn change_pos(pose: &mut (f64, f64), start_pose: (i32, i32)) {
                pose.0 -= start_pose.0 as f64;
                pose.1 -= start_pose.1 as f64;
            }

            match &mut event {
                MouseEvent::Release(pos, _) | MouseEvent::Press(pos, _) => {
                    change_pos(pos, start_pos.get())
                }
                MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                    change_pos(pos, start_pos.get())
                }
                MouseEvent::Leave => {}
            }

            match event {
                MouseEvent::Release(_, key) => {
                    let mut window_pop_state = window_pop_state.borrow_mut();
                    if key == window_pop_state.pin_key {
                        window_pop_state.toggle_pin(data.hovering);
                        do_redraw()
                    };
                }
                MouseEvent::Enter(_) => {
                    window_pop_state.borrow_mut().enter();
                    do_redraw()
                }
                MouseEvent::Leave => {
                    window_pop_state.borrow_mut().leave();
                    do_redraw()
                }
                MouseEvent::Motion(_) => window_pop_state.borrow_mut().invalidate_pop(),
                _ => {}
            }

            let widget_trigger_redraw = w.borrow_mut().on_mouse_event(data, event);

            if trigger_redraw || widget_trigger_redraw {
                redraw_func()
            }
        }
    );

    mouse_state.borrow_mut().set_event_cb(cb);
}
