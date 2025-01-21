use std::{cell::UnsafeCell, rc::Rc, time::Duration};

use smithay_client_toolkit::seat::pointer::BTN_MIDDLE;

use crate::animation::{ToggleAnimationRc, ToggleDirection};

#[derive(Debug)]
pub struct WindowPopState {
    pub pin_state: bool,
    pub pop_state: Rc<UnsafeCell<Option<Rc<()>>>>,
    pub pop_animation: ToggleAnimationRc,
    pub pin_key: u32,
    pub pop_duration: Duration,
}
impl WindowPopState {
    pub fn new(ani: ToggleAnimationRc, pop_state: Rc<UnsafeCell<Option<Rc<()>>>>) -> Self {
        Self {
            pin_state: false,
            pop_state,
            pop_animation: ani,
            pin_key: BTN_MIDDLE,
            pop_duration: Duration::from_secs(1),
        }
    }
    pub fn invalidate_pop(&mut self) {
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
    pub fn enter(&mut self) {
        self.invalidate_pop();
        if self.pin_state {
            return;
        }
        self.pop_animation
            .borrow_mut()
            .set_direction(ToggleDirection::Forward);
    }
    pub fn leave(&mut self) {
        self.invalidate_pop();
        if self.pin_state {
            return;
        }
        self.pop_animation
            .borrow_mut()
            .set_direction(ToggleDirection::Backward);
    }
}
