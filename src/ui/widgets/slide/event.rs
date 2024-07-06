use gtk::gdk::BUTTON_MIDDLE;

use crate::ui::draws::{
    mouse_state::{check_transition_direction, BaseMouseState, PinState, PopState},
    transition_state::TransitionState,
};

pub struct MouseState {
    pub base: BaseMouseState,
    pub pin_state: PinState,
    pub pop_state: PopState,
}

impl MouseState {
    pub fn new(ts: &TransitionState<f64>) -> Self {
        Self {
            base: BaseMouseState::new(ts),
            pin_state: PinState::new(),
            pop_state: PopState::new(),
        }
    }
    pub fn set_hovering(&mut self, h: bool) {
        self.base.set_hovering(h);
        if !self.pin_state.is_pinned() {
            self.ensure();
        }
    }
    pub fn set_pressing(&mut self, p: u32) {
        self.base.set_pressing(p);
        // pin state
        if p == BUTTON_MIDDLE {
            if self.pin_state.toggle_pin() {
                self.set_transition(true);
            } else {
                self.ensure();
            }
        };
    }
    pub fn take_pressing(&mut self) -> Option<u32> {
        let key = self.base.take_pressing();
        if !self.pin_state.is_pinned() && key.is_some() {
            self.ensure();
        }
        key
    }

    pub fn ensure(&mut self) -> bool {
        let direction = check_transition_direction(&self.base.hovering, &self.base.pressing.get());
        if direction != self.base.is_forward.get() {
            self.set_transition(direction);
            true
        } else {
            false
        }
    }

    pub fn set_transition(&mut self, open: bool) {
        // invalidate pop callback when transition_state is modified by other event
        self.pop_state.invalidate();
        self.base.set_transition(open);
    }

    fn pop(&mut self) {
        self.pop_state.pop(move || {
            self.ensure();
        })
    }
}
