use crate::Stopwatch;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::Reflect;
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or
/// exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Timer {
    stopwatch: Stopwatch,
    duration: Duration,
    repeating: bool,
    finished: bool,
    times_finished: u32,
}

impl Timer {
    /// Creates a new timer with a given duration.
    ///
    /// See also [`Timer::from_seconds`](Timer::from_seconds).
    pub fn new(duration: Duration, repeating: bool) -> Self {
        Self {
            duration,
            repeating,
            ..Default::default()
        }
    }

    /// Creates a new timer with a given duration in seconds.
    ///
    /// # Example
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// ```
    pub fn from_seconds(duration: f32, repeating: bool) -> Self {
        Self {
            duration: Duration::from_secs_f32(duration),
            repeating,
            ..Default::default()
        }
    }

    /// Returns `true` if the timer has reached its duration.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// assert!(timer.finished());
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert!(timer.finished());
    /// ```
    #[inline]
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Returns `true` only on the tick the timer reached its duration.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// assert!(timer.just_finished());
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert!(!timer.just_finished());
    /// ```
    #[inline]
    pub fn just_finished(&self) -> bool {
        self.times_finished > 0
    }

    /// Returns the time elapsed on the timer. Guaranteed to be between 0.0 and `duration`.
    /// Will only equal `duration` when the timer is finished and non repeating.
    ///
    /// See also [`Stopwatch::elapsed`](Stopwatch::elapsed).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed(), Duration::from_secs_f32(0.5));
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.stopwatch.elapsed()
    }

    /// Returns the time elapsed on the timer as a `f32`.
    /// See also [`Timer::elapsed`](Timer::elapsed).
    #[inline]
    pub fn elapsed_secs(&self) -> f32 {
        self.stopwatch.elapsed_secs()
    }

    /// Sets the elapsed time of the timer without any other considerations.
    ///
    /// See also [`Stopwatch::set`](Stopwatch::set).
    ///
    /// #
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.set_elapsed(Duration::from_secs(2));
    /// assert_eq!(timer.elapsed(), Duration::from_secs(2));
    /// // the timer is not finished even if the elapsed time is greater than the duration.
    /// assert!(!timer.finished());
    /// ```
    #[inline]
    pub fn set_elapsed(&mut self, time: Duration) {
        self.stopwatch.set_elapsed(time);
    }

    /// Returns the duration of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let timer = Timer::new(Duration::from_secs(1), false);
    /// assert_eq!(timer.duration(), Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Sets the duration of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.5, false);
    /// timer.set_duration(Duration::from_secs(1));
    /// assert_eq!(timer.duration(), Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Returns `true` if the timer is repeating.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = Timer::from_seconds(1.0, true);
    /// assert!(timer.repeating());
    /// ```
    #[inline]
    pub fn repeating(&self) -> bool {
        self.repeating
    }

    /// Sets whether the timer is repeating or not.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = Timer::from_seconds(1.0, true);
    /// timer.set_repeating(false);
    /// assert!(!timer.repeating());
    /// ```
    #[inline]
    pub fn set_repeating(&mut self, repeating: bool) {
        if !self.repeating && repeating && self.finished {
            self.stopwatch.reset();
            self.finished = self.just_finished();
        }
        self.repeating = repeating
    }

    /// Advance the timer by `delta` seconds.
    /// Non repeating timer will clamp at duration.
    /// Repeating timer will wrap around.
    ///
    /// See also [`Stopwatch::tick`](Stopwatch::tick).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// let mut repeating = Timer::from_seconds(1.0, true);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// repeating.tick(Duration::from_secs_f32(1.5));
    /// assert_eq!(timer.elapsed_secs(), 1.0);
    /// assert_eq!(repeating.elapsed_secs(), 0.5);
    /// ```
    pub fn tick(&mut self, delta: Duration) -> &Self {
        if self.paused() {
            return self;
        }

        if !self.repeating() && self.finished() {
            self.times_finished = 0;
            return self;
        }

        self.stopwatch.tick(delta);
        self.finished = self.elapsed() >= self.duration();

        if self.finished() {
            if self.repeating() {
                self.times_finished =
                    (self.elapsed().as_nanos() / self.duration().as_nanos()) as u32;
                // Duration does not have a modulo
                self.set_elapsed(self.elapsed() - self.duration() * self.times_finished);
            } else {
                self.times_finished = 1;
                self.set_elapsed(self.duration());
            }
        } else {
            self.times_finished = 0;
        }

        self
    }

    /// Pauses the Timer. Disables the ticking of the timer.
    ///
    /// See also [`Stopwatch::pause`](Stopwatch::pause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.pause();
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed_secs(), 0.0);
    /// ```
    #[inline]
    pub fn pause(&mut self) {
        self.stopwatch.pause();
    }

    /// Unpauses the Timer. Resumes the ticking of the timer.
    ///
    /// See also [`Stopwatch::unpause()`](Stopwatch::unpause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.pause();
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// timer.unpause();
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed_secs(), 0.5);
    /// ```
    #[inline]
    pub fn unpause(&mut self) {
        self.stopwatch.unpause();
    }

    /// Returns `true` if the timer is paused.
    ///
    /// See also [`Stopwatch::paused`](Stopwatch::paused).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// assert!(!timer.paused());
    /// timer.pause();
    /// assert!(timer.paused());
    /// timer.unpause();
    /// assert!(!timer.paused());
    /// ```
    #[inline]
    pub fn paused(&self) -> bool {
        self.stopwatch.paused()
    }

    /// Resets the timer. the reset doesn't affect the `paused` state of the timer.
    ///
    /// See also [`Stopwatch::reset`](Stopwatch::reset).
    ///
    /// Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// timer.reset();
    /// assert!(!timer.finished());
    /// assert!(!timer.just_finished());
    /// assert_eq!(timer.elapsed_secs(), 0.0);
    /// ```
    pub fn reset(&mut self) {
        self.stopwatch.reset();
        self.finished = false;
        self.times_finished = 0;
    }

    /// Returns the percentage of the timer elapsed time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(2.0, false);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.percent(), 0.25);
    /// ```
    #[inline]
    pub fn percent(&self) -> f32 {
        self.elapsed().as_secs_f32() / self.duration().as_secs_f32()
    }

    /// Returns the percentage of the timer remaining time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(2.0, false);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.percent_left(), 0.75);
    /// ```
    #[inline]
    pub fn percent_left(&self) -> f32 {
        1.0 - self.percent()
    }

    /// Returns the number of times a repeating timer
    /// finished during the last [`tick`](Timer<T>::tick) call.
    ///
    /// For non repeating-timers, this method will only ever
    /// return 0 or 1.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, true);
    /// timer.tick(Duration::from_secs_f32(6.0));
    /// assert_eq!(timer.times_finished(), 6);
    /// timer.tick(Duration::from_secs_f32(2.0));
    /// assert_eq!(timer.times_finished(), 2);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.times_finished(), 0);
    /// ```
    #[inline]
    pub fn times_finished(&self) -> u32 {
        self.times_finished
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn non_repeating_timer() {
        let mut t = Timer::from_seconds(10.0, false);
        // Tick once, check all attributes
        t.tick(Duration::from_secs_f32(0.25));
        assert_eq!(t.elapsed_secs(), 0.25);
        assert_eq!(t.duration(), Duration::from_secs_f32(10.0));
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished(), 0);
        assert!(!t.repeating());
        assert_eq!(t.percent(), 0.025);
        assert_eq!(t.percent_left(), 0.975);
        // Ticking while paused changes nothing
        t.pause();
        t.tick(Duration::from_secs_f32(500.0));
        assert_eq!(t.elapsed_secs(), 0.25);
        assert_eq!(t.duration(), Duration::from_secs_f32(10.0));
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished(), 0);
        assert!(!t.repeating());
        assert_eq!(t.percent(), 0.025);
        assert_eq!(t.percent_left(), 0.975);
        // Tick past the end and make sure elapsed doesn't go past 0.0 and other things update
        t.unpause();
        t.tick(Duration::from_secs_f32(500.0));
        assert_eq!(t.elapsed_secs(), 10.0);
        assert!(t.finished());
        assert!(t.just_finished());
        assert_eq!(t.times_finished(), 1);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
        // Continuing to tick when finished should only change just_finished
        t.tick(Duration::from_secs_f32(1.0));
        assert_eq!(t.elapsed_secs(), 10.0);
        assert!(t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished(), 0);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
    }

    #[test]
    fn repeating_timer() {
        let mut t = Timer::from_seconds(2.0, true);
        // Tick once, check all attributes
        t.tick(Duration::from_secs_f32(0.75));
        assert_eq!(t.elapsed_secs(), 0.75);
        assert_eq!(t.duration(), Duration::from_secs_f32(2.0));
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished(), 0);
        assert!(t.repeating());
        assert_eq!(t.percent(), 0.375);
        assert_eq!(t.percent_left(), 0.625);
        // Tick past the end and make sure elapsed wraps
        t.tick(Duration::from_secs_f32(1.5));
        assert_eq!(t.elapsed_secs(), 0.25);
        assert!(t.finished());
        assert!(t.just_finished());
        assert_eq!(t.times_finished(), 1);
        assert_eq!(t.percent(), 0.125);
        assert_eq!(t.percent_left(), 0.875);
        // Continuing to tick should turn off both finished & just_finished for repeating timers
        t.tick(Duration::from_secs_f32(1.0));
        assert_eq!(t.elapsed_secs(), 1.25);
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished(), 0);
        assert_eq!(t.percent(), 0.625);
        assert_eq!(t.percent_left(), 0.375);
    }

    #[test]
    fn times_finished_repeating() {
        let mut t = Timer::from_seconds(1.0, true);
        assert_eq!(t.times_finished(), 0);
        t.tick(Duration::from_secs_f32(3.5));
        assert_eq!(t.times_finished(), 3);
        assert_eq!(t.elapsed_secs(), 0.5);
        assert!(t.finished());
        assert!(t.just_finished());
        t.tick(Duration::from_secs_f32(0.2));
        assert_eq!(t.times_finished(), 0);
    }

    #[test]
    fn times_finished() {
        let mut t = Timer::from_seconds(1.0, false);
        assert_eq!(t.times_finished(), 0);
        t.tick(Duration::from_secs_f32(1.5));
        assert_eq!(t.times_finished(), 1);
        t.tick(Duration::from_secs_f32(0.5));
        assert_eq!(t.times_finished(), 0);
    }

    #[test]
    fn times_finished_precise() {
        let mut t = Timer::from_seconds(0.01, true);
        let duration = Duration::from_secs_f64(1.0 / 3.0);

        t.tick(duration);
        assert_eq!(t.times_finished(), 33);
        t.tick(duration);
        assert_eq!(t.times_finished(), 33);
        t.tick(duration);
        assert_eq!(t.times_finished(), 33);
        // It has one additional tick this time to compensate for missing 100th tick
        t.tick(duration);
        assert_eq!(t.times_finished(), 34);
    }
}
