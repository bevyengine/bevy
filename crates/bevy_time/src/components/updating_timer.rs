use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_utils::Duration;

use crate::{context::Context, Time, TimeTracker, Timer, TimerMode};

/// A version of a [`Timer`] that acts as a component .
///
/// The generic `T` defines what [`Time<T>`](Time) this timer will follow.
///
/// # Fixed update
/// If this timer is set to track [`Time<Fixed>`](Fixed) it will report incorrect information when read outside of [`FixedUpdate`](bevy_app::FixedUpdate).
/// Conversely when not set to track fixed time this timer will report incorrect information when not read in `FixedUpdate`. If you need a timer that works
/// in both contexts use a [`MixedTimer`](super::MixedTimer).
#[derive(Component)]
pub struct UpdatingTimer<T> {
    pub(super) timer: Timer,
    tracking: PhantomData<T>,
}

impl<T> UpdatingTimer<T> {
    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            tracking: PhantomData,
        }
    }

    pub fn finished(&self) -> bool {
        self.timer.finished()
    }

    pub fn just_finished(&self) -> bool {
        self.timer.just_finished()
    }

    pub fn elapsed(&self) -> Duration {
        self.timer.elapsed()
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.timer.elapsed_secs()
    }

    pub fn duration(&self) -> Duration {
        self.timer.duration()
    }

    pub fn mode(&self) -> TimerMode {
        self.timer.mode()
    }

    pub fn set_mode(&mut self, mode: TimerMode) {
        self.timer.set_mode(mode);
    }

    pub fn pause(&mut self) {
        self.timer.pause();
    }

    pub fn unpase(&mut self) {
        self.timer.unpause();
    }

    pub fn paused(&self) -> bool {
        self.timer.paused()
    }

    pub fn reset(&mut self) {
        self.timer.reset();
    }

    pub fn fraction(&self) -> f32 {
        self.timer.fraction()
    }

    pub fn fraction_remaining(&self) -> f32 {
        self.timer.fraction_remaining()
    }

    pub fn remaining_secs(&self) -> f32 {
        self.timer.remaining_secs()
    }

    pub fn remaining(&self) -> Duration {
        self.timer.remaining()
    }

    pub fn times_finished_this_tick(&self) -> u32 {
        self.timer.times_finished_this_tick()
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

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
            tracking: self.tracking.clone(),
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
