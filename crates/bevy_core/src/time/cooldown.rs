use crate::Stopwatch;
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::Duration;

#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Cooldown<T: Send + Sync + 'static> {
    stopwatch: Stopwatch<T>,
    duration: Duration,
    repeating: bool,
    available: bool,
    just_available: bool,
}

impl<T: Send + Sync + 'static> Cooldown<T> {
    /// Creates a new cooldown with a given duration.
    pub fn new(duration: Duration, repeating: bool) -> Self {
        Self {
            duration,
            repeating,
            ..Default::default()
        }
    }

    /// Creates a new cooldown with a given duration in seconds.
    ///
    /// # Example
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// ```
    pub fn from_seconds(duration: f32, repeating: bool) -> Self {
        Self {
            duration: Duration::from_secs_f32(duration),
            repeating,
            ..Default::default()
        }
    }

    /// Starts the cooldown.
    ///
    /// A call to `start` is mandatory for non-repeating cooldown
    /// as it's the only way to enable ticking.
    /// Repeating cooldown doesn't need to call `start` to enable ticking.
    #[inline]
    pub fn start(&mut self) {
        self.available = false;
        self.just_available = false;
    }

    /// Returns `true` if the cooldown is available to start.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// assert!(cooldown.available());
    /// cooldown.start();
    /// assert!(!cooldown.available());
    /// cooldown.tick_f32(1.5);
    /// assert!(cooldown.available());
    /// ```
    #[inline]
    pub fn available(&self) -> bool {
        self.available
    }

    /// Returns `true` only on the tick the cooldown became available.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// cooldown.start();
    /// cooldown.tick_f32(1.5);
    /// assert!(cooldown.just_available());
    /// cooldown.tick_f32(0.5);
    /// assert!(!cooldown.just_available());
    /// ```
    pub fn just_available(&self) -> bool {
        self.just_available
    }

    /// Returns the elapsed time of the cooldown.
    ///
    /// See also [`Stopwatch::elapsed`](Stopwatch<T>::elapsed).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// cooldown.start();
    /// cooldown.tick_f32(0.5);
    /// assert_eq!(cooldown.elapsed_f32(), 0.5);
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.stopwatch.elapsed()
    }

    pub fn elapsed_f32(&self) -> f32 {
        self.stopwatch.elapsed_f32()
    }

    /// Sets the elapsed time of the cooldown without any other considerations.
    ///
    /// See also [`Stopwatch::set`](Stopwatch<T>::set).
    ///
    /// #
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// cooldown.set_elapsed(Duration::from_secs_f32(1.5));
    /// assert_eq!(cooldown.elapsed(), Duration::from_secs_f32(1.5));
    /// // the cooldown is available even if the elapsed time is greater than the duration.
    /// assert!(cooldown.available());
    /// ```
    /// ```
    #[inline]
    pub fn set_elapsed(&mut self, time: Duration) {
        self.stopwatch.set(time);
    }

    /// Returns the duration of the cooldown.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let cooldown: Cooldown<()> = Cooldown::from_seconds(1.5, false);
    /// assert_eq!(cooldown.duration(), Duration::from_secs_f32(1.5));
    /// ```
    #[inline]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn duration_f32(&self) -> f32 {
        self.duration.as_secs_f32()
    }

    /// Sets the duration of the cooldown.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.5, false);
    /// cooldown.set_duration(Duration::from_secs(1));
    /// assert_eq!(cooldown.duration(), Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Returns `true` if the cooldown is repeating.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, true);
    /// assert!(cooldown.repeating());
    /// ```
    #[inline]
    pub fn repeating(&self) -> bool {
        self.repeating
    }

    /// Sets whether the Cooldown is repeating or not.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, true);
    /// cooldown.set_repeating(false);
    /// assert!(!cooldown.repeating());
    /// ```
    #[inline]
    pub fn set_repeating(&mut self, repeating: bool) {
        self.repeating = repeating
    }

    /// Advances the cooldown by `delta` seconds.
    ///
    pub fn tick(&mut self, delta: Duration) -> &Self {
        if self.paused() {
            return self;
        }

        if self.repeating() {
            self.tick_repeating(delta)
        } else {
            self.tick_non_repeating(delta)
        }
    }

    pub fn tick_f32(&mut self, delta: f32) -> &Self {
        self.tick(Duration::from_secs_f32(delta))
    }

    /// Pauses the Cooldown. Disables the ticking of the cooldown.
    ///
    /// See also [`Stopwatch::pause`](Stopwatch<T>::pause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// cooldown.pause();
    /// cooldown.tick_f32(0.5);
    /// assert_eq!(cooldown.elapsed_f32(), 0.0);
    /// ```
    #[inline]
    pub fn pause(&mut self) {
        self.stopwatch.pause();
    }

    /// Unpauses the Cooldown. Resumes the ticking of the cooldown.
    ///
    /// See also [`Stopwatch::unpause()`](Stopwatch<T>::unpause).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// cooldown.start();
    /// cooldown.pause();
    /// cooldown.tick_f32(0.5);
    /// cooldown.unpause();
    /// cooldown.tick_f32(0.5);
    /// assert_eq!(cooldown.elapsed_f32(), 0.5);
    /// ```
    #[inline]
    pub fn unpause(&mut self) {
        self.stopwatch.unpause();
    }

    /// Returns `true` if the cooldown is paused.
    ///
    /// See also [`Stopwatch::paused`](Stopwatch<T>::paused).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// assert!(!cooldown.paused());
    /// cooldown.pause();
    /// assert!(cooldown.paused());
    /// cooldown.unpause();
    /// assert!(!cooldown.paused());
    /// ```
    #[inline]
    pub fn paused(&self) -> bool {
        self.stopwatch.paused()
    }

    /// Resets the cooldown. the reset doesn't affect the `paused` state of the cooldown.
    ///
    /// See also [`Stopwatch::reset`](Stopwatch<T>::reset).
    ///
    /// Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(1.0, false);
    /// cooldown.tick_f32(1.5);
    /// cooldown.reset();
    /// assert!(cooldown.available());
    /// assert!(cooldown.just_available());
    /// assert_eq!(cooldown.elapsed_f32(), 0.0);
    /// ```
    pub fn reset(&mut self) {
        self.stopwatch.reset();
        self.available = true;
        self.just_available = true;
    }

    /// Returns the percentage of the cooldown elapsed time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(2.0, false);
    /// cooldown.start();
    /// cooldown.tick_f32(0.5);
    /// assert_eq!(cooldown.percent(), 0.25);
    /// ```
    #[inline]
    pub fn percent(&self) -> f32 {
        self.elapsed().as_secs_f32() / self.duration().as_secs_f32()
    }

    /// Returns the percentage of the cooldown remaining time (goes from 0.0 to 1.0).
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut cooldown: Cooldown<()> = Cooldown::from_seconds(2.0, false);
    /// cooldown.start();
    /// cooldown.tick_f32(0.5);
    /// assert_eq!(cooldown.percent_left(), 0.75);
    /// ```
    #[inline]
    pub fn percent_left(&self) -> f32 {
        1.0 - self.percent()
    }

    fn tick_repeating(&mut self, delta: Duration) -> &Self {
        let elapsed = self.stopwatch.tick(delta).elapsed();
        let coeff = self.percent().floor() as u32;
        if elapsed >= self.duration() {
            self.reset();
            self.set_elapsed(elapsed - self.duration() * coeff);
        } else {
            self.start();
        };

        self
    }

    fn tick_non_repeating(&mut self, delta: Duration) -> &Self {
        if self.available() {
            self.just_available = false;
            return self;
        }

        self.stopwatch.tick(delta);
        if self.elapsed() >= self.duration() {
            self.reset();
        }

        self
    }
}

impl<T: Send + Sync + 'static> Default for Cooldown<T> {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(1),
            repeating: Default::default(),
            stopwatch: Default::default(),
            available: true,
            just_available: true,
        }
    }
}

#[cfg(test)]
#[allow(clippy::clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn non_repeating_cooldown() {
        let mut cd: Cooldown<()> = Cooldown::from_seconds(10.0, false);
        // Tick once without starting, check all attributes
        cd.tick_f32(0.25);
        assert_eq!(cd.elapsed_f32(), 0.0);
        assert_eq!(cd.duration_f32(), 10.0);
        assert_eq!(cd.available(), true);
        assert_eq!(cd.just_available(), false);
        assert_eq!(cd.repeating(), false);
        assert_eq!(cd.percent(), 0.0);
        assert_eq!(cd.percent_left(), 1.0);

        cd.start();
        assert_eq!(cd.available(), false);
        assert_eq!(cd.just_available(), false);
        // Ticking while paused changes nothing
        cd.pause();
        cd.tick_f32(500.0);
        assert_eq!(cd.elapsed_f32(), 0.0);
        assert_eq!(cd.duration_f32(), 10.0);
        assert_eq!(cd.available(), false);
        assert_eq!(cd.just_available(), false);
        assert_eq!(cd.repeating(), false);
        assert_eq!(cd.percent(), 0.0);
        assert_eq!(cd.percent_left(), 1.0);
        // Tick past the end and make sure elapsed returns to 0.0 and other things update
        cd.unpause();
        cd.tick_f32(500.0);
        assert_eq!(cd.elapsed_f32(), 0.0);
        assert_eq!(cd.available(), true);
        assert_eq!(cd.just_available(), true);
        assert_eq!(cd.percent(), 0.0);
        assert_eq!(cd.percent_left(), 1.0);
        // Continuing to tick when finished should only change just_finished
        cd.tick_f32(1.0);
        assert_eq!(cd.elapsed_f32(), 0.0);
        assert_eq!(cd.available(), true);
        assert_eq!(cd.just_available(), false);
        assert_eq!(cd.percent(), 0.0);
        assert_eq!(cd.percent_left(), 1.0);
    }

    #[test]
    fn repeating_cooldown() {
        let mut cd: Cooldown<()> = Cooldown::from_seconds(2.0, true);
        assert!(cd.available());
        assert!(cd.just_available());
        // Tick once, check all attributes
        cd.tick_f32(0.75);
        assert_eq!(cd.elapsed_f32(), 0.75);
        assert_eq!(cd.duration_f32(), 2.0);
        assert_eq!(cd.available(), false);
        assert_eq!(cd.just_available(), false);
        assert_eq!(cd.repeating(), true);
        assert_eq!(cd.percent(), 0.375);
        assert_eq!(cd.percent_left(), 0.625);
        // Tick past the end and make sure elapsed wraps
        cd.tick_f32(1.5);
        assert_eq!(cd.elapsed_f32(), 0.25);
        assert_eq!(cd.available(), true);
        assert_eq!(cd.just_available(), true);
        assert_eq!(cd.percent(), 0.125);
        assert_eq!(cd.percent_left(), 0.875);
        // Continuing to tick should turn off both available & just_available for repeating timers
        cd.tick_f32(1.0);
        assert_eq!(cd.elapsed_f32(), 1.25);
        assert_eq!(cd.available(), false);
        assert_eq!(cd.just_available(), false);
        assert_eq!(cd.percent(), 0.625);
        assert_eq!(cd.percent_left(), 0.375);
    }
}
