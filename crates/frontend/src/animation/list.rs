use std::{collections::HashSet, time::Duration};

use super::{base::Curve, ToggleAnimation, ToggleAnimationRc};

use way_edges_derive::wrap_rc;

// wrap_rc!(pub AnimationListRc, pub AnimationList);

#[wrap_rc(rc = "pub", normal = "pub")]
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

    // TODO: HHH
    // pub fn new_transition(&mut self, time_cost: u64, curve: Curve) -> ToggleAnimationRc {
    pub fn new_transition(&mut self, time_cost: u64) -> ToggleAnimationRc {
        let item = ToggleAnimationRc::new(ToggleAnimation::new(
            Duration::from_millis(time_cost),
            Curve::Linear,
        ));
        self.inner.insert(item.clone());
        item
    }

    pub fn refresh_and_has_in_progress(&mut self) -> bool {
        let mut has = false;
        self.inner.iter().for_each(|f| {
            let mut f = f.borrow_mut();
            f.refresh();
            if !has && f.is_in_progress() {
                has = true
            }
        });
        has
    }

    pub fn has_in_progress(&self) -> bool {
        let mut has = false;
        self.inner.iter().for_each(|f| {
            if !has && f.borrow().is_in_progress() {
                has = true
            };
        });
        has
    }

    pub fn refresh(&mut self) {
        self.inner.iter().for_each(|f| {
            f.borrow_mut().refresh();
        });
    }

    pub fn extend_list(&mut self, l: &Self) {
        self.inner.extend(l.inner.iter().cloned());
    }

    pub fn remove_item(&mut self, item: &ToggleAnimationRc) {
        self.inner.remove(item);
    }
}
