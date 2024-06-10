use crate::Stopwatch;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or
/// exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
///
/// Note that in order to advance the timer [`tick`](Timer::tick) **MUST** be called.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
pub struct Timer {
    stopwatch: Stopwatch,
    duration: Duration,
    mode: TimerMode,
    finished: bool,
    times_finished_this_tick: u32,
}

impl Timer {
    /// Creates a new timer with a given duration.
    ///
    /// See also [`Timer::from_seconds`](Timer::from_seconds).
    pub fn new(duration: Duration, mode: TimerMode) -> Self {
        Self {
            duration,
            mode,
            ..Default::default()
        }
    }

    /// Creates a new timer with a given duration in seconds.
    ///
    /// # Example
    /// ```
    /// # use bevy_time::*;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
    /// ```
    pub fn from_seconds(duration: f32, mode: TimerMode) -> Self {
        Self {
            duration: Duration::from_secs_f32(duration),
            mode,
            ..Default::default()
        }
    }

    /// Returns `true` if the timer has reached its duration.
    ///
    /// For repeating timers, this method behaves identically to [`Timer::just_finished`].
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    ///
    /// let mut timer_once = Timer::from_seconds(1.0, TimerMode::Once);
    /// timer_once.tick(Duration::from_secs_f32(1.5));
    /// assert!(timer_once.finished());
    /// timer_once.tick(Duration::from_secs_f32(0.5));
    /// assert!(timer_once.finished());
    ///
    /// let mut timer_repeating = Timer::from_seconds(1.0, TimerMode::Repeating);
    /// timer_repeating.tick(Duration::from_secs_f32(1.1));
    /// assert!(timer_repeating.finished());
    /// timer_repeating.tick(Duration::from_secs_f32(0.8));
    /// assert!(!timer_repeating.finished());
    /// timer_repeating.tick(Duration::from_secs_f32(0.6));
    /// assert!(timer_repeating.finished());
    /// ```
    #[inline]
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Returns `true` only on the tick the timer reached its duration.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// assert!(timer.just_finished());
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert!(!timer.just_finished());
    /// ```
    #[inline]
    pub fn just_finished(&self) -> bool {
        self.times_finished_this_tick > 0
    }

    /// Returns the time elapsed on the timer. Guaranteed to be between 0.0 and `duration`.
    /// Will only equal `duration` when the timer is finished and non repeating.
    ///
    /// See also [`Stopwatch::elapsed`](Stopwatch::elapsed).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed(), Duration::from_secs_f32(0.5));
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.stopwatch.elapsed()
    }

    /// Returns the time elapsed on the timer as an `f32`.
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
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
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
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let timer = Timer::new(Duration::from_secs(1), TimerMode::Once);
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
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.5, TimerMode::Once);
    /// timer.set_duration(Duration::from_secs(1));
    /// assert_eq!(timer.duration(), Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Returns the mode of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Repeating);
    /// assert_eq!(timer.mode(), TimerMode::Repeating);
    /// ```
    #[inline]
    pub fn mode(&self) -> TimerMode {
        self.mode
    }

    /// Sets the mode of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Repeating);
    /// timer.set_mode(TimerMode::Once);
    /// assert_eq!(timer.mode(), TimerMode::Once);
    /// ```
    #[doc(alias = "repeating")]
    #[inline]
    pub fn set_mode(&mut self, mode: TimerMode) {
        if self.mode != TimerMode::Repeating && mode == TimerMode::Repeating && self.finished {
            self.stopwatch.reset();
            self.finished = self.just_finished();
        }
        self.mode = mode;
    }

    /// Advance the timer by `delta` seconds.
    /// Non repeating timer will clamp at duration.
    /// Repeating timer will wrap around.
    /// Will not affect paused timers.
    ///
    /// See also [`Stopwatch::tick`](Stopwatch::tick).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
    /// let mut repeating = Timer::from_seconds(1.0, TimerMode::Repeating);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// repeating.tick(Duration::from_secs_f32(1.5));
    /// assert_eq!(timer.elapsed_secs(), 1.0);
    /// assert_eq!(repeating.elapsed_secs(), 0.5);
    /// ```
    pub fn tick(&mut self, delta: Duration) -> &Self {
        if self.paused() {
            self.times_finished_this_tick = 0;
            if self.mode == TimerMode::Repeating {
                self.finished = false;
            }
            return self;
        }

        if self.mode != TimerMode::Repeating && self.finished() {
            self.times_finished_this_tick = 0;
            return self;
        }

        self.stopwatch.tick(delta);
        self.finished = self.elapsed() >= self.duration();

        if self.finished() {
            if self.mode == TimerMode::Repeating {
                self.times_finished_this_tick = self
                    .elapsed()
                    .as_nanos()
                    .checked_div(self.duration().as_nanos())
                    .map_or(u32::MAX, |x| x as u32);
                self.set_elapsed(
                    self.elapsed()
                        .as_nanos()
                        .checked_rem(self.duration().as_nanos())
                        .map_or(Duration::ZERO, |x| Duration::from_nanos(x as u64)),
                );
            } else {
                self.times_finished_this_tick = 1;
                self.set_elapsed(self.duration());
            }
        } else {
            self.times_finished_this_tick = 0;
        }

        self
    }

    /// Pauses the Timer. Disables the ticking of the timer.
    ///
    /// See also [`Stopwatch::pause`](Stopwatch::pause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
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
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
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
    /// # use bevy_time::*;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
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

    /// Resets the timer. The reset doesn't affect the `paused` state of the timer.
    ///
    /// See also [`Stopwatch::reset`](Stopwatch::reset).
    ///
    /// Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// timer.reset();
    /// assert!(!timer.finished());
    /// assert!(!timer.just_finished());
    /// assert_eq!(timer.elapsed_secs(), 0.0);
    /// ```
    pub fn reset(&mut self) {
        self.stopwatch.reset();
        self.finished = false;
        self.times_finished_this_tick = 0;
    }

    /// Returns the fraction of the timer elapsed time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(2.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.fraction(), 0.25);
    /// ```
    #[inline]
    pub fn fraction(&self) -> f32 {
        if self.duration == Duration::ZERO {
            1.0
        } else {
            self.elapsed().as_secs_f32() / self.duration().as_secs_f32()
        }
    }

    /// Returns the fraction of the timer remaining time (goes from 1.0 to 0.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(2.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.fraction_remaining(), 0.75);
    /// ```
    #[inline]
    pub fn fraction_remaining(&self) -> f32 {
        1.0 - self.fraction()
    }

    /// Returns the remaining time in seconds
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::cmp::Ordering;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(2.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// let result = timer.remaining_secs().total_cmp(&1.5);
    /// assert_eq!(Ordering::Equal, result);
    /// ```
    #[inline]
    pub fn remaining_secs(&self) -> f32 {
        self.remaining().as_secs_f32()
    }

    /// Returns the remaining time using Duration
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(2.0, TimerMode::Once);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.remaining(), Duration::from_secs_f32(1.5));
    /// ```
    #[inline]
    pub fn remaining(&self) -> Duration {
        self.duration() - self.elapsed()
    }

    /// Returns the number of times a repeating timer
    /// finished during the last [`tick`](Timer<T>::tick) call.
    ///
    /// For non repeating-timers, this method will only ever
    /// return 0 or 1.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// use std::time::Duration;
    /// let mut timer = Timer::from_seconds(1.0, TimerMode::Repeating);
    /// timer.tick(Duration::from_secs_f32(6.0));
    /// assert_eq!(timer.times_finished_this_tick(), 6);
    /// timer.tick(Duration::from_secs_f32(2.0));
    /// assert_eq!(timer.times_finished_this_tick(), 2);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.times_finished_this_tick(), 0);
    /// ```
    #[inline]
    pub fn times_finished_this_tick(&self) -> u32 {
        self.times_finished_this_tick
    }
}

/// Specifies [`Timer`] behavior.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
pub enum TimerMode {
    /// Run once and stop.
    #[default]
    Once,
    /// Reset when finished.
    Repeating,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn non_repeating_timer() {
        let mut t = Timer::from_seconds(10.0, TimerMode::Once);
        // Tick once, check all attributes
        t.tick(Duration::from_secs_f32(0.25));
        assert_eq!(t.elapsed_secs(), 0.25);
        assert_eq!(t.duration(), Duration::from_secs_f32(10.0));
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.mode(), TimerMode::Once);
        assert_eq!(t.fraction(), 0.025);
        assert_eq!(t.fraction_remaining(), 0.975);
        // Ticking while paused changes nothing
        t.pause();
        t.tick(Duration::from_secs_f32(500.0));
        assert_eq!(t.elapsed_secs(), 0.25);
        assert_eq!(t.duration(), Duration::from_secs_f32(10.0));
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.mode(), TimerMode::Once);
        assert_eq!(t.fraction(), 0.025);
        assert_eq!(t.fraction_remaining(), 0.975);
        // Tick past the end and make sure elapsed doesn't go past 0.0 and other things update
        t.unpause();
        t.tick(Duration::from_secs_f32(500.0));
        assert_eq!(t.elapsed_secs(), 10.0);
        assert!(t.finished());
        assert!(t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 1);
        assert_eq!(t.fraction(), 1.0);
        assert_eq!(t.fraction_remaining(), 0.0);
        // Continuing to tick when finished should only change just_finished
        t.tick(Duration::from_secs_f32(1.0));
        assert_eq!(t.elapsed_secs(), 10.0);
        assert!(t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.fraction(), 1.0);
        assert_eq!(t.fraction_remaining(), 0.0);
    }

    #[test]
    fn repeating_timer() {
        let mut t = Timer::from_seconds(2.0, TimerMode::Repeating);
        // Tick once, check all attributes
        t.tick(Duration::from_secs_f32(0.75));
        assert_eq!(t.elapsed_secs(), 0.75);
        assert_eq!(t.duration(), Duration::from_secs_f32(2.0));
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.mode(), TimerMode::Repeating);
        assert_eq!(t.fraction(), 0.375);
        assert_eq!(t.fraction_remaining(), 0.625);
        // Tick past the end and make sure elapsed wraps
        t.tick(Duration::from_secs_f32(1.5));
        assert_eq!(t.elapsed_secs(), 0.25);
        assert!(t.finished());
        assert!(t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 1);
        assert_eq!(t.fraction(), 0.125);
        assert_eq!(t.fraction_remaining(), 0.875);
        // Continuing to tick should turn off both finished & just_finished for repeating timers
        t.tick(Duration::from_secs_f32(1.0));
        assert_eq!(t.elapsed_secs(), 1.25);
        assert!(!t.finished());
        assert!(!t.just_finished());
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.fraction(), 0.625);
        assert_eq!(t.fraction_remaining(), 0.375);
    }

    #[test]
    fn times_finished_repeating() {
        let mut t = Timer::from_seconds(1.0, TimerMode::Repeating);
        assert_eq!(t.times_finished_this_tick(), 0);
        t.tick(Duration::from_secs_f32(3.5));
        assert_eq!(t.times_finished_this_tick(), 3);
        assert_eq!(t.elapsed_secs(), 0.5);
        assert!(t.finished());
        assert!(t.just_finished());
        t.tick(Duration::from_secs_f32(0.2));
        assert_eq!(t.times_finished_this_tick(), 0);
    }

    #[test]
    fn times_finished_this_tick() {
        let mut t = Timer::from_seconds(1.0, TimerMode::Once);
        assert_eq!(t.times_finished_this_tick(), 0);
        t.tick(Duration::from_secs_f32(1.5));
        assert_eq!(t.times_finished_this_tick(), 1);
        t.tick(Duration::from_secs_f32(0.5));
        assert_eq!(t.times_finished_this_tick(), 0);
    }

    #[test]
    fn times_finished_this_tick_repeating_zero_duration() {
        let mut t = Timer::from_seconds(0.0, TimerMode::Repeating);
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.elapsed(), Duration::ZERO);
        assert_eq!(t.fraction(), 1.0);
        t.tick(Duration::from_secs(1));
        assert_eq!(t.times_finished_this_tick(), u32::MAX);
        assert_eq!(t.elapsed(), Duration::ZERO);
        assert_eq!(t.fraction(), 1.0);
        t.tick(Duration::from_secs(2));
        assert_eq!(t.times_finished_this_tick(), u32::MAX);
        assert_eq!(t.elapsed(), Duration::ZERO);
        assert_eq!(t.fraction(), 1.0);
        t.reset();
        assert_eq!(t.times_finished_this_tick(), 0);
        assert_eq!(t.elapsed(), Duration::ZERO);
        assert_eq!(t.fraction(), 1.0);
    }

    #[test]
    fn times_finished_this_tick_precise() {
        let mut t = Timer::from_seconds(0.01, TimerMode::Repeating);
        let duration = Duration::from_secs_f64(0.333);

        // total duration: 0.333 => 33 times finished
        t.tick(duration);
        assert_eq!(t.times_finished_this_tick(), 33);
        // total duration: 0.666 => 33 times finished
        t.tick(duration);
        assert_eq!(t.times_finished_this_tick(), 33);
        // total duration: 0.999 => 33 times finished
        t.tick(duration);
        assert_eq!(t.times_finished_this_tick(), 33);
        // total duration: 1.332 => 34 times finished
        t.tick(duration);
        assert_eq!(t.times_finished_this_tick(), 34);
    }

    #[test]
    fn paused() {
        let mut t = Timer::from_seconds(10.0, TimerMode::Once);

        t.tick(Duration::from_secs_f32(10.0));
        assert!(t.just_finished());
        assert!(t.finished());
        // A paused timer should change just_finished to false after a tick
        t.pause();
        t.tick(Duration::from_secs_f32(5.0));
        assert!(!t.just_finished());
        assert!(t.finished());
    }

    #[test]
    fn paused_repeating() {
        let mut t = Timer::from_seconds(10.0, TimerMode::Repeating);

        t.tick(Duration::from_secs_f32(10.0));
        assert!(t.just_finished());
        assert!(t.finished());
        // A paused repeating timer should change finished and just_finished to false after a tick
        t.pause();
        t.tick(Duration::from_secs_f32(5.0));
        assert!(!t.just_finished());
        assert!(!t.finished());
    }
}
