use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_utils::Duration;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

use crate::{context::Context, Stopwatch, Time, TimeTracker};

/// A version of a [`Stopwatch`] that acts as a component.
///
/// The generic `T` defines what [`Time<T>`](Time) this stopwatch will follow.
///
/// # Fixed update
/// If this stopwatch is set to track [`Time<Fixed>`](crate::Fixed) it will report incorrect information when read outside of [`FixedUpdate`](bevy_app::FixedUpdate).
/// Conversely when not set to track fixed time this stopwatch will report incorrect information when not read in `FixedUpdate`. If you need a stopwatch that works
/// in both contexts use a [`MixedStopwatch`](super::MixedStopwatch).
#[derive(Component)]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
pub struct UpdatingStopwatch<T> {
    watch: Stopwatch,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    tracking: PhantomData<T>,
}

impl<T> UpdatingStopwatch<T> {
    /// Creates a new [`UpdatingStopwatch`] from the given [`Stopwatch`].
    ///
    /// See [`Stopwatch::new`].
    pub fn new(watch: Stopwatch) -> Self {
        Self {
            watch,
            tracking: PhantomData,
        }
    }

    /// Returns the elapsed time since the last [`reset`](UpdatingStopwatch::reset)
    /// of the stopwatch.
    pub fn elapsed(&self) -> Duration {
        self.watch.elapsed()
    }

    /// Returns the elapsed time since the last [`reset`](Stopwatch::reset)
    /// of the stopwatch, in seconds.
    pub fn elapsed_secs(&self) -> f32 {
        self.watch.elapsed_secs()
    }

    /// Returns the elapsed time since the last [`reset`](Stopwatch::reset)
    /// of the stopwatch, in seconds, as f64.
    pub fn elapsed_secs_f64(&self) -> f64 {
        self.watch.elapsed_secs_f64()
    }

    /// Pauses the stopwatch.
    pub fn pause(&mut self) {
        self.watch.pause();
    }

    /// Unpauses the stopwatch.
    pub fn unpause(&mut self) {
        self.watch.unpause();
    }

    /// Returns `true` if the stopwatch is paused.
    pub fn paused(&self) -> bool {
        self.watch.paused()
    }

    /// Resets the stopwatch. The reset doesn't affect the paused state of the stopwatch.
    pub fn reset(&mut self) {
        self.watch.reset();
    }
}

impl<C: Context + Default + Send + Sync + 'static> TimeTracker for UpdatingStopwatch<C> {
    type Time = Time<C>;

    fn update(
        &mut self,
        time: &<<Self::Time as crate::context::TimesWithContext>::AsSystemParam<'_> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.watch.tick(time.delta());
    }
}

impl<T> Clone for UpdatingStopwatch<T> {
    fn clone(&self) -> Self {
        Self {
            watch: self.watch.clone(),
            tracking: self.tracking,
        }
    }
}

impl<T> Debug for UpdatingStopwatch<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdatingStopWatch")
            .field("watch", &self.watch)
            .field("tracking", &self.tracking)
            .finish()
    }
}

impl<T> Default for UpdatingStopwatch<T> {
    fn default() -> Self {
        Self {
            watch: Default::default(),
            tracking: Default::default(),
        }
    }
}

impl<T> PartialEq for UpdatingStopwatch<T> {
    fn eq(&self, other: &Self) -> bool {
        self.watch == other.watch && self.tracking == other.tracking
    }
}

impl<T> Eq for UpdatingStopwatch<T> {}

impl<C> From<Stopwatch> for UpdatingStopwatch<C> {
    fn from(value: Stopwatch) -> Self {
        Self::new(value)
    }
}
