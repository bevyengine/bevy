use crate::Stopwatch;
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::Duration;

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
#[derive(Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Timer<T: Send + Sync + 'static = ()> {
    stopwatch: Stopwatch<T>,
    duration: f32,
    repeating: bool,
    finished: bool,
    times_finished: i32,
}

impl<T: Send + Sync + 'static> Timer<T> {
    /// Creates a new timer with a given duration.
    ///
    /// See also [`Timer::from_seconds`](Timer<T>::from_seconds).
    pub fn new(duration: Duration, repeating: bool) -> Self {
        Self {
            duration: duration.as_secs_f32(),
            repeating,
            ..Default::default()
        }
    }

    /// Creates a new timer with a given duration in seconds.
    ///
    /// # Example
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// ```
    pub fn from_seconds(duration: f32, repeating: bool) -> Self {
        Self {
            duration,
            repeating,
            ..Default::default()
        }
    }

    /// Returns `true` if the timer has reached its duration.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.tick(1.5);
    /// assert!(timer.finished());
    /// timer.tick(0.5);
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
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.tick(1.5);
    /// assert!(timer.just_finished());
    /// timer.tick(0.5);
    /// assert!(!timer.just_finished());
    /// ```
    #[inline]
    pub fn just_finished(&self) -> bool {
        self.times_finished > 0
    }

    /// Returns the elapsed time of the timer.
    ///
    /// See also [`Stopwatch::elapsed`](Stopwatch<T>::elapsed).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.tick(0.5);
    /// assert_eq!(timer.elapsed(), 0.5);
    /// ```
    #[inline]
    pub fn elapsed(&self) -> f32 {
        self.stopwatch.elapsed()
    }

    /// Sets the elapsed time of the timer without any other considerations.
    ///
    /// See also [`Stopwatch::set`](Stopwatch<T>::set).
    ///
    /// #
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.set_elapsed(1.5);
    /// assert_eq!(timer.elapsed(), 1.5);
    /// // the timer is not finished even if the elapsed time is greater than the duration.
    /// assert!(!timer.finished());
    /// ```
    /// ```
    #[inline]
    pub fn set_elapsed(&mut self, time: f32) {
        self.stopwatch.set(time);
    }

    /// Returns the duration of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let timer: Timer<()> = Timer::from_seconds(1.5, false);
    /// assert_eq!(timer.duration(), 1.5);
    /// ```
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Sets the duration of the timer.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.5, false);
    /// timer.set_duration(1.0);
    /// assert_eq!(timer.duration(), 1.0);
    /// ```
    #[inline]
    pub fn set_duration(&mut self, duration: f32) {
        self.duration = duration;
    }

    /// Returns `true` if the timer is repeating.
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, true);
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
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, true);
    /// timer.set_repeating(false);
    /// assert!(!timer.repeating());
    /// ```
    #[inline]
    pub fn set_repeating(&mut self, repeating: bool) {
        self.repeating = repeating
    }

    /// Advance the timer by `delta` seconds.
    /// Non repeating timer will clamp at duration.
    /// Repeating timer will wrap around.
    ///
    /// See also [`Stopwatch::tick`](Stopwatch<T>::tick).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// let mut repeating: Timer<()> = Timer::from_seconds(1.0, true);
    /// timer.tick(1.5);
    /// repeating.tick(1.5);
    /// assert_eq!(timer.elapsed(), 1.0);
    /// assert_eq!(repeating.elapsed(), 0.5);
    /// ```
    pub fn tick(&mut self, delta: f32) -> &Self {
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
                self.times_finished = (self.elapsed() / self.duration()).floor() as i32;
                self.stopwatch.set(self.stopwatch.elapsed() % self.duration);
            } else {
                self.times_finished = 1;
                self.stopwatch.set(self.duration());
            }
        } else {
            self.times_finished = 0;
        }

        self
    }

    /// Pauses the Timer. Disables the ticking of the timer.
    ///
    /// See also [`Stopwatch::pause`](Stopwatch<T>::pause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.pause();
    /// timer.tick(0.5);
    /// assert_eq!(timer.elapsed(), 0.0);
    /// ```
    #[inline]
    pub fn pause(&mut self) {
        self.stopwatch.pause();
    }

    /// Unpauses the Timer. Resumes the ticking of the timer.
    ///
    /// See also [`Stopwatch::unpause()`](Stopwatch<T>::unpause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.pause();
    /// timer.tick(0.5);
    /// timer.unpause();
    /// timer.tick(0.5);
    /// assert_eq!(timer.elapsed(), 0.5);
    /// ```
    #[inline]
    pub fn unpause(&mut self) {
        self.stopwatch.unpause();
    }

    /// Returns `true` if the timer is paused.
    ///
    /// See also [`Stopwatch::paused`](Stopwatch<T>::paused).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
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
    /// See also [`Stopwatch::reset`](Stopwatch<T>::reset).
    ///
    /// Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, false);
    /// timer.tick(1.5);
    /// timer.reset();
    /// assert!(!timer.finished());
    /// assert!(!timer.just_finished());
    /// assert_eq!(timer.elapsed(), 0.0);
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
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(2.0, false);
    /// timer.tick(0.5);
    /// assert_eq!(timer.percent(), 0.25);
    /// ```
    #[inline]
    pub fn percent(&self) -> f32 {
        self.elapsed() / self.duration()
    }

    /// Returns the percentage of the timer remaining time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(2.0, false);
    /// timer.tick(0.5);
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
    /// # use bevy_time::*;
    /// let mut timer: Timer<()> = Timer::from_seconds(1.0, true);
    /// timer.tick(6.0);
    /// assert_eq!(timer.times_finished(), 6);
    /// timer.tick(2.0);
    /// assert_eq!(timer.times_finished(), 2);
    /// timer.tick(0.5);
    /// assert_eq!(timer.times_finished(), 0);
    /// ```
    #[inline]
    pub fn times_finished(&self) -> i32 {
        self.times_finished
    }
}

impl<T: Send + Sync + 'static> Default for Timer<T> {
    fn default() -> Self {
        Self {
            duration: 1.0,
            repeating: Default::default(),
            stopwatch: Default::default(),
            finished: Default::default(),
            times_finished: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_repeating_timer() {
        let mut t: Timer<()> = Timer::from_seconds(10.0, false);
        // Tick once, check all attributes
        t.tick(0.25);
        assert_eq!(t.elapsed(), 0.25);
        assert_eq!(t.duration(), 10.0);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
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
        assert_eq!(t.repeating(), false);
        assert_eq!(t.percent(), 0.025);
        assert_eq!(t.percent_left(), 0.975);
        // Tick past the end and make sure elapsed doesn't go past 0.0 and other things update
        t.unpause();
        t.tick(500.0);
        assert_eq!(t.elapsed(), 10.0);
        assert_eq!(t.finished(), true);
        assert_eq!(t.just_finished(), true);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
        // Continuing to tick when finished should only change just_finished
        t.tick(1.0);
        assert_eq!(t.elapsed(), 10.0);
        assert_eq!(t.finished(), true);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.percent(), 1.0);
        assert_eq!(t.percent_left(), 0.0);
    }

    #[test]
    fn repeating_timer() {
        let mut t: Timer<()> = Timer::from_seconds(2.0, true);
        // Tick once, check all attributes
        t.tick(0.75);
        assert_eq!(t.elapsed(), 0.75);
        assert_eq!(t.duration(), 2.0);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.repeating(), true);
        assert_eq!(t.percent(), 0.375);
        assert_eq!(t.percent_left(), 0.625);
        // Tick past the end and make sure elapsed wraps
        t.tick(1.5);
        assert_eq!(t.elapsed(), 0.25);
        assert_eq!(t.finished(), true);
        assert_eq!(t.just_finished(), true);
        assert_eq!(t.percent(), 0.125);
        assert_eq!(t.percent_left(), 0.875);
        // Continuing to tick should turn off both finished & just_finished for repeating timers
        t.tick(1.0);
        assert_eq!(t.elapsed(), 1.25);
        assert_eq!(t.finished(), false);
        assert_eq!(t.just_finished(), false);
        assert_eq!(t.percent(), 0.625);
        assert_eq!(t.percent_left(), 0.375);
    }

    #[test]
    fn times_finished_repeating() {
        let mut t: Timer<()> = Timer::from_seconds(1.0, true);
        assert_eq!(t.times_finished(), 0);
        t.tick(3.5);
        assert_eq!(t.times_finished(), 3);
        assert_eq!(t.elapsed(), 0.5);
        assert!(t.finished());
        assert!(t.just_finished());
        t.tick(0.2);
        assert_eq!(t.times_finished(), 0);
    }

    #[test]
    fn times_finished() {
        let mut t: Timer<()> = Timer::from_seconds(1.0, false);
        assert_eq!(t.times_finished(), 0);
        t.tick(1.5);
        assert_eq!(t.times_finished(), 1);
        t.tick(0.5);
        assert_eq!(t.times_finished(), 0);
    }
}
