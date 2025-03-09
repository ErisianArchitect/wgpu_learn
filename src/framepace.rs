use std::{collections::VecDeque, time::{Duration, Instant}};


/*
- Framepace needs to measure update time, render time, and frame time.
- It needs to know the refresh rate.
- It needs to handle whether or not the present mode is VSync.
- If the present mode is VSync, it needs to calculate the right time to begin update
| so that update ends just before render needs to begin, and measured so that render ends
| just before the monitor refreshes.
| That means that the beginning of update time is `frame_time - (avg_render_time + avg_update_time)`
| and the beginning of render time is `frame_time - avg_render_time`.
*/
pub struct Framepace {
    update_average: AverageBuffer,
    render_average: AverageBuffer,
    frame_rate_secs: f64,
    frame_time: Option<Instant>,
}

impl Framepace {
    pub fn new(average_capacity: usize, frame_time: f64) -> Self {
        Self {
            update_average: AverageBuffer::new(average_capacity),
            render_average: AverageBuffer::new(average_capacity),
            frame_rate_secs: frame_time,
            frame_time: None,
        }
    }

    fn measure_time<R, F: FnOnce() -> R>(f: F) -> (R, Duration) {
        let start_time = Instant::now();
        let result = f();
        let elapsed = start_time.elapsed();
        (result, elapsed)
    }

    pub fn measure_update<R, F: FnOnce() -> R>(&mut self, update: F) -> R {
        let (result, time) = Self::measure_time(update);
        self.update_average.push(time.as_secs_f64());
        result
    }

    pub fn measure_render<R, F: FnOnce() -> R>(&mut self, render: F) -> R {
        let (result, time) = Self::measure_time(render);
        self.render_average.push(time.as_secs_f64());
        result
    }

    // pub fn is_time_to_update(&self) -> bool {
    //     let Some(ref frame_time) = self.frame_time else {
    //         return false;
    //     };
    //     let update_avg = self.update_average.average();
    //     let render_avg = self.render_average.average();

    // }

    pub fn end_frame(&mut self) {
        self.frame_time = Some(Instant::now())
    }
}

#[derive(Debug, Clone)]
pub struct AverageBuffer {
    pub buffer: VecDeque<f64>,
}

impl AverageBuffer {
    pub fn new(capacity: usize) -> Self {
        assert_ne!(capacity, 0, "Capacity must be greater than 0.");
        Self {
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    pub fn with_seed(capacity: usize, seed: f64) -> Self {
        let mut new = Self::new(capacity);
        new.buffer.push_back(seed);
        new
    }

    pub fn average(&self) -> f64 {
        self.buffer.iter().cloned().sum::<f64>() / self.buffer.len() as f64
    }

    pub fn push(&mut self, t: f64) {
        if self.buffer.len() == self.buffer.capacity() {
            self.buffer.pop_front();
        }
        self.buffer.push_back(t);
    }

    /// Pushes value and then gets the resulting average.
    pub fn push_get(&mut self, t: f64) -> f64 {
        self.push(t);
        self.average()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn reset(&mut self, new_seed: f64) {
        self.buffer.clear();
        self.push(new_seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn avg_test() {
        let mut avgs = AverageBuffer::with_seed(10, 5.0);
        avgs.push(10.0);
        avgs.push(15.0);
        println!("{}", avgs.average());
        avgs.reset(50.0);
        println!("{}", avgs.average());
    }
}