use bevy_ecs::component::Component;
use bevy_utils::Duration;

use crate::{Fixed, Stopwatch, Time, TimeTracker, UpdatingStopwatch, Virtual};

use super::TrackedTime;

/// A stopwatch component that tracks time elapsed when started.
///
/// Note unlike [`Stopwatch`] this stopwatch will automatically advance when attached to an entity.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MixedStopwatch {
    fixed: UpdatingStopwatch<Fixed>,
    virt: UpdatingStopwatch<Virtual>,
    tracked: TrackedTime,
}

impl MixedStopwatch {
    /// Creates a new [`MixedStopwatch`] from the given [`Stopwatch`].
    ///
    /// See [`Stopwatch::new`].
    pub fn new(stopwatch: Stopwatch) -> Self {
        Self {
            fixed: UpdatingStopwatch::new(stopwatch.clone()),
            virt: UpdatingStopwatch::new(stopwatch),
            tracked: TrackedTime::Virtual,
        }
    }

    /// Returns the elapsed time since the last [`reset`](MixedStopwatch::reset)
    /// of the stopwatch.
    pub fn elapsed(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed(),
            TrackedTime::Fixed => self.fixed.elapsed(),
        }
    }

    /// Returns the elapsed time since the last [`reset`](Stopwatch::reset)
    /// of the stopwatch, in seconds.
    pub fn elapsed_secs(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed_secs(),
            TrackedTime::Fixed => self.fixed.elapsed_secs(),
        }
    }

    /// Returns the elapsed time since the last [`reset`](Stopwatch::reset)
    /// of the stopwatch, in seconds, as f64.
    pub fn elapsed_secs_f64(&self) -> f64 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed_secs_f64(),
            TrackedTime::Fixed => self.fixed.elapsed_secs_f64(),
        }
    }

    /// Pauses the stopwatch.
    pub fn pause(&mut self) {
        self.virt.pause();
        self.fixed.pause();
    }

    /// Unpauses the stopwatch.
    pub fn unpase(&mut self) {
        self.virt.unpause();
        self.fixed.unpause();
    }

    /// Returns `true` if the stopwatch is paused.
    pub fn paused(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.paused(),
            TrackedTime::Fixed => self.fixed.paused(),
        }
    }

    /// Resets the stopwatch. The reset doesn't affect the paused state of the stopwatch.
    pub fn reset(&mut self) {
        self.virt.reset();
        self.fixed.reset();
    }

    /// Returns references to the underlying stopwatches.
    pub fn watches(&self) -> (&UpdatingStopwatch<Virtual>, &UpdatingStopwatch<Fixed>) {
        (&self.virt, &self.fixed)
    }

    /// Returns mutable references to the underlying stopwatches.
    ///
    /// When mutating these stopwatches you should take care to match their state.
    /// Otherwise this stopwatch might start behaving erratically.
    pub fn watches_mut(
        &mut self,
    ) -> (
        &mut UpdatingStopwatch<Virtual>,
        &mut UpdatingStopwatch<Fixed>,
    ) {
        (&mut self.virt, &mut self.fixed)
    }
}

impl TimeTracker for MixedStopwatch {
    type Time = (Time<Virtual>, Time<Fixed>);

    fn update(
        &mut self,
        (virt, fixed): &<<Self::Time as crate::context::TimesWithContext>::AsSystemParam<'_> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        match self.tracked {
            TrackedTime::Virtual => self.virt.update(virt),
            TrackedTime::Fixed => self.fixed.update(fixed),
        }
    }

    fn enter_fixed_update(&mut self) {
        self.tracked = TrackedTime::Fixed;
    }

    fn exit_fixed_update(&mut self) {
        self.tracked = TrackedTime::Virtual;
    }
}
