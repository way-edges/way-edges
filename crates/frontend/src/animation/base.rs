use std::time::{Duration, Instant};

pub enum Curve {
    Linear,
    EaseOutQuad,
    EaseOutCubic,
    EaseOutExpo,
}

#[derive(Debug)]
pub(super) struct Animation {
    pub start_time: Instant,
    pub animation_costs: Duration,

    calculate_func: fn(f64) -> f64,
    cache_y: f64,
}
impl Animation {
    pub(super) fn new(time_cost: Duration, curve: Curve) -> Self {
        fn linear(x: f64) -> f64 {
            x
        }
        fn quad(x: f64) -> f64 {
            x * (2.0 - x)
        }
        fn cubic(x: f64) -> f64 {
            let x_minus_one = x - 1.0;
            1.0 + x_minus_one * x_minus_one * x_minus_one
        }
        fn expo(x: f64) -> f64 {
            1. - 2f64.powf(-10. * x)
        }
        let calculate_func: fn(f64) -> f64 = match curve {
            Curve::Linear => linear,
            Curve::EaseOutQuad => quad,
            Curve::EaseOutCubic => cubic,
            Curve::EaseOutExpo => expo,
        };

        Self {
            start_time: Instant::now(),
            animation_costs: time_cost,
            calculate_func,
            cache_y: 0.,
        }
    }
    pub(super) fn refresh(&mut self) {
        let max_time = self.animation_costs.as_secs_f64();
        let x = self.start_time.elapsed().as_secs_f64() / max_time;
        self.cache_y = if x >= 1. {
            1.
        } else if x <= 0. {
            0.
        } else {
            (self.calculate_func)(x)
        };
    }
    pub(super) fn reset(&mut self) {
        self.start_time = Instant::now();
        self.cache_y = 0.;
    }
    pub(super) fn flip(&mut self) {
        let passed = self.start_time.elapsed();
        if passed < self.animation_costs {
            self.start_time = Instant::now()
                .checked_sub(self.animation_costs - passed)
                .unwrap();
        } else {
            self.start_time = Instant::now();
        }
        self.refresh();
    }
    pub(super) fn progress(&self) -> f64 {
        self.cache_y
    }
}
