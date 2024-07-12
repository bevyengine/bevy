use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_utils::Duration;

use crate::{context::Context, Time, TimeTracker, Timer, TimerMode};

/// A version of a [`Timer`] that acts as a component.
///
/// The generic `T` defines what [`Time<T>`](Time) this timer will follow.
///
/// # Fixed update
/// If this timer is set to track [`Time<Fixed>`](crate::Fixed) it will report incorrect information when read outside of [`FixedUpdate`](bevy_app::FixedUpdate).
/// Conversely when not set to track fixed time this timer will report incorrect information when not read in `FixedUpdate`. If you need a timer that works
/// in both contexts use a [`MixedTimer`](super::MixedTimer).
#[derive(Component)]
pub struct UpdatingTimer<T> {
    timer: Timer,
    tracking: PhantomData<T>,
}

impl<T> UpdatingTimer<T> {
    /// Creates a new [`UpdatingTimer`] from the given [`Timer`].
    ///
    /// See [`Timer::new`].
    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            tracking: PhantomData,
        }
    }

    /// Returns `true` if the timer has reached its duration.
    ///
    /// For repeating timers, this method behaves identically to [`UpdatingTimer::just_finished`].
    pub fn finished(&self) -> bool {
        self.timer.finished()
    }

    /// Returns `true` only on the tick the timer reached its duration.
    pub fn just_finished(&self) -> bool {
        self.timer.just_finished()
    }

    /// Returns the time elapsed on the timer. Guaranteed to be between 0.0 and `duration`.
    /// Will only equal `duration` when the timer is finished and non repeating.
    pub fn elapsed(&self) -> Duration {
        self.timer.elapsed()
    }

    /// Returns the time elapsed on the timer as an `f32`.
    /// See also [`UpdatingTimer::elapsed`].
    pub fn elapsed_secs(&self) -> f32 {
        self.timer.elapsed_secs()
    }

    /// Returns the duration of the timer.
    pub fn duration(&self) -> Duration {
        self.timer.duration()
    }

    /// Sets the duration of the timer.
    pub fn set_duration(&mut self, duration: Duration) {
        self.timer.set_duration(duration);
    }

    /// Returns the mode of the timer.
    pub fn mode(&self) -> TimerMode {
        self.timer.mode()
    }

    /// Sets the mode of the timer.
    pub fn set_mode(&mut self, mode: TimerMode) {
        self.timer.set_mode(mode);
    }

    /// Pauses the timer. Disables the ticking of the timer.
    pub fn pause(&mut self) {
        self.timer.pause();
    }

    /// Unpauses the timer. Resumes the ticking of the timer.
    pub fn unpause(&mut self) {
        self.timer.unpause();
    }

    /// Returns `true` if the timer is paused.
    pub fn paused(&self) -> bool {
        self.timer.paused()
    }

    /// Resets the timer. The reset doesn't affect the `paused` state of the timer.
    pub fn reset(&mut self) {
        self.timer.reset();
    }

    /// Returns the fraction of the timer elapsed time (goes from 0.0 to 1.0).
    pub fn fraction(&self) -> f32 {
        self.timer.fraction()
    }

    /// Returns the fraction of the timer remaining time (goes from 1.0 to 0.0).
    pub fn fraction_remaining(&self) -> f32 {
        self.timer.fraction_remaining()
    }

    /// Returns the remaining time in seconds.
    pub fn remaining_secs(&self) -> f32 {
        self.timer.remaining_secs()
    }

    /// Returns the remaining time using Duration.
    pub fn remaining(&self) -> Duration {
        self.timer.remaining()
    }

    /// Returns the number of times a repeating timer
    /// finished during the last [`update`](TimeTracker::update).
    ///
    /// For non repeating-timers, this method will only ever
    /// return 0 or 1.
    pub fn times_finished_this_tick(&self) -> u32 {
        self.timer.times_finished_this_tick()
    }

    /// Returns a references to the underlying [`Timer`].
    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    /// Returns a mutable references to the underlying [`Timer`].
    pub fn timer_mut(&mut self) -> &mut Timer {
        &mut self.timer
    }
}

impl<C: Context + Default + Send + Sync + 'static> TimeTracker for UpdatingTimer<C> {
    type Time = Time<C>;

    fn update(
        &mut self,
        time: &<<Self::Time as crate::context::TimesWithContext>::AsSystemParam<'_> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.timer.tick(time.delta());
    }
}

impl<T> Clone for UpdatingTimer<T> {
    fn clone(&self) -> Self {
        Self {
            timer: self.timer.clone(),
            tracking: self.tracking,
        }
    }
}

impl<T> Debug for UpdatingTimer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdatingTimer")
            .field("timer", &self.timer)
            .field("tracking", &self.tracking)
            .finish()
    }
}

impl<T> Default for UpdatingTimer<T> {
    fn default() -> Self {
        Self {
            timer: Default::default(),
            tracking: Default::default(),
        }
    }
}

impl<T> PartialEq for UpdatingTimer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.timer == other.timer
    }
}

impl<T> Eq for UpdatingTimer<T> {}
