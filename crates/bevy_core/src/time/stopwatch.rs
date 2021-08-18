use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_utils::Duration;

/// A Stopwatch is a struct that track elapsed time when started.
///
/// # Examples
///
/// ```
/// # use bevy_core::*;
/// use std::time::Duration;
/// let mut stopwatch = Stopwatch::new();
/// assert_eq!(stopwatch.elapsed_secs(), 0.0);
///
/// stopwatch.tick(Duration::from_secs_f32(1.0)); // tick one second
/// assert_eq!(stopwatch.elapsed_secs(), 1.0);
///
/// stopwatch.pause();
/// stopwatch.tick(Duration::from_secs_f32(1.0)); // paused stopwatches don't tick
/// assert_eq!(stopwatch.elapsed_secs(), 1.0);
///
/// stopwatch.reset(); // reset the stopwatch
/// assert!(stopwatch.paused());
/// assert_eq!(stopwatch.elapsed_secs(), 0.0);
/// ```
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Stopwatch {
    elapsed: Duration,
    paused: bool,
}

impl Stopwatch {
    /// Create a new unpaused `Stopwatch` with no elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let stopwatch = Stopwatch::new();
    /// assert_eq!(stopwatch.elapsed_secs(), 0.0);
    /// assert_eq!(stopwatch.paused(), false);
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns the elapsed time since the last [`reset`](Stopwatch::reset)
    /// of the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = Stopwatch::new();
    /// stopwatch.tick(Duration::from_secs(1));
    /// assert_eq!(stopwatch.elapsed(), Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[inline]
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed().as_secs_f32()
    }

    /// Sets the elapsed time of the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = Stopwatch::new();
    /// stopwatch.set_elapsed(Duration::from_secs_f32(1.0));
    /// assert_eq!(stopwatch.elapsed_secs(), 1.0);
    /// ```
    #[inline]
    pub fn set_elapsed(&mut self, time: Duration) {
        self.elapsed = time;
    }

    /// Advance the stopwatch by `delta` seconds.
    /// If the stopwatch is paused, ticking will not have any effect
    /// on elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = Stopwatch::new();
    /// stopwatch.tick(Duration::from_secs_f32(1.5));
    /// assert_eq!(stopwatch.elapsed_secs(), 1.5);
    /// ```
    pub fn tick(&mut self, delta: Duration) -> &Self {
        if !self.paused() {
            self.elapsed += delta;
        }
        self
    }

    /// Pauses the stopwatch. Any call to [`tick`](Stopwatch::tick) while
    /// paused will not have any effect on the elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = Stopwatch::new();
    /// stopwatch.pause();
    /// stopwatch.tick(Duration::from_secs_f32(1.5));
    /// assert!(stopwatch.paused());
    /// assert_eq!(stopwatch.elapsed_secs(), 0.0);
    /// ```
    #[inline]
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpauses the stopwatch. Resume the effect of ticking on elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = Stopwatch::new();
    /// stopwatch.pause();
    /// stopwatch.tick(Duration::from_secs_f32(1.0));
    /// stopwatch.unpause();
    /// stopwatch.tick(Duration::from_secs_f32(1.0));
    /// assert!(!stopwatch.paused());
    /// assert_eq!(stopwatch.elapsed_secs(), 1.0);
    /// ```
    #[inline]
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Returns `true` if the stopwatch is paused.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = Stopwatch::new();
    /// assert!(!stopwatch.paused());
    /// stopwatch.pause();
    /// assert!(stopwatch.paused());
    /// stopwatch.unpause();
    /// assert!(!stopwatch.paused());
    /// ```
    #[inline]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Resets the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = Stopwatch::new();
    /// stopwatch.tick(Duration::from_secs_f32(1.5));
    /// stopwatch.reset();
    /// assert_eq!(stopwatch.elapsed_secs(), 0.0);
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.elapsed = Default::default();
    }
}

/// A DiscreteStopwatch is a struct that tracks the number of times it has been incremented when started.
///
/// # Examples
///
/// ```
/// # use bevy_core::*;
/// let mut stopwatch = DiscreteStopwatch::new();
/// assert_eq!(stopwatch.elapsed(), 0);
///
/// stopwatch.tick(1); // tick once
/// assert_eq!(stopwatch.elapsed(), 1);
///
/// stopwatch.pause();
/// stopwatch.tick(1); // paused stopwatches don't tick
/// assert_eq!(stopwatch.elapsed(), 1);
///
/// stopwatch.reset(); // reset the stopwatch
/// assert!(stopwatch.paused());
/// assert_eq!(stopwatch.elapsed(), 0);
/// ```
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DiscreteStopwatch {
    elapsed: u64,
    paused: bool,
}

impl DiscreteStopwatch {
    /// Create a new unpaused `Stopwatch` with no elapsed increments.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let stopwatch = DiscreteStopwatch::new();
    /// assert_eq!(stopwatch.elapsed(), 0);
    /// assert_eq!(stopwatch.paused(), false);
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns the elapsed increments since the last [`reset`](DiscreteStopwatch::reset)
    /// of the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// stopwatch.tick(1);
    /// assert_eq!(stopwatch.elapsed(), 1);
    /// ```
    #[inline]
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }

    /// Sets the elapsed time of the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// stopwatch.set_elapsed(1);
    /// assert_eq!(stopwatch.elapsed(), 1);
    /// ```
    #[inline]
    pub fn set_elapsed(&mut self, time: u64) {
        self.elapsed = time;
    }

    /// Advance the stopwatch by `delta` increments.
    /// If the stopwatch is paused, ticking will not have any effect
    /// on elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// stopwatch.tick(3);
    /// assert_eq!(stopwatch.elapsed(), 3);
    /// ```
    pub fn tick(&mut self, delta: u64) -> &Self {
        if !self.paused() {
            self.elapsed += delta;
        }
        self
    }

    /// Pauses the stopwatch. Any call to [`tick`](Stopwatch::tick) while
    /// paused will not have any effect on the elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// stopwatch.pause();
    /// stopwatch.tick(2);
    /// assert!(stopwatch.paused());
    /// assert_eq!(stopwatch.elapsed(), 0);
    /// ```
    #[inline]
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpauses the stopwatch. Resume the effect of ticking on elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// stopwatch.pause();
    /// stopwatch.tick(1);
    /// stopwatch.unpause();
    /// stopwatch.tick(1);
    /// assert!(!stopwatch.paused());
    /// assert_eq!(stopwatch.elapsed(), 1);
    /// ```
    #[inline]
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Returns `true` if the stopwatch is paused.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// assert!(!stopwatch.paused());
    /// stopwatch.pause();
    /// assert!(stopwatch.paused());
    /// stopwatch.unpause();
    /// assert!(!stopwatch.paused());
    /// ```
    #[inline]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Resets the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let mut stopwatch = DiscreteStopwatch::new();
    /// stopwatch.tick(3);
    /// stopwatch.reset();
    /// assert_eq!(stopwatch.elapsed(), 0);
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.elapsed = 0;
    }
}
