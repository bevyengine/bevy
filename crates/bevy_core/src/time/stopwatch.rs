use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::Duration;

/// A Stopwatch is a struct that track elapsed time when started.
/// It requires a type `T` in order to be specialized for your systems and components.
/// This specialization `T` does not take any additional space in the struct.
///
/// # Examples
///
/// ```
/// # use bevy_core::*;
/// use std::time::Duration;
/// let mut stopwatch = Stopwatch::<()>::new();
/// assert_eq!(stopwatch.elapsed_f32(), 0.0);
/// stopwatch.tick_f32(1.0); // tick one second
/// assert_eq!(stopwatch.elapsed_f32(), 1.0);
/// stopwatch.pause();
/// stopwatch.tick_f32(1.0); // paused stopwatches don't tick
/// assert_eq!(stopwatch.elapsed_f32(), 1.0);
/// stopwatch.reset(); // reset the stopwatch
/// assert!(stopwatch.paused());
/// assert_eq!(stopwatch.elapsed_f32(), 0.0);
/// ```
#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Stopwatch<T: Send + Sync + 'static> {
    elapsed: Duration,
    paused: bool,
    marker: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Stopwatch<T> {
    /// Create a new unpaused `Stopwatch` with no elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// let stopwatch: Stopwatch<()> = Stopwatch::new();
    /// assert_eq!(stopwatch.elapsed_f32(), 0.0);
    /// assert_eq!(stopwatch.paused(), false);
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns the elapsed time since the last [`reset`](Stopwatch<T>::reset)
    /// of the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
    /// stopwatch.tick(Duration::from_secs(1));
    /// assert_eq!(stopwatch.elapsed(), Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[inline]
    pub fn elapsed_f32(&self) -> f32 {
        self.elapsed().as_secs_f32()
    }

    /// Sets the elapsed time of the stopwatch.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
    /// stopwatch.set(Duration::from_secs_f32(1.0));
    /// assert_eq!(stopwatch.elapsed_f32(), 1.0);
    /// ```
    #[inline]
    pub fn set(&mut self, time: Duration) {
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
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
    /// stopwatch.tick_f32(1.5);
    /// assert_eq!(stopwatch.elapsed_f32(), 1.5);
    /// ```
    pub fn tick(&mut self, delta: Duration) -> &Self {
        if !self.paused() {
            self.elapsed += delta;
        }
        self
    }

    pub fn tick_f32(&mut self, delta: f32) -> &Self {
        self.tick(Duration::from_secs_f32(delta))
    }

    /// Pauses the stopwatch. Any call to [`tick`](Stopwatch<T>::tick) while
    /// paused will not have any effect on the elapsed time.
    ///
    /// # Examples
    /// ```
    /// # use bevy_core::*;
    /// use std::time::Duration;
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
    /// stopwatch.pause();
    /// stopwatch.tick_f32(1.5);
    /// assert!(stopwatch.paused());
    /// assert_eq!(stopwatch.elapsed_f32(), 0.0);
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
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
    /// stopwatch.pause();
    /// stopwatch.tick_f32(1.0);
    /// stopwatch.unpause();
    /// stopwatch.tick_f32(1.0);
    /// assert!(!stopwatch.paused());
    /// assert_eq!(stopwatch.elapsed_f32(), 1.0);
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
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
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
    /// let mut stopwatch: Stopwatch<()> = Stopwatch::new();
    /// stopwatch.tick_f32(1.5);
    /// stopwatch.reset();
    /// assert_eq!(stopwatch.elapsed_f32(), 0.0);
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.elapsed = Default::default();
    }
}

impl<T: Send + Sync + 'static> Default for Stopwatch<T> {
    fn default() -> Self {
        Self {
            elapsed: Default::default(),
            paused: Default::default(),
            marker: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specialization_costs_zero() {
        use std::mem::size_of;
        struct Small(i32);
        struct Big<'a>(&'a [i32; 64]);

        assert_eq!(size_of::<Stopwatch<Small>>(), size_of::<Stopwatch<Big>>());
        assert_eq!(size_of::<Stopwatch<()>>(), size_of::<Stopwatch<Small>>());
    }
}
