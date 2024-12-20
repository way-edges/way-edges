use std::{collections::HashSet, time::Duration};

use super::{base::Curve, ToggleAnimation, ToggleAnimationRc};

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

// impl Deref for TransitionStateList {
//     type Target = Vec<Option<TransitionStateRc>>;
//     fn deref(&self) -> &Self::Target {
//         self.inner.as_ref()
//     }
// }
