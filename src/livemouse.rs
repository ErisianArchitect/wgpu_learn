use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LiveMouse {
    pub velocity: (f64, f64),
    pub target_velocity: (f64, f64),
    pub max_velocity: f64,
    pub acceleration_factor: f64,
    pub deceleration_factor: f64,
    pub halting: bool,
}

impl LiveMouse {
    pub fn new(
        acceleration_factor: f64,
        deceleration_factor: f64,
        max_velocity: f64,
        halting: bool,
    ) -> Self {
        Self {
            velocity: (0.0, 0.0),
            target_velocity: (0.0, 0.0),
            max_velocity,
            acceleration_factor,
            deceleration_factor,
            halting,
        }
    }

    pub fn set_target(&mut self, delta_x: f64, delta_y: f64) {
        let mag = (delta_x * delta_x + delta_y * delta_y).sqrt();
        if mag > 0.0001 {
            let scale = mag.min(self.max_velocity / mag);
            self.target_velocity = (delta_x * scale, delta_y * scale);
        } else {
            if self.halting {
                self.reset();
            } else {
                self.target_velocity = (0.0, 0.0);
            }
        }
    }

    pub fn update(&mut self, dt: Duration) -> (f64, f64) {
        let dt_seconds = dt.as_secs_f64();

        let dx = self.target_velocity.0 - self.velocity.0;
        let dy = self.target_velocity.1 - self.velocity.1;

        let factor = if self.is_accelerating() {
            self.acceleration_factor
        } else {
            self.deceleration_factor
        };

        let smooth_factor = 1.0 - (-factor * dt_seconds).exp();

        self.velocity.0 += dx * smooth_factor;
        self.velocity.1 += dy * smooth_factor;

        self.velocity
    }

    pub fn velocity(&self) -> (f64, f64) {
        self.velocity
    }

    pub fn is_accelerating(&self) -> bool {
        let current_mag_sq = self.velocity.0 * self.velocity.0 + self.velocity.1 * self.velocity.1;
        let target_mag_sq = self.target_velocity.0 * self.target_velocity.0 + self.target_velocity.1 * self.target_velocity.1;

        target_mag_sq > current_mag_sq
    }

    pub fn reset(&mut self) {
        self.velocity = (0.0, 0.0);
        self.target_velocity = (0.0, 0.0);
    }
}

pub struct ExpMouse {
    pub delta: (f64, f64),
    pub delta_accum: (f64, f64),
    pub smoothing_factor: f64,
    pub halting: bool,
}

// impl ExpMouse {
//     pub fn new(
//         smoothing_factor: f64,
//         halting: bool,
//     ) -> Self {
//         Self {
//             delta: (0.0, 0.0),
//             delta_accum: (0.0, 0.0),
//             smoothing_factor,
//             halting,
//         }
//     }

//     pub fn accumulate_delta(&mut self, delta: (f64, f64)) {
//         self.delta_accum.0 += delta.0;
//         self.delta_accum.1 += delta.1;
//     }

//     pub fn update(&mut self, dt: Duration) -> (f64, f64) {
//         let secs = dt.as_secs_f64();
        
//     }
// }