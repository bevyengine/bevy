use std::ops::Add;

use crate::time::Stopwatch;
use crate::{DiscreteStopwatch, DurationStopwatch};
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once its duration is reached.
///
/// This type is useful for measuring wall-clock time, and can be used to track cooldowns, countdowns and recurring triggers.
/// Consider using ['DiscreteTimer'] for gameplay events whose interval should decrease in the case of frame-rate drops.
///
/// Non-repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or
/// exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
pub trait Timer {
    /// The unit by which elapsed time is measured
    type TimeUnit: Default + Add<Output = Self::TimeUnit> + PartialOrd;
    type EmbeddedStopwatch: Stopwatch<TimeUnit = Self::TimeUnit>;

    /// Creates a new timer with a given duration.
    fn new(duration: Self::TimeUnit, repeating: bool) -> Self;

    /// Returns `true` if the timer has reached its duration.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// assert!(timer.finished());
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert!(timer.finished());
    /// ```
    fn finished(&self) -> bool;

    /// Returns `true` only on the tick the timer reached its duration.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// assert!(timer.just_finished());
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert!(!timer.just_finished());
    /// ```
    #[inline]
    fn just_finished(&self) -> bool {
        self.times_finished() > 0
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
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed(), Duration::from_secs_f32(0.5));
    /// ```
    #[inline]
    fn elapsed(&self) -> Self::TimeUnit {
        self.stopwatch().elapsed()
    }

    /// Sets the elapsed time of the timer without any other considerations.
    ///
    /// See also [`Stopwatch::set`](Stopwatch::set).
    ///
    /// #
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.set_elapsed(Duration::from_secs(2));
    /// assert_eq!(timer.elapsed(), Duration::from_secs(2));
    /// // the timer is not finished even if the elapsed time is greater than the duration.
    /// assert!(!timer.finished());
    /// ```
    #[inline]
    fn set_elapsed(&mut self, time: Self::TimeUnit) {
        self.stopwatch_mut().set_elapsed(time);
    }

    /// Returns the duration of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let timer = DurationTimer::new(Duration::from_secs(1), false);
    /// assert_eq!(timer.duration(), Duration::from_secs(1));
    /// ```
    fn duration(&self) -> Self::TimeUnit;

    /// Sets the duration of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.5, false);
    /// timer.set_duration(Duration::from_secs(1));
    /// assert_eq!(timer.duration(), Duration::from_secs(1));
    /// ```
    fn set_duration(&mut self, duration: Self::TimeUnit);

    /// Returns `true` if the timer is repeating.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = DurationTimer::from_seconds(1.0, true);
    /// assert!(timer.repeating());
    /// ```
    fn repeating(&self) -> bool;

    /// Sets whether the timer is repeating or not.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = DurationTimer::from_seconds(1.0, true);
    /// timer.set_repeating(false);
    /// assert!(!timer.repeating());
    /// ```
    fn set_repeating(&mut self, repeating: bool);

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
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// let mut repeating = DurationTimer::from_seconds(1.0, true);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// repeating.tick(Duration::from_secs_f32(1.5));
    /// assert_eq!(timer.elapsed_secs(), 1.0);
    /// assert_eq!(repeating.elapsed_secs(), 0.5);
    /// ```
    fn tick(&mut self, delta: Self::TimeUnit) -> &Self;

    /// Pauses the Timer. Disables the ticking of the timer.
    ///
    /// See also [`Stopwatch::pause`](Stopwatch::pause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.pause();
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed_secs(), 0.0);
    /// ```
    fn pause(&mut self) {
        self.stopwatch_mut().pause();
    }

    /// Unpauses the Timer. Resumes the ticking of the timer.
    ///
    /// See also [`Stopwatch::unpause()`](Stopwatch::unpause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.pause();
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// timer.unpause();
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.elapsed_secs(), 0.5);
    /// ```
    #[inline]
    fn unpause(&mut self) {
        self.stopwatch_mut().unpause();
    }

    /// Returns `true` if the timer is paused.
    ///
    /// See also [`Stopwatch::paused`](Stopwatch::paused).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// assert!(!timer.paused());
    /// timer.pause();
    /// assert!(timer.paused());
    /// timer.unpause();
    /// assert!(!timer.paused());
    /// ```
    #[inline]
    fn paused(&self) -> bool {
        self.stopwatch().paused()
    }

    /// Resets the timer. the reset doesn't affect the `paused` state of the timer.
    ///
    /// See also [`Stopwatch::reset`](Stopwatch::reset).
    ///
    /// Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// timer.tick(Duration::from_secs_f32(1.5));
    /// timer.reset();
    /// assert!(!timer.finished());
    /// assert!(!timer.just_finished());
    /// assert_eq!(timer.elapsed_secs(), 0.0);
    /// ```
    fn reset(&mut self);

    /// Returns the percentage of the timer elapsed time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(2.0, false);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.percent(), 0.25);
    /// ```
    fn percent(&self) -> f32;

    /// Returns the percentage of the timer remaining time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut timer = DurationTimer::from_seconds(2.0, false);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.percent_left(), 0.75);
    /// ```
    #[inline]
    fn percent_left(&self) -> f32 {
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
    /// let mut timer = DurationTimer::from_seconds(1.0, true);
    /// timer.tick(Duration::from_secs_f32(6.0));
    /// assert_eq!(timer.times_finished(), 6);
    /// timer.tick(Duration::from_secs_f32(2.0));
    /// assert_eq!(timer.times_finished(), 2);
    /// timer.tick(Duration::from_secs_f32(0.5));
    /// assert_eq!(timer.times_finished(), 0);
    /// ```
    fn times_finished(&self) -> u32;

    /// Returns a reference to the `EmbeddedStopwatch`
    /// stored within the struct that implements this trait
    fn stopwatch(&self) -> &Self::EmbeddedStopwatch;

    /// Returns a mutable reference to the `EmbeddedStopwatch`
    /// stored within the struct that implements this trait
    fn stopwatch_mut(&mut self) -> &mut Self::EmbeddedStopwatch;
}

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// This [`Timer`] measures wall-clock time, and can be used to track cooldowns, countdowns and recurring triggers.
/// Consider using ['DiscreteTimer'] for gameplay events whose interval should decrease in the case of frame-rate drops.
///
/// Non-repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or
/// exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DurationTimer {
    stopwatch: DurationStopwatch,
    duration: Duration,
    repeating: bool,
    finished: bool,
    times_finished: u32,
}

impl DurationTimer {
    /// Creates a new timer with a given duration in seconds.
    ///
    /// # Example
    /// ```
    /// # use bevy_core::*;
    /// let mut timer = DurationTimer::from_seconds(1.0, false);
    /// ```
    pub fn from_seconds(duration: f32, repeating: bool) -> Self {
        Self {
            duration: Duration::from_secs_f32(duration),
            repeating,
            ..Default::default()
        }
    }

    /// Returns the time elapsed on the timer as a `f32`.
    /// See also [`Timer::elapsed`](Timer::elapsed).
    #[inline]
    pub fn elapsed_secs(&self) -> f32 {
        self.stopwatch.elapsed_secs()
    }
}

impl Timer for DurationTimer {
    type TimeUnit = Duration;
    type EmbeddedStopwatch = DurationStopwatch;

    fn new(duration: Self::TimeUnit, repeating: bool) -> Self {
        Self {
            duration,
            repeating,
            ..Default::default()
        }
    }

    fn finished(&self) -> bool {
        self.finished
    }

    fn duration(&self) -> Self::TimeUnit {
        self.duration
    }

    fn set_duration(&mut self, duration: Self::TimeUnit) {
        self.duration = duration;
    }

    fn repeating(&self) -> bool {
        self.repeating
    }

    fn set_repeating(&mut self, repeating: bool) {
        self.repeating = repeating
    }

    fn tick(&mut self, delta: Self::TimeUnit) -> &Self {
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

    fn reset(&mut self) {
        self.stopwatch.reset();
        self.finished = false;
        self.times_finished = 0;
    }

    fn percent(&self) -> f32 {
        self.elapsed().as_secs_f32() / self.duration().as_secs_f32()
    }

    fn times_finished(&self) -> u32 {
        self.times_finished
    }

    fn stopwatch(&self) -> &Self::EmbeddedStopwatch {
        &self.stopwatch
    }

    fn stopwatch_mut(&mut self) -> &mut Self::EmbeddedStopwatch {
        &mut self.stopwatch
    }
}

/// This ['Timer'] is useful for accurately counting frames, ticks or occurences between gameplay events.
/// Use ['DurationTimer'] if you care about wall-clock time.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DiscreteTimer {
    stopwatch: DiscreteStopwatch,
    duration: u64,
    repeating: bool,
    finished: bool,
    times_finished: u32,
}

impl Timer for DiscreteTimer {
    type TimeUnit = u64;
    type EmbeddedStopwatch = DiscreteStopwatch;

    fn new(duration: Self::TimeUnit, repeating: bool) -> Self {
        Self {
            duration,
            repeating,
            ..Default::default()
        }
    }

    fn finished(&self) -> bool {
        self.finished
    }

    fn duration(&self) -> Self::TimeUnit {
        self.duration
    }

    fn set_duration(&mut self, duration: Self::TimeUnit) {
        self.duration = duration;
    }

    fn repeating(&self) -> bool {
        self.repeating
    }

    fn set_repeating(&mut self, repeating: bool) {
        self.repeating = repeating
    }

    fn tick(&mut self, delta: Self::TimeUnit) -> &Self {
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
                self.times_finished = (self.elapsed() / self.duration()) as u32;
                self.set_elapsed(self.elapsed() % self.duration());
            } else {
                self.times_finished = 1;
                self.set_elapsed(self.duration());
            }
        } else {
            self.times_finished = 0;
        }

        self
    }

    fn reset(&mut self) {
        self.stopwatch.reset();
        self.finished = false;
        self.times_finished = 0;
    }

    fn percent(&self) -> f32 {
        self.elapsed() as f32 / self.duration() as f32
    }

    fn times_finished(&self) -> u32 {
        self.times_finished
    }

    fn stopwatch(&self) -> &Self::EmbeddedStopwatch {
        &self.stopwatch
    }

    fn stopwatch_mut(&mut self) -> &mut Self::EmbeddedStopwatch {
        &mut self.stopwatch
    }
}
