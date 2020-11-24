use crate::time::Time;
use bevy_ecs::prelude::*;
use bevy_property::Properties;
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, and can still be reset at any given point.
#[derive(Clone, Debug, Default, Properties)]
pub struct Timer {
    /// Time elapsed on the timer. Guaranteed to be between 0.0 and `duration`, inclusive.
    pub elapsed: f32,
    pub duration: f32,
    /// Non repeating timers will stop tracking and stay in the finished state until reset.
    /// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, and can still be reset at any given point.
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
    pub fn tick(&mut self, delta: f32) -> &Self {
        let prev_finished = self.finished;
        self.elapsed += delta;

        self.finished = self.elapsed >= self.duration;
        self.just_finished = !prev_finished && self.finished;
        if self.finished {
            if self.repeating {
                // Repeating timers wrap around
                self.elapsed %= self.duration;
            } else {
                // Non-repeating timers clamp to duration
                self.elapsed = self.duration;
            }
        }
        self
    }

    pub fn reset(&mut self) {
        self.finished = false;
        self.just_finished = false;
        self.elapsed = 0.0;
    }

    /// Percent timer has elapsed (goes from 0.0 to 1.0)
    pub fn percent(&self) -> f32 {
        self.elapsed / self.duration
    }

    /// Percent left on timer (goes from 1.0 to 0.0)
    pub fn percent_left(&self) -> f32 {
        (self.duration - self.elapsed) / self.duration
    }
}

pub(crate) fn timer_system(time: Res<Time>, mut query: Query<&mut Timer>) {
    for mut timer in query.iter_mut() {
        timer.tick(time.delta_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::Timer;

    #[test]
    fn test_non_repeating() {
        let mut t = Timer::from_seconds(10.0, false);
        // Tick once, check all attributes
        t.tick(0.25);
        assert_eq!(t.elapsed, 0.25);
        assert_eq!(t.duration, 10.0);
        assert_eq!(t.finished, false);
        assert_eq!(t.just_finished, false);
        assert_eq!(t.repeating, false);
        assert_eq!(t.percent(), 0.025);
        assert_eq!(t.percent_left(), 0.975);
        // Tick past the end and make sure elapsed doesn't go past 0.0 and other things update
        t.tick(500.0);
        assert_eq!(t.elapsed, 10.0);
        assert_eq!(t.finished, true);
        assert_eq!(t.just_finished, true);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
        // Continuing to tick when finished should only change just_finished
        t.tick(1.0);
        assert_eq!(t.elapsed, 10.0);
        assert_eq!(t.finished, true);
        assert_eq!(t.just_finished, false);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
    }

    #[test]
    fn test_repeating() {
        let mut t = Timer::from_seconds(2.0, true);
        // Tick once, check all attributes
        t.tick(0.75);
        assert_eq!(t.elapsed, 0.75);
        assert_eq!(t.duration, 2.0);
        assert_eq!(t.finished, false);
        assert_eq!(t.just_finished, false);
        assert_eq!(t.repeating, true);
        assert_eq!(t.percent(), 0.375);
        assert_eq!(t.percent_left(), 0.625);
        // Tick past the end and make sure elapsed wraps
        t.tick(1.5);
        assert_eq!(t.elapsed, 0.25);
        assert_eq!(t.finished, true);
        assert_eq!(t.just_finished, true);
        assert_eq!(t.percent(), 0.125);
        assert_eq!(t.percent_left(), 0.875);
        // Continuing to tick should turn off both finished & just_finished for repeating timers
        t.tick(1.0);
        assert_eq!(t.elapsed, 1.25);
        assert_eq!(t.finished, false);
        assert_eq!(t.just_finished, false);
        assert_eq!(t.percent(), 0.625);
        assert_eq!(t.percent_left(), 0.375);
    }
}
