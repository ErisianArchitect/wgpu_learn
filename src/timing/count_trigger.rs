
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CountTrigger {
    count: u64,
}

impl CountTrigger {
    pub const ZERO: Self = Self::new();
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    #[inline(always)]
    pub const fn frame_count(self) -> u64 {
        self.count
    }

    #[inline(always)]
    pub fn set_count(&mut self, count: u64) {
        self.count = count;
    }

    #[inline(always)]
    pub fn increment(&mut self) -> u64 {
        self.count += 1;
        self.count
    }

    #[inline(always)]
    pub fn add(&mut self, add: u64) -> u64 {
        self.count += add;
        self.count
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.count = 0;
    }

    #[inline(always)]
    pub fn every_nth<R, F: FnOnce(u64) -> R>(self, nth: u64, f: F) -> Option<R> {
        if self.count % nth == 0 {
            Some(f(self.count))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn ge<R, F: FnOnce(u64) -> R>(self, lhs: u64, f: F) -> Option<R> {
        if self.count >= lhs {
            Some(f(self.count))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn lt<R, F: FnOnce(u64) -> R>(self, lhs: u64, f: F) -> Option<R> {
        if self.count < lhs {
            Some(f(self.count))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn nth<R, F: FnOnce(u64) -> R>(self, nth: u64, f: F) -> Option<R> {
        if self.count == nth {
            Some(f(self.count))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn fct_nth_test() {
        let mut counter = CountTrigger::new();
        for _ in 0..100 {
            counter.nth(10, |i| {
                println!("nth(10): {i}");
            });
            counter.every_nth(3, |i| {
                println!("every_nth(17): {i}");
            });
            counter.ge(90, |i| {
                println!(">= 90: {i}");
            });
            counter.increment();
        }
    }
}