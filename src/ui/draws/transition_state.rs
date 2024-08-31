use std::{
    cell::RefCell,
    ops::{Deref, Not},
    rc::Rc,
    time::{Duration, Instant},
};

pub type TransitionStateRc = Rc<RefCell<TransitionState>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TransitionDirection {
    Forward,
    Backward,
}
impl Not for TransitionDirection {
    type Output = TransitionDirection;

    fn not(self) -> Self::Output {
        match self {
            TransitionDirection::Forward => Self::Backward,
            TransitionDirection::Backward => Self::Forward,
        }
    }
}
impl From<bool> for TransitionDirection {
    fn from(x: bool) -> Self {
        if x {
            Self::Forward
        } else {
            Self::Backward
        }
    }
}

#[derive(Debug)]
pub struct TransitionState {
    // change
    pub t: Instant,
    pub direction: TransitionDirection,
    // const + Copy
    pub duration: Duration,

    cache_y: f64,
}
impl TransitionState {
    pub fn new(time_cost: Duration) -> TransitionState {
        Self {
            t: Instant::now().checked_sub(time_cost).unwrap(),
            duration: time_cost,
            direction: TransitionDirection::Backward,

            cache_y: 0.,
        }
    }
    fn calculation(&self, x: f64) -> f64 {
        let max_time = self.duration.as_secs_f64();
        if x >= max_time {
            1.
        } else if x <= 0. {
            0.
        } else {
            // real calculation, for now it's simply: y=x
            // for example:
            // :: move 40px(from -10px to 30px) in 300ms(0.3s)
            // -> x = 0.15s
            // -- normalize x: 0.15/0.3 = 0.5, so the input will always be in [0, 1]
            // <- normalized_y: y = x
            // :: get px(y) given normalized_y
            // -> normalized_y = 0.5
            // <- y: y = -10 + (30 - -10) * normalized_y
            let x = x / max_time;
            let y = x;
            y
        }
    }

    pub fn refresh(&mut self) {
        let passed_duration = self.t.elapsed();
        let y = match self.direction {
            TransitionDirection::Forward => self.calculation(passed_duration.as_secs_f64()),
            TransitionDirection::Backward => {
                self.calculation(self.duration.as_secs_f64() - passed_duration.as_secs_f64())
            }
        };
        self.cache_y = y
    }
    pub fn get_y(&self) -> f64 {
        self.cache_y
    }
    pub fn get_abs_y(&self) -> f64 {
        match self.direction {
            TransitionDirection::Forward => self.cache_y,
            TransitionDirection::Backward => 1. - self.cache_y,
        }
    }

    pub fn is_in_transition(&self) -> bool {
        is_in_transition(self.get_y())
    }
    pub fn set_direction_self(&mut self, to_direction: TransitionDirection) {
        if self.direction == to_direction {
            return;
        }
        // let max_time = self.duration;
        let passed_duration = self.t.elapsed();

        // NOTE: assume that `passed_duration` will not be 0.
        self.t = if passed_duration < self.duration {
            Instant::now()
                .checked_sub(self.duration - passed_duration)
                .unwrap()
        } else {
            Instant::now()
        };
        self.direction = to_direction;

        // Self::set_direction(
        //     &mut self.t,
        //     self.duration,
        //     &mut self.direction,
        //     to_direction,
        // )
    }
    pub fn set_direction(
        t: &mut Instant,
        max_time: Duration,
        direction_state: &mut TransitionDirection,
        to_direction: TransitionDirection,
    ) {
        if *direction_state == to_direction {
            return;
        }
        // let max_time = self.duration;
        let passed_duration = t.elapsed();

        // NOTE: assume that `passed_duration` will not be 0.
        *t = if passed_duration < max_time {
            Instant::now()
                .checked_sub(max_time - passed_duration)
                .unwrap()
        } else {
            Instant::now()
        };
        *direction_state = to_direction;
    }
}

pub fn calculate_transition(y: f64, range: (f64, f64)) -> f64 {
    range.0 + (range.1 - range.0) * y
}

pub fn is_in_transition(y: f64) -> bool {
    y > 0. && y < 1.
}

// pub struct TransitionStateList(Vec<TransitionStateRc>);
// impl TransitionStateList {
//     pub fn new() -> Self {
//         Self(vec![])
//     }
//
//     pub fn new_transition(&mut self, duration: Duration) -> TransitionStateRc {
//         let ts = TransitionState::new(duration);
//         let item = Rc::new(RefCell::new(ts));
//         self.0.push(item.clone());
//         item
//     }
//
//     pub fn refresh(&mut self) {
//         self.0.iter_mut().for_each(|f| {
//             f.borrow_mut().refresh();
//         });
//     }
//
//     pub fn extend_list(&mut self, l: &[TransitionStateRc]) {
//         self.0.extend_from_slice(l);
//     }
//
//     pub fn remove_item(&mut self, id: usize) {
//         self.0.remove()
//     }
// }
// impl Deref for TransitionStateList {
//     type Target = Vec<TransitionStateRc>;
//     fn deref(&self) -> &Self::Target {
//         self.0.as_ref()
//     }
// }
// impl DerefMut for TransitionStateList {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.0.as_mut()
//     }
// }
//

pub struct TransitionStateItem {
    pub index: usize,
    pub item: TransitionStateRc,
}

pub struct TransitionStateList {
    inner: Vec<Option<TransitionStateRc>>,
    empties: Vec<usize>,
}
impl TransitionStateList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            empties: Vec::new(),
        }
    }

    pub fn new_transition(&mut self, duration: Duration) -> TransitionStateItem {
        let ts = TransitionState::new(duration);
        let item = Rc::new(RefCell::new(ts));

        let index = if !self.empties.is_empty() {
            let index = self.empties.pop().unwrap();
            self.inner[index] = Some(item.clone());
            index
        } else {
            self.inner.push(Some(item.clone()));
            self.inner.len() - 1
        };

        TransitionStateItem { index, item }
    }

    pub fn refresh(&mut self) {
        self.inner.iter_mut().for_each(|f| {
            if let Some(f) = f.as_ref() {
                f.borrow_mut().refresh();
            }
        });
    }

    pub fn extend_list(&mut self, l: &[TransitionStateRc]) {
        let a = l
            .iter()
            .map(|i| Some(i.clone()))
            .collect::<Vec<Option<TransitionStateRc>>>();
        self.inner.extend_from_slice(&a);
    }

    pub fn remove_item(&mut self, id: usize) {
        if let Some(item) = self.inner.get_mut(id) {
            *item = None
        }
        self.empties.push(id)
    }
}

impl Deref for TransitionStateList {
    type Target = Vec<Option<TransitionStateRc>>;
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}
