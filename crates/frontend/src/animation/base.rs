use std::time::{Duration, Instant};

use config::common::Curve;

#[derive(Debug)]
pub(super) struct Animation {
    pub start_time: Instant,
    pub animation_costs: Duration,

    get_y: fn(f64) -> f64,
    get_x: fn(f64) -> f64,
    cache_y: f64,
}
impl Animation {
    pub(super) fn new(time_cost: Duration, curve: Curve) -> Self {
        fn linear(x: f64) -> f64 {
            x
        }

        fn quad_y(x: f64) -> f64 {
            x * (2.0 - x)
        }
        fn quad_x(y: f64) -> f64 {
            1.0 - (1.0 - y).sqrt()
        }

        fn cubic_y(x: f64) -> f64 {
            let x_minus_one = x - 1.0;
            1.0 + x_minus_one * x_minus_one * x_minus_one
        }
        fn cubic_x(y: f64) -> f64 {
            1.0 + (y - 1.0).cbrt()
        }

        fn expo_x(x: f64) -> f64 {
            1. - 2f64.powf(-10. * x)
        }
        fn expo_y(y: f64) -> f64 {
            -(1.0 - y).ln() / (10.0 * 2.0f64.ln())
        }

        #[allow(clippy::type_complexity)]
        let (get_y, get_x): (fn(f64) -> f64, fn(f64) -> f64) = match curve {
            Curve::Linear => (linear, linear),
            Curve::EaseQuad => (quad_y, quad_x),
            Curve::EaseCubic => (cubic_y, cubic_x),
            Curve::EaseExpo => (expo_y, expo_x),
        };

        Self {
            start_time: Instant::now(),
            animation_costs: time_cost,
            get_y,
            get_x,
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
            (self.get_y)(x)
        };
    }
    pub(super) fn flip(&mut self) {
        let passed = self.start_time.elapsed();
        if passed < self.animation_costs {
            self.start_time = Instant::now()
                .checked_sub(
                    self.animation_costs
                        .mul_f64((self.get_x)(1.0 - self.progress())),
                )
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
