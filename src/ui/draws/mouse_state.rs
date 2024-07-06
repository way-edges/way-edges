use gtk::glib;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use super::transition_state::TransitionState;

pub struct BaseMouseState {
    pub hovering: bool,
    pub pressing: Rc<Cell<Option<u32>>>,

    // transition_state related
    pub t: Rc<Cell<Instant>>,
    pub is_forward: Rc<Cell<bool>>,
    pub max_time: Duration,
}

impl BaseMouseState {
    pub fn new(ts: &TransitionState<f64>) -> Self {
        Self {
            hovering: false,
            pressing: Rc::new(Cell::new(None)),
            t: ts.t.clone(),
            is_forward: ts.is_forward.clone(),
            max_time: ts.duration,
        }
    }
    pub fn set_hovering(&mut self, h: bool) {
        self.hovering = h;
    }
    pub fn set_pressing(&mut self, p: u32) {
        self.pressing.set(Some(p));
    }
    pub fn take_pressing(&mut self) -> Option<u32> {
        if let Some(old) = self.pressing.take() {
            Some(old)
        } else {
            None
        }
    }

    // return is transition change
    pub fn ensure_transition_direction(&mut self) -> bool {
        let direction = check_transition_direction(&self.hovering, &self.pressing.get());
        if direction != self.is_forward.get() {
            self.set_transition(direction);
            true
        } else {
            false
        }
    }

    pub fn set_transition(&self, open: bool) {
        TransitionState::<f64>::set_direction(&self.t, self.max_time, &self.is_forward, open);
    }
}

/// true -> forward
/// false -> backward
pub fn check_transition_direction(hovering: &bool, pressing: &Option<u32>) -> bool {
    // not hovering and no pressing
    if !*hovering && pressing.is_none() {
        false
    } else {
        true
    }
}

pub struct PinState {
    is_pinned: Rc<Cell<bool>>,
}
impl PinState {
    pub fn new() -> Self {
        Self {
            is_pinned: Rc::new(Cell::new(false)),
        }
    }
    pub fn pin(&self) {
        self.is_pinned.set(true);
    }
    pub fn unpin(&self) {
        self.is_pinned.set(false);
    }
    // return the pin state
    pub fn toggle_pin(&self) -> bool {
        let state = !self.is_pinned.get();
        self.is_pinned.set(state);
        state
    }
    pub fn without_pin(&self, f: impl FnOnce()) {
        if !self.is_pinned.get() {
            f()
        }
    }
    pub fn is_pinned(&self) -> bool {
        self.is_pinned.get()
    }
}

pub struct PopState {
    pub pop: Rc<Cell<Option<Rc<Cell<bool>>>>>,
    timeout: Duration,
}
impl PopState {
    pub fn new() -> Self {
        Self {
            pop: Rc::new(Cell::new(None)),
            timeout: Duration::from_secs(2),
        }
    }
    pub fn pop(&mut self, closefn: impl 'static + FnOnce()) {
        self.invalidate();
        let state = Rc::new(Cell::new(true));
        let state_clone = state.clone();
        self.pop.set(Some(state));
        let pop_clone = self.pop.clone();
        glib::timeout_add_local_once(self.timeout, move || {
            if state_clone.get() {
                pop_clone.set(None);
                closefn();
            }
        });
    }
    pub fn invalidate(&mut self) {
        if let Some(before) = self.pop.take() {
            before.set(false);
        }
    }
}
