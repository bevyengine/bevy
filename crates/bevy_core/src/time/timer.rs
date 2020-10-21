use crate::time::Time;
use bevy_ecs::prelude::*;
use bevy_property::Properties;
use std::time::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, and can still be reset at any given point.
#[derive(Clone, Debug, Default, Properties)]
pub struct Timer {
    pub elapsed: f32,
    pub duration: f32,
    pub finished: bool,
    /// Will only be true on the tick `duration` is reached or exceeded.
    pub just_finished: bool,
    pub repeating: bool,
}

impl Timer {
    pub fn new(duration: Duration, repeating: bool) -> Self {
        Timer {
            duration: duration.as_secs_f32(),
            repeating,
            ..Default::default()
        }
    }

    pub fn from_seconds(seconds: f32, repeating: bool) -> Self {
        Timer {
            duration: seconds,
            repeating,
            ..Default::default()
        }
    }

    /// Advances the timer by `delta` seconds.
    pub fn tick(&mut self, delta: f32) {
        let prev_finished = self.elapsed >= self.duration;
        if !prev_finished {
            self.elapsed += delta;
        }

        self.finished = self.elapsed >= self.duration;
        self.just_finished = !prev_finished && self.finished;

        if self.repeating && self.finished {
            self.elapsed %= self.duration;
        }
    }

    pub fn reset(&mut self) {
        self.finished = false;
        self.just_finished = false;
        self.elapsed = 0.0;
    }
}

pub(crate) fn timer_system(time: Res<Time>, mut query: Query<&mut Timer>) {
    for mut timer in &mut query.iter() {
        timer.tick(time.delta_seconds);
    }
}
