use std::{
    cell::RefCell,
    ops::Not,
    rc::Rc,
    time::{Duration, Instant},
};

// const DIRECTION_FORWARD: i8 = 0;
// const DIRECTION_BACKWARD: i8 = 1;
// enum Direction {
//     Forward,
//     Backward,
// }
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
}
impl TransitionState {
    pub fn new(time_cost: Duration) -> TransitionState {
        Self {
            t: Instant::now().checked_sub(time_cost).unwrap(),
            duration: time_cost,
            direction: TransitionDirection::Backward,
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
    pub fn get_y(&self) -> f64 {
        let passed_duration = self.t.elapsed();
        match self.direction {
            TransitionDirection::Forward => self.calculation(passed_duration.as_secs_f64()),
            TransitionDirection::Backward => {
                self.calculation(self.duration.as_secs_f64() - passed_duration.as_secs_f64())
            }
        }
    }
    pub fn is_in_transition(&self) -> bool {
        is_in_transition(self.get_y())
    }
    pub fn set_direction_self(&mut self, to_direction: TransitionDirection) {
        Self::set_direction(
            &mut self.t,
            self.duration,
            &mut self.direction,
            to_direction,
        )
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

    pub fn get_abs(&self, y: f64) -> f64 {
        match self.direction {
            TransitionDirection::Forward => y,
            TransitionDirection::Backward => 1. - y,
        }
    }
}

pub fn calculate_transition(y: f64, range: (f64, f64)) -> f64 {
    range.0 + (range.1 - range.0) * y
}

pub fn is_in_transition(y: f64) -> bool {
    y > 0. && y < 1.
}
