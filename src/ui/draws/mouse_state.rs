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

    pub pop_state: Rc<Cell<Option<Rc<Cell<bool>>>>>,
    timeout: Duration,
    is_pinned: Rc<Cell<bool>>,

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
            pop_state: Rc::new(Cell::new(None)),
            timeout: Duration::from_secs(2),
            is_pinned: Rc::new(Cell::new(false)),

            t: ts.t.clone(),
            is_forward: ts.is_forward.clone(),
            max_time: ts.duration,
        }
    }
    pub fn set_hovering(&mut self, h: bool) {
        self.hovering = h;
        self.ensure_transition_direction();
    }
    pub fn set_pressing(&self, p: Option<u32>) -> Option<u32> {
        let a = self.pressing.replace(p);
        self.ensure_transition_direction();
        a
    }
    /// return is transition change
    pub fn ensure_transition_direction(&self) -> bool {
        if !self.is_pinned() {
            let direction = check_transition_direction(&self.hovering, &self.pressing.get());
            if direction != self.is_forward.get() {
                self.set_transition(direction);
                return true;
            };
        };
        false
    }
    pub fn set_transition(&self, open: bool) {
        self.invalidate_pop();
        set_transition(&self.t, self.max_time, &self.is_forward, open);
    }

    // pin
    pub fn pin(&self) {
        self.is_pinned.set(true);
        self.set_transition(true);
    }
    pub fn unpin(&self) {
        self.is_pinned.set(false);
        self.ensure_transition_direction();
    }
    /// return the pin state
    pub fn toggle_pin(&self) -> bool {
        if self.is_pinned.get() {
            self.unpin();
            false
        } else {
            self.pin();
            true
        }
    }
    pub fn is_pinned(&self) -> bool {
        self.is_pinned.get()
    }

    // pop
    pub fn pop(&self) {
        self.invalidate_pop();

        let state = Rc::new(Cell::new(true));
        let state_clone = state.clone();
        self.pop_state.set(Some(state));
        let pop_clone = self.pop_state.clone();

        {
            let t = self.t.clone();
            let max_time = self.max_time;
            let is_forward = self.is_forward.clone();
            glib::timeout_add_local_once(self.timeout, move || {
                if state_clone.get() {
                    pop_clone.set(None);
                    // close
                    set_transition(&t, max_time, &is_forward, false);
                }
            });
        };
    }
    pub fn invalidate_pop(&self) {
        if let Some(before) = self.pop_state.take() {
            before.set(false);
        }
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

fn set_transition(
    t: &Rc<Cell<Instant>>,
    max_time: Duration,
    is_forward: &Rc<Cell<bool>>,
    open: bool,
) {
    TransitionState::<f64>::set_direction(t, max_time, is_forward, open)
}
