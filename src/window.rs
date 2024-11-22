use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct EventWindow {
    events: VecDeque<Instant>,
}

impl EventWindow {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn push(&mut self, window: Duration) {
        let now = Instant::now();
        self.events.push_back(now);
        self.remove_old(window, now);
    }

    pub fn exceeds_threshold(&self, threshold: usize) -> bool {
        self.events.len() > threshold
    }

    fn remove_old(&mut self, window: Duration, now: Instant) {
        while let Some(&oldest) = self.events.front() {
            if now.duration_since(oldest) > window {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }
}
