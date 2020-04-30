use legion::prelude::*;
use std::time::{Duration, Instant};

pub struct Time {
    pub delta: Duration,
    pub instant: Instant,
    pub delta_seconds_f64: f64,
    pub delta_seconds: f32,
}

impl Time {
    pub fn new() -> Time {
        Time {
            delta: Duration::from_secs(0),
            instant: Instant::now(),
            delta_seconds_f64: 0.0,
            delta_seconds: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.instant = Instant::now();
    }

    pub fn stop(&mut self) {
        self.delta = Instant::now() - self.instant;
        self.delta_seconds_f64 =
            self.delta.as_secs() as f64 + (self.delta.subsec_nanos() as f64 / 1.0e9);
        self.delta_seconds = self.delta_seconds_f64 as f32;
    }
}

pub fn start_timer_system(mut time: ResourceMut<Time>) {
    time.start();
}

pub fn stop_timer_system(mut time: ResourceMut<Time>) {
    time.stop();
}