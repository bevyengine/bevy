use bevy_ecs::component::Component;
use bevy_utils::Duration;

use crate::{
    Fixed, Stopwatch, Time, TimeTracker, Timer, TimerMode, UpdatingStopwatch, UpdatingTimer,
    Virtual,
};

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
enum TrackedTime {
    #[default]
    Virtual,
    Fixed,
}

/// Tracks elapsed time. Enters the finished state once `duration` is reached.
///
/// Non repeating timers will stop tracking and stay in the finished state until reset.
/// Repeating timers will only be in the finished state on each tick `duration` is reached or
/// exceeded, and can still be reset at any given point.
///
/// Paused timers will not have elapsed time increased.
///
/// Note that unlike [`Timer`] this timer will advanced automatically once attached to an entity.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MixedTimer {
    fixed: UpdatingTimer<Fixed>,
    virt: UpdatingTimer<Virtual>,
    tracked: TrackedTime,
}

impl MixedTimer {
    /// Creates a new [`MixedTimer`] from the given [`Timer`].
    ///
    /// See [`Timer::new`].
    pub fn new(timer: Timer) -> Self {
        Self {
            fixed: UpdatingTimer::new(timer.clone()),
            virt: UpdatingTimer::new(timer),
            tracked: TrackedTime::Virtual,
        }
    }

    /// Returns `true` if the timer has reached its duration.
    ///
    /// For repeating timers, this method behaves identically to [`MixedTimer::just_finished`].
    pub fn finished(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.finished(),
            TrackedTime::Fixed => self.fixed.finished(),
        }
    }

    /// Returns `true` only on the tick the timer reached its duration.
    ///
    /// # Fixed update
    ///
    /// This method may behave surprisingly when read both from [`FixedUpdate`](bevy_app::FixedUpdate) and [`Update`](bevy_app::Update).
    /// When 2 fixed updates in the same game tick this method may return `true` in the first one then return `false` in the second one
    /// and finally return `true` in `Update`.
    pub fn just_finished(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.just_finished(),
            TrackedTime::Fixed => self.fixed.just_finished(),
        }
    }

    /// Returns the time elapsed on the timer. Guaranteed to be between 0.0 and `duration`.
    /// Will only equal `duration` when the timer is finished and non repeating.
    pub fn elapsed(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed(),
            TrackedTime::Fixed => self.fixed.elapsed(),
        }
    }

    /// Returns the time elapsed on the timer as an `f32`.
    /// See also [`MixedTimer::elapsed`].
    pub fn elapsed_secs(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed_secs(),
            TrackedTime::Fixed => self.fixed.elapsed_secs(),
        }
    }

    /// Returns the duration of the timer.
    pub fn duration(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.duration(),
            TrackedTime::Fixed => self.fixed.duration(),
        }
    }

    /// Sets the duration of the timer.
    pub fn set_duration(&mut self, duration: Duration) {
        self.virt.set_duration(duration);
        self.fixed.set_duration(duration);
    }

    /// Returns the mode of the timer.
    pub fn mode(&self) -> TimerMode {
        match self.tracked {
            TrackedTime::Virtual => self.virt.mode(),
            TrackedTime::Fixed => self.fixed.mode(),
        }
    }

    /// Sets the mode of the timer.
    pub fn set_mode(&mut self, mode: TimerMode) {
        self.virt.set_mode(mode);
        self.fixed.set_mode(mode);
    }

    /// Pauses the timer. Disables the ticking of the timer.
    pub fn pause(&mut self) {
        self.virt.pause();
        self.fixed.pause();
    }

    /// Unpauses the timer. Resumes the ticking of the timer.
    pub fn unpause(&mut self) {
        self.virt.unpause();
        self.fixed.unpause();
    }

    /// Returns `true` if the timer is paused.
    pub fn paused(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.paused(),
            TrackedTime::Fixed => self.fixed.paused(),
        }
    }

    /// Resets the timer. The reset doesn't affect the `paused` state of the timer.
    pub fn reset(&mut self) {
        self.virt.reset();
        self.fixed.reset();
    }

    /// Returns the fraction of the timer elapsed time (goes from 0.0 to 1.0).
    pub fn fraction(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.fraction(),
            TrackedTime::Fixed => self.fixed.fraction(),
        }
    }

    /// Returns the fraction of the timer remaining time (goes from 1.0 to 0.0).
    pub fn fraction_remaining(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.fraction_remaining(),
            TrackedTime::Fixed => self.fixed.fraction_remaining(),
        }
    }

    /// Returns the remaining time in seconds.
    pub fn remaining_secs(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.remaining_secs(),
            TrackedTime::Fixed => self.fixed.remaining_secs(),
        }
    }

    /// Returns the remaining time using Duration.
    pub fn remaining(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.remaining(),
            TrackedTime::Fixed => self.fixed.remaining(),
        }
    }

    /// Returns the number of times a repeating timer
    /// finished during the last [`update`](TimeTracker::update).
    ///
    /// For non repeating-timers, this method will only ever
    /// return 0 or 1.
    pub fn times_finished_this_tick(&self) -> u32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.times_finished_this_tick(),
            TrackedTime::Fixed => self.fixed.times_finished_this_tick(),
        }
    }

    /// Returns references to the underlying timers.
    pub fn timers(&self) -> (&UpdatingTimer<Virtual>, &UpdatingTimer<Fixed>) {
        (&self.virt, &self.fixed)
    }

    /// Returns mutable references to the underlying timers.
    ///
    /// When mutating these timers you should take care to match their state.
    /// Otherwise this timer might start behaving erratically.
    pub fn timer_mut(&mut self) -> (&mut UpdatingTimer<Virtual>, &mut UpdatingTimer<Fixed>) {
        (&mut self.virt, &mut self.fixed)
    }
}

impl TimeTracker for MixedTimer {
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
