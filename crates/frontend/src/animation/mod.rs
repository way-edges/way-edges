mod base;
mod list;

use std::{hash::Hash, ops::Not, rc::Rc, time::Duration};

use config::shared::Curve;
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
