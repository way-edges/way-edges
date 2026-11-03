mod base;
mod list;

use std::{hash::Hash, ops::Not, rc::Rc, time::Duration};

use config::def::shared::Curve;
use way_edges_derive::wrap_rc;

use base::Animation;
pub use list::AnimationList;
// pub use list::{AnimationList, AnimationListRc};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ToggleDirection {
    Forward,
    Backward,
}
impl Not for ToggleDirection {
    type Output = ToggleDirection;

    fn not(self) -> Self::Output {
        match self {
            ToggleDirection::Forward => Self::Backward,
            ToggleDirection::Backward => Self::Forward,
        }
    }
}
impl From<bool> for ToggleDirection {
    fn from(x: bool) -> Self {
        if x {
            Self::Forward
        } else {
            Self::Backward
        }
    }
}

#[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Debug)]
pub struct ToggleAnimation {
    pub direction: ToggleDirection,
    base_animation: Animation,
}
impl ToggleAnimation {
    pub fn new(time_cost: Duration, curve: Curve) -> ToggleAnimation {
        Self {
            direction: ToggleDirection::Backward,
            base_animation: Animation::new(time_cost, curve),
        }
    }
    /// Starts a full forward transition from its initial value.
    pub fn restart(&mut self) {
        self.direction = ToggleDirection::Forward;
        self.base_animation.start_time = std::time::Instant::now();
        self.base_animation.refresh();
    }
    pub fn refresh(&mut self) {
        self.base_animation.refresh();
    }
    pub fn progress(&self) -> f64 {
        let p = self.base_animation.progress();
        match self.direction {
            ToggleDirection::Forward => p,
            ToggleDirection::Backward => 1. - p,
        }
    }
    pub fn set_direction(&mut self, to_direction: ToggleDirection) {
        if self.direction == to_direction {
            return;
        }
        self.base_animation.flip();
        self.direction = to_direction;
    }
    pub fn flip(&mut self) {
        self.set_direction(self.direction.not());
    }
    pub fn progress_abs(&self) -> f64 {
        self.base_animation.progress()
    }
    pub fn is_in_progress(&self) -> bool {
        let p = self.progress();
        p > 0. && p < 1.
    }
}

impl Hash for ToggleAnimationRc {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&*self.0, state)
    }
}
impl Eq for ToggleAnimationRc {}
impl PartialEq for ToggleAnimationRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub fn calculate_transition(y: f64, range: (f64, f64)) -> f64 {
    range.0 + (range.1 - range.0) * y
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn restarted_transition_requests_frames_until_completion() {
        let mut list = AnimationList::new();
        let animation = list.new_transition(260, Curve::EaseCubic);
        {
            let mut animation = animation.borrow_mut();
            animation.restart();
            animation.base_animation.start_time = Instant::now() - Duration::from_millis(130);
        }
        list.refresh();
        assert!(list.has_in_progress());
        assert!(animation.borrow().progress_abs() > 0.5);
        animation.borrow_mut().base_animation.start_time =
            Instant::now() - Duration::from_millis(300);
        list.refresh();
        assert!(!list.has_in_progress());
        assert_eq!(animation.borrow().progress_abs(), 1.0);
        animation.borrow_mut().restart();
        assert!(animation.borrow().progress_abs() < 0.1);
    }

    #[test]
    fn zero_duration_transition_completes_immediately() {
        let mut list = AnimationList::new();
        let animation = list.new_transition(0, Curve::EaseCubic);
        animation.borrow_mut().restart();
        list.refresh();
        assert_eq!(animation.borrow().progress_abs(), 1.0);
        assert!(!list.has_in_progress());
    }
}
