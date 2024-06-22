use std::cell::Cell;
use std::ops::{Add, Mul, Sub};
use std::rc::Rc;
use std::time::{Duration, Instant};

// const DIRECTION_FORWARD: i8 = 0;
// const DIRECTION_BACKWARD: i8 = 1;
// enum Direction {
//     Forward,
//     Backward,
// }
#[derive(Clone)]
pub struct TransitionState<T>
where
    T: Add<Output = T> + Mul<Output = T> + Sub<Output = T> + From<f64> + Clone + Copy,
{
    // change
    pub t: Rc<Cell<Instant>>,
    pub is_forward: Rc<Cell<bool>>,
    // const
    pub duration: Duration,
    max_y: T,
    min_y: T,
}
impl<T> TransitionState<T>
where
    T: Add<Output = T> + Mul<Output = T> + Sub<Output = T> + From<f64> + Clone + Copy,
{
    pub fn new(time_cost: Duration, min_y: T, max_y: T) -> TransitionState<T> {
        Self {
            t: Rc::new(Cell::new(Instant::now().checked_sub(time_cost).unwrap())),
            duration: time_cost,
            is_forward: Rc::new(Cell::new(false)),
            max_y,
            min_y,
        }
    }
    fn calculation(&self, x: f64) -> T {
        let max_time = self.duration.as_secs_f64();
        if x >= max_time {
            self.max_y
        } else if x <= 0. {
            self.min_y
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
            self.min_y + (self.max_y - self.min_y) * T::from(y)
        }
    }
    pub fn get_y(&self) -> T {
        let passed_duration = self.t.get().elapsed();
        if self.is_forward.get() {
            self.calculation(passed_duration.as_secs_f64())
        } else {
            self.calculation(self.duration.as_secs_f64() - passed_duration.as_secs_f64())
        }
    }
    // pub fn set_direction(&mut self, is_forward: bool) {
    pub fn set_direction(
        t: &Rc<Cell<Instant>>,
        max_time: Duration,
        is_forward_state: &Rc<Cell<bool>>,
        is_forward: bool,
    ) {
        if is_forward_state.get() == is_forward {
            return;
        }
        // let max_time = self.duration;
        let passed_duration = t.get().elapsed();

        // NOTE: assume that `passed_duration` will not be 0.
        t.set(if passed_duration < max_time {
            Instant::now()
                .checked_sub(max_time - passed_duration)
                .unwrap()
        } else {
            Instant::now()
        });
        is_forward_state.set(is_forward);
    }
}
