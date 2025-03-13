/*
The gizmo uses batches for rendering. Call render functions from anywhere during the span of the frame and have them all render at frame update.
*/

#![allow(unused)]

use std::{collections::VecDeque, sync::atomic::AtomicU32};

struct Heavy(u32);

fn next_heavy() -> Heavy {
    static HEAVIES: std::sync::LazyLock<AtomicU32> = std::sync::LazyLock::new(|| AtomicU32::new(0));
    let next_id = HEAVIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Heavy(next_id)
}

struct Batcher<T: PartialEq> {
    batch: Vec<T>,
    
    // The number of elements present in the batch.
    count: usize,
    back_queue: VecDeque<T>,
}

impl<T: PartialEq> Batcher<T> {
    pub fn new() -> Self {
        Self {
            batch: Vec::new(),
            count: 0,
            back_queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        if let Some(item) = self.batch.get_mut(self.count) {
            if !value.ne(item) {

            }
        } else {

        }
    }
}