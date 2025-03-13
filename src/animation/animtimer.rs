use std::time::{Duration, Instant};


pub struct AnimTimer {
    start: Instant,
    duration: Duration,
}

impl AnimTimer {
    pub fn start(duration: Duration) -> Self {
        Self {
            start: Instant::now(),
            duration
        }
    }

    pub fn alpha_f32(&self) -> f32 {
        let elapsed = self.start.elapsed();
        if elapsed >= self.duration {
            1.0
        } else {
            let dur_time = self.duration.as_secs_f32();
            let elap_time = elapsed.as_secs_f32();
            1.0 - (dur_time - elap_time) / dur_time
        }
    }

    pub fn alpha_f64(&self) -> f64 {
        let elapsed = self.start.elapsed();
        if elapsed >= self.duration {
            1.0
        } else {
            let dur_time = self.duration.as_secs_f64();
            let elap_time = elapsed.as_secs_f64();
            1.0 - (dur_time - elap_time) / dur_time
        }
    }

    /// Get the `alpha` then reset the timer.
    pub fn get_reset_f32(&mut self) -> f32 {
        let alpha = self.alpha_f32();
        self.start = Instant::now();
        alpha
    }

    /// Get the `alpha` then reset the timer.
    pub fn get_reset_f64(&mut self) -> f64 {
        let alpha = self.alpha_f64();
        self.start = Instant::now();
        alpha
    }

    /// Reset the timer.
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    pub fn is_finished(&self) -> bool {
        self.start.elapsed() >= self.duration
    }
}

#[cfg(test)]
mod testing_sandbox {
    // TODO: Remove this sandbox when it is no longer in use.
    use super::*;
    #[test]
    fn sandbox() {
        let anim = AnimTimer::start(Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(1000));
        println!("{}", anim.alpha_f32());
    }
}