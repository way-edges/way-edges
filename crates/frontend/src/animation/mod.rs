mod base;
mod list;
pub use list::{AnimationList, AnimationListRc};

use std::{
    cell::RefCell,
    hash::Hash,
    ops::{Deref, Not},
    rc::Rc,
    time::Duration,
};

use base::{Animation, Curve};

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
        self.base_animation.progress()
    }
    pub fn set_direction(&mut self, to_direction: ToggleDirection) {
        if self.direction == to_direction {
            return;
        }
        self.base_animation.flip();
        self.direction = to_direction;
    }
    pub fn progress_abs(&self) -> f64 {
        let p = self.progress();
        match self.direction {
            ToggleDirection::Forward => p,
            ToggleDirection::Backward => 1. - p,
        }
    }
    pub fn is_in_transition(&self) -> bool {
        let p = self.progress();
        p > 0. && p < 1.
    }
}

pub fn calculate_transition(y: f64, range: (f64, f64)) -> f64 {
    range.0 + (range.1 - range.0) * y
}

#[derive(Debug, Clone)]
pub struct ToggleAnimationRc(Rc<RefCell<ToggleAnimation>>);
impl ToggleAnimationRc {
    fn new(ani: ToggleAnimation) -> Self {
        Self(Rc::new(RefCell::new(ani)))
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
impl Deref for ToggleAnimationRc {
    type Target = RefCell<ToggleAnimation>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
