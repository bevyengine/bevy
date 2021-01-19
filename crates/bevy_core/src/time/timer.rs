use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Timer {
    elapsed: f32,
    duration: f32,
    finished: bool,
    /// Will only be non-zero on the tick `duration` is reached or exceeded.
    just_finished_count: u32,
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

    #[inline]
    pub fn pause(&mut self) {
        self.paused = true
    }

    #[inline]
    pub fn unpause(&mut self) {
        self.paused = false
    }

    #[inline]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Returns the time elapsed on the timer. Guaranteed to be between 0.0 and `duration`.
    /// Will only equal `duration` when the timer is finished and non repeating.
    #[inline]
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    #[inline]
    pub fn set_elapsed(&mut self, elapsed: f32) {
        self.elapsed = elapsed
    }

    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    pub fn set_duration(&mut self, duration: f32) {
        self.duration = duration
    }

    /// Returns the finished state of the timer.
    ///
    /// Non-repeating timers will stop tracking and stay in the finished state until reset.
    /// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, so in that case
    /// this function is equivalent to `just_finished`.
    #[inline]
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Will only be true on the tick the timer's duration is reached or exceeded.
    #[inline]
    pub fn just_finished(&self) -> bool {
        self.just_finished_count > 0
    }

    /// Returns the total number of times the timer finished during this tick.
    ///
    /// This value can be used to ensure no completions of a repeating timer are skipped over due to a tick with an unexpectedly
    /// long delta time. For non repeating timers, the value will only ever be 0 or 1.
    #[inline]
    pub fn just_finished_count(&self) -> u32 {
        self.just_finished_count
    }

    #[inline]
    pub fn repeating(&self) -> bool {
        self.repeating
    }

    pub fn set_repeating(&mut self, repeating: bool) {
        if !self.repeating && repeating && self.finished {
            self.elapsed = 0.0;
            self.finished = self.just_finished();
        }
        self.repeating = repeating
    }

    /// Advances the timer by `delta` seconds.
    pub fn tick(&mut self, delta: f32) -> &Self {
        if self.paused {
            return self;
        }

        let prev_finished = self.finished;
        self.elapsed += delta;
        self.finished = self.elapsed >= self.duration;

        if self.finished {
            if self.repeating {
                // Count the number of times the timer will wrap around from this tick
                self.just_finished_count = (self.elapsed / self.duration) as u32;
                // Repeating timers wrap around
                self.elapsed %= self.duration;
            } else {
                self.just_finished_count = if prev_finished { 0 } else { 1 };
                // Non-repeating timers clamp to duration
                self.elapsed = self.duration;
            }
        } else {
            self.just_finished_count = 0;
        }
        self
    }

    #[inline]
    pub fn reset(&mut self) {
        self.finished = false;
        self.just_finished_count = 0;
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

#[cfg(test)]
mod tests {
    use super::Timer;

    #[test]
    fn test_non_repeating() {
        let mut t = Timer::from_seconds(10.0, false);
        // Tick once, check all attributes
        t.tick(0.25);
        assert_eq!(t.elapsed(), 0.25);
        assert_eq!(t.duration(), 10.0);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.just_finished_count(), 0);
        assert_eq!(t.repeating(), false);
        assert_eq!(t.percent(), 0.025);
        assert_eq!(t.percent_left(), 0.975);
        // Ticking while paused changes nothing
        t.pause();
        t.tick(500.0);
        assert_eq!(t.elapsed(), 0.25);
        assert_eq!(t.duration(), 10.0);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.just_finished_count(), 0);
        assert_eq!(t.repeating(), false);
        assert_eq!(t.percent(), 0.025);
        assert_eq!(t.percent_left(), 0.975);
        // Tick past the end and make sure elapsed doesn't go past 0.0 and other things update
        t.unpause();
        t.tick(500.0);
        assert_eq!(t.elapsed(), 10.0);
        assert_eq!(t.finished(), true);
        assert_eq!(t.just_finished(), true);
        assert_eq!(t.just_finished_count(), 1);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
        // Continuing to tick when finished should only change just_finished
        t.tick(1.0);
        assert_eq!(t.elapsed(), 10.0);
        assert_eq!(t.finished(), true);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.just_finished_count(), 0);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
    }

    #[test]
    fn test_repeating() {
        let mut t = Timer::from_seconds(2.0, true);
        // Tick once, check all attributes
        t.tick(0.75);
        assert_eq!(t.elapsed(), 0.75);
        assert_eq!(t.duration(), 2.0);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.just_finished_count(), 0);
        assert_eq!(t.repeating(), true);
        assert_eq!(t.percent(), 0.375);
        assert_eq!(t.percent_left(), 0.625);
        // Tick past the end and make sure elapsed wraps
        t.tick(3.5);
        assert_eq!(t.elapsed(), 0.25);
        assert_eq!(t.finished(), true);
        assert_eq!(t.just_finished(), true);
        assert_eq!(t.just_finished_count(), 2);
        assert_eq!(t.percent(), 0.125);
        assert_eq!(t.percent_left(), 0.875);
        // Continuing to tick should turn off both finished & just_finished for repeating timers
        t.tick(1.0);
        assert_eq!(t.elapsed(), 1.25);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.just_finished_count(), 0);
        assert_eq!(t.percent(), 0.625);
        assert_eq!(t.percent_left(), 0.375);
    }
}
