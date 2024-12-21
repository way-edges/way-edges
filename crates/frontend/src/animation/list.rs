use std::{cell::RefCell, collections::HashSet, ops::Deref, rc::Rc, time::Duration};

use super::{base::Curve, ToggleAnimation, ToggleAnimationRc};

#[derive(Debug, Clone)]
pub struct AnimationListRc(Rc<RefCell<AnimationList>>);
impl AnimationListRc {
    fn new(ani: AnimationList) -> Self {
        Self(Rc::new(RefCell::new(ani)))
    }
}
impl Deref for AnimationListRc {
    type Target = RefCell<AnimationList>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    pub fn make_rc(self) -> AnimationListRc {
        AnimationListRc::new(self)
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
