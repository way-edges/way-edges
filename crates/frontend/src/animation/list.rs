use std::{collections::HashSet, time::Duration};

use config::shared::Curve;

use super::{ToggleAnimation, ToggleAnimationRc};

// use way_edges_derive::wrap_rc;

// #[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Debug)]
pub struct AnimationList {
    inner: HashSet<ToggleAnimationRc>,
}
impl AnimationList {
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    // this is mainly for
    pub fn has_in_progress(&self) -> bool {
        self.inner.iter().any(|f| f.borrow().is_in_progress())
    }

    pub fn new_transition(&mut self, time_cost: u64, curve: Curve) -> ToggleAnimationRc {
        let item = ToggleAnimationRc::new(ToggleAnimation::new(
            Duration::from_millis(time_cost),
            curve,
        ));
        self.inner.insert(item.clone());
        item
    }

    pub fn refresh(&mut self) {
        self.inner.iter().for_each(|f| f.borrow_mut().refresh());
    }

    pub fn extend_list(&mut self, l: &Self) {
        self.inner.extend(l.inner.iter().cloned());
    }

    pub fn remove_item(&mut self, item: &ToggleAnimationRc) {
        self.inner.remove(item);
    }
}

impl Default for AnimationList {
    fn default() -> Self {
        Self::new()
    }
}
