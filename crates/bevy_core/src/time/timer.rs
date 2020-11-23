use crate::time::Time;
use bevy_ecs::prelude::*;
use bevy_property::Properties;
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
#[derive(Clone, Debug, Default, Properties)]
pub struct Timer {
    elapsed: f32,
    duration: f32,
    finished: bool,
    /// Will only be true on the tick `duration` is reached or exceeded.
    just_finished: bool,
    paused: bool,
    repeating: bool,
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

    pub fn pause(&mut self) {
        self.paused = true
    }

    pub fn resume(&mut self) {
        self.paused = false
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn just_finished(&self) -> bool {
        self.just_finished
    }

    pub fn is_repeating(&self) -> bool {
        self.repeating
    }

    /// Advances the timer by `delta` seconds.
    pub fn tick(&mut self, delta: f32) -> &Self {
        if self.paused {
            return self;
        }

        let prev_finished = self.elapsed >= self.duration;
        if !prev_finished {
            self.elapsed += delta;
        }

        self.finished = self.elapsed >= self.duration;
        self.just_finished = !prev_finished && self.finished;

        if self.repeating && self.finished {
            self.elapsed %= self.duration;
        }
        self
    }

    pub fn reset(&mut self) {
        self.finished = false;
        self.just_finished = false;
        self.elapsed = 0.0;
    }
}

pub(crate) fn timer_system(time: Res<Time>, mut query: Query<&mut Timer>) {
    for mut timer in query.iter_mut() {
        timer.tick(time.delta_seconds);
    }
}
