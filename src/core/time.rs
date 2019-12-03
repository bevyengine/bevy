use std::time::{Duration, Instant};

pub struct Time {
    pub delta: Duration,
    pub instant: Instant,
    pub delta_seconds: f32
}

impl Time {
    pub fn new() -> Time {
        Time {
            delta: Duration::from_secs(0),
            instant: Instant::now(),
            delta_seconds: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.instant = Instant::now();
    }

    pub fn stop(&mut self) {
        self.delta = Instant::now() - self.instant;
        self.delta_seconds = self.delta.as_secs() as f32 + (self.delta.subsec_nanos() as f32 / 1.0e9);
    }
}