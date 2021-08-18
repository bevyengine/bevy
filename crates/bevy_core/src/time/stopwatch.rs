use std::ops::Add;

use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_utils::Duration;

/// The `Stopwatch` trait enables counting-up behavior to track the passage of time.
pub trait Stopwatch: Default {
    /// The unit by which elapsed time is measured
    type TimeUnit: Default + Add<Output = Self::TimeUnit>;

    /// Create a new unpaused `Stopwatch` object with no elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let stopwatch = Stopwatch::new();
    /// assert_eq!(stopwatch.elapsed_secs(), 0.0);
    /// assert_eq!(stopwatch.paused(), false);
    /// ```
    fn new() -> Self {
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
    fn elapsed(&self) -> Self::TimeUnit;

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
    fn set_elapsed(&mut self, time: Self::TimeUnit);

    /// Advance the stopwatch by `delta` units.
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
    fn tick(&mut self, delta: Self::TimeUnit) -> &Self {
        if !self.paused() {
            self.set_elapsed(self.elapsed() + delta);
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
    fn pause(&mut self);

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
    fn unpause(&mut self);

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
    fn paused(&self) -> bool;

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
    fn reset(&mut self) {
        self.set_elapsed(Self::TimeUnit::default());
    }
}

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
pub struct DurationStopwatch {
    elapsed: Duration,
    paused: bool,
}

impl Stopwatch for DurationStopwatch {
    type TimeUnit = Duration;

    #[inline]
    fn elapsed(&self) -> Self::TimeUnit {
        self.elapsed
    }

    #[inline]
    fn set_elapsed(&mut self, time: Self::TimeUnit) {
        self.elapsed = time;
    }

    #[inline]
    fn pause(&mut self) {
        self.paused = true;
    }

    #[inline]
    fn unpause(&mut self) {
        self.paused = false;
    }

    #[inline]
    fn paused(&self) -> bool {
        self.paused
    }
}

impl DurationStopwatch {
    #[inline]
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed().as_secs_f32()
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

impl Stopwatch for DiscreteStopwatch {
    type TimeUnit = u64;

    #[inline]
    fn elapsed(&self) -> Self::TimeUnit {
        self.elapsed
    }

    #[inline]
    fn set_elapsed(&mut self, time: Self::TimeUnit) {
        self.elapsed = time;
    }

    #[inline]
    fn pause(&mut self) {
        self.paused = true;
    }

    #[inline]
    fn unpause(&mut self) {
        self.paused = false;
    }

    #[inline]
    fn paused(&self) -> bool {
        self.paused
    }
}
