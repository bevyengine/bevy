use legion::prelude::*;
use std::time::{Duration, Instant};

pub struct Time {
    pub delta: Duration,
    pub instant: Option<Instant>,
    pub delta_seconds_f64: f64,
    pub delta_seconds: f32,
}

impl Default for Time {
    fn default() -> Time {
        Time {
            delta: Duration::from_secs(0),
            instant: None,
            delta_seconds_f64: 0.0,
            delta_seconds: 0.0,
        }
    }
}

impl Time {
    pub fn update(&mut self) {
        let now = Instant::now();
        if let Some(instant) = self.instant {
            self.delta = now - instant;
            self.delta_seconds_f64 =
                self.delta.as_secs() as f64 + (self.delta.subsec_nanos() as f64 / 1.0e9);
            self.delta_seconds = self.delta_seconds_f64 as f32;
        }
        self.instant = Some(now);
    }
}

pub fn timer_system(mut time: ResMut<Time>) {
    time.update();
}