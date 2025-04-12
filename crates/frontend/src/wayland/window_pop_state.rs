use std::rc::Rc;

use crate::animation::{ToggleAnimationRc, ToggleDirection};

#[derive(Debug)]
pub struct WindowPopState {
    pinnale: bool,
    pin_with_key: bool,
    pin_key: u32,
    pub pin_state: bool,
    pub pop_state: Option<Rc<()>>,
    pub pop_animation: ToggleAnimationRc,
}
impl WindowPopState {
    pub fn new(ani: ToggleAnimationRc, pinnale: bool, pin_with_key: bool, pin_key: u32) -> Self {
        Self {
            pin_state: false,
            pop_state: None,
            pop_animation: ani,
            pin_key,
            pinnale,
            pin_with_key,
        }
    }
    pub fn invalidate_pop(&mut self) {
        drop(self.pop_state.take());
    }
    pub fn toggle_pin(&mut self, is_hovering: bool) {
        if !self.pinnale {
            return;
        }

        self.invalidate_pop();
        let state = !self.pin_state;
        self.pin_state = state;
        if is_hovering {
            return;
        }
        self.pop_animation.borrow_mut().set_direction(state.into());
    }
    pub fn toggle_pin_with_key(&mut self, key: u32, is_hovering: bool) -> bool {
        if !self.pin_with_key || key != self.pin_key {
            return false;
        }
        self.toggle_pin(is_hovering);
        true
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
