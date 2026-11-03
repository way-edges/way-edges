use std::{
    rc::{Rc, Weak},
    time::Duration,
};

use crate::animation::{ToggleAnimationRc, ToggleDirection};

#[derive(Debug)]
pub struct WindowPopState {
    pinnale: bool,
    pin_with_key: bool,
    pin_key: u32,
    pub pin_state: bool,
    pub hide_delay: Duration,
    pub pop_state: Option<Rc<()>>,
    pub pop_animation: ToggleAnimationRc,
}
impl WindowPopState {
    pub fn new(ani: ToggleAnimationRc, pinnale: bool, pin_with_key: bool, pin_key: u32) -> Self {
        Self {
            pin_state: false,
            hide_delay: Duration::ZERO,
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
    /// Tracks a pending timeout until hover, pin state, or another timeout invalidates it.
    pub fn start_pop_timeout(&mut self) -> Weak<()> {
        let guard = Rc::new(());
        let weak = Rc::downgrade(&guard);
        self.pop_state = Some(guard);
        weak
    }

    pub fn defer_leave(&mut self) -> Option<(Duration, Weak<()>)> {
        self.invalidate_pop();
        if self.pin_state {
            return None;
        }
        if self.hide_delay.is_zero() {
            self.leave();
            return None;
        }
        Some((self.hide_delay, self.start_pop_timeout()))
    }

    pub fn complete_pop_timeout(&mut self, guard: &Weak<()>, hovering: bool) -> bool {
        let is_current = self
            .pop_state
            .as_ref()
            .is_some_and(|active| guard.ptr_eq(&Rc::downgrade(active)));
        if !is_current {
            return false;
        }
        self.invalidate_pop();
        if hovering || self.pin_state {
            return false;
        }
        self.leave();
        true
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::ToggleAnimation;
    use config::def::shared::Curve;

    fn shown_state(delay: u64) -> WindowPopState {
        let animation =
            ToggleAnimation::new(Duration::from_millis(300), Curve::EaseCubic).make_rc();
        let mut state = WindowPopState::new(animation, true, false, 274);
        state.hide_delay = Duration::from_millis(delay);
        state.enter();
        state
    }

    #[test]
    fn pointer_leave_keeps_the_dock_shown_until_its_timeout() {
        let mut state = shown_state(1500);
        let (duration, guard) = state.defer_leave().unwrap();
        assert_eq!(duration, Duration::from_millis(1500));
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Forward
        );
        assert!(state.complete_pop_timeout(&guard, false));
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Backward
        );
        assert!(guard.upgrade().is_none());
    }

    #[test]
    fn pointer_return_cancels_the_pending_hide_and_next_leave_restarts_it() {
        let mut state = shown_state(1500);
        let (_, first) = state.defer_leave().unwrap();
        // A live external reference must not let a stale timer affect the next hover.
        let _first_guard = first.upgrade().unwrap();
        state.enter();
        assert!(!state.complete_pop_timeout(&first, false));
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Forward
        );
        let (duration, second) = state.defer_leave().unwrap();
        assert_eq!(duration, Duration::from_millis(1500));
        assert!(!state.complete_pop_timeout(&first, false));
        assert!(state.complete_pop_timeout(&second, false));
    }

    #[test]
    fn hovered_or_pinned_widgets_do_not_hide_when_a_timer_expires() {
        let mut state = shown_state(1500);
        let (_, guard) = state.defer_leave().unwrap();
        assert!(!state.complete_pop_timeout(&guard, true));
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Forward
        );
        let (_, guard) = state.defer_leave().unwrap();
        state.toggle_pin(false);
        assert!(!state.complete_pop_timeout(&guard, false));
        assert!(state.pin_state);
        assert!(state.defer_leave().is_none());
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Forward
        );
    }

    #[test]
    fn zero_delay_hides_immediately_without_scheduling_a_timeout() {
        let mut state = shown_state(0);
        assert!(state.defer_leave().is_none());
        assert!(state.pop_state.is_none());
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Backward
        );
    }

    #[test]
    fn event_triggered_pop_uses_the_same_timeout_completion() {
        let mut state = shown_state(0);
        let guard = state.start_pop_timeout();
        assert!(state.complete_pop_timeout(&guard, false));
        assert_eq!(
            state.pop_animation.borrow().direction,
            ToggleDirection::Backward
        );
    }
}
