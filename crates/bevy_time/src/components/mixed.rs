use bevy_ecs::component::Component;
use bevy_utils::Duration;

use crate::{
    Fixed, Stopwatch, TimeTracker, Timer, TimerMode, UpdatingStopwatch, UpdatingTimer, Virtual,
};

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
enum TrackedTime {
    #[default]
    Virtual,
    Fixed,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MixedTimer {
    fixed: UpdatingTimer<Fixed>,
    virt: UpdatingTimer<Virtual>,
    tracked: TrackedTime,
}

impl MixedTimer {
    pub fn new(timer: Timer) -> Self {
        Self {
            fixed: UpdatingTimer::new(timer.clone()),
            virt: UpdatingTimer::new(timer),
            tracked: TrackedTime::Virtual,
        }
    }

    pub fn finished(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.finished(),
            TrackedTime::Fixed => self.fixed.finished(),
        }
    }

    pub fn just_finished(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.just_finished(),
            TrackedTime::Fixed => self.fixed.just_finished(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed(),
            TrackedTime::Fixed => self.fixed.elapsed(),
        }
    }

    pub fn elapsed_secs(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed_secs(),
            TrackedTime::Fixed => self.fixed.elapsed_secs(),
        }
    }

    pub fn duration(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.duration(),
            TrackedTime::Fixed => self.fixed.duration(),
        }
    }

    pub fn mode(&self) -> TimerMode {
        match self.tracked {
            TrackedTime::Virtual => self.virt.mode(),
            TrackedTime::Fixed => self.fixed.mode(),
        }
    }

    pub fn set_mode(&mut self, mode: TimerMode) {
        self.virt.set_mode(mode);
        self.fixed.set_mode(mode);
    }

    pub fn pause(&mut self) {
        self.virt.pause();
        self.fixed.pause();
    }

    pub fn unpase(&mut self) {
        self.virt.unpase();
        self.fixed.unpase();
    }

    pub fn paused(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.paused(),
            TrackedTime::Fixed => self.fixed.paused(),
        }
    }

    pub fn reset(&mut self) {
        self.virt.reset();
        self.fixed.reset();
    }

    pub fn fraction(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.fraction(),
            TrackedTime::Fixed => self.fixed.fraction(),
        }
    }

    pub fn fraction_remaining(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.fraction_remaining(),
            TrackedTime::Fixed => self.fixed.fraction_remaining(),
        }
    }

    pub fn remaining_secs(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.remaining_secs(),
            TrackedTime::Fixed => self.fixed.remaining_secs(),
        }
    }

    pub fn remaining(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.remaining(),
            TrackedTime::Fixed => self.fixed.remaining(),
        }
    }

    pub fn times_finished_this_tick(&self) -> u32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.times_finished_this_tick(),
            TrackedTime::Fixed => self.fixed.times_finished_this_tick(),
        }
    }

    pub fn timers(&self) -> (&UpdatingTimer<Virtual>, &UpdatingTimer<Fixed>) {
        (&self.virt, &self.fixed)
    }

    pub fn timer_mut(&mut self) -> (&mut UpdatingTimer<Virtual>, &mut UpdatingTimer<Fixed>) {
        (&mut self.virt, &mut self.fixed)
    }
}

impl TimeTracker for MixedTimer {
    const DOES_UPDATE: bool = true;

    const DOES_FIXED_UPDATE: bool = true;

    type UpdateSource<'w> = <UpdatingTimer<Virtual> as TimeTracker>::UpdateSource<'w>;

    type FixedUpdateSource<'w> = <UpdatingTimer<Fixed> as TimeTracker>::UpdateSource<'w>;

    fn update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::UpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.virt.update(time);
        self.tracked = TrackedTime::Virtual;
    }

    fn fixed_update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::FixedUpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.fixed.update(time);
        self.tracked = TrackedTime::Fixed;
    }
}

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MixedStopwatch {
    fixed: UpdatingStopwatch<Fixed>,
    virt: UpdatingStopwatch<Virtual>,
    tracked: TrackedTime,
}

impl MixedStopwatch {
    pub fn new(stopwatch: Stopwatch) -> Self {
        Self {
            fixed: UpdatingStopwatch::new(stopwatch.clone()),
            virt: UpdatingStopwatch::new(stopwatch),
            tracked: TrackedTime::Virtual,
        }
    }

    pub fn elapsed(&self) -> Duration {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed(),
            TrackedTime::Fixed => self.fixed.elapsed(),
        }
    }

    pub fn elapsed_secs(&self) -> f32 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed_secs(),
            TrackedTime::Fixed => self.fixed.elapsed_secs(),
        }
    }

    pub fn elapsed_secs_f64(&self) -> f64 {
        match self.tracked {
            TrackedTime::Virtual => self.virt.elapsed_secs_f64(),
            TrackedTime::Fixed => self.fixed.elapsed_secs_f64(),
        }
    }

    pub fn pause(&mut self) {
        self.virt.pause();
        self.fixed.pause();
    }

    pub fn unpase(&mut self) {
        self.virt.unpause();
        self.fixed.unpause();
    }

    pub fn paused(&self) -> bool {
        match self.tracked {
            TrackedTime::Virtual => self.virt.paused(),
            TrackedTime::Fixed => self.fixed.paused(),
        }
    }

    pub fn reset(&mut self) {
        self.virt.reset();
        self.fixed.reset();
    }

    pub fn watches(&self) -> (&UpdatingStopwatch<Virtual>, &UpdatingStopwatch<Fixed>) {
        (&self.virt, &self.fixed)
    }

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
    const DOES_UPDATE: bool = true;

    const DOES_FIXED_UPDATE: bool = true;

    type UpdateSource<'w> = <UpdatingStopwatch<Virtual> as TimeTracker>::UpdateSource<'w>;

    type FixedUpdateSource<'w> = <UpdatingStopwatch<Fixed> as TimeTracker>::FixedUpdateSource<'w>;

    fn update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::UpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.virt.update(time);
        self.tracked = TrackedTime::Virtual;
    }

    fn fixed_update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::FixedUpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.fixed.fixed_update(time);
        self.tracked = TrackedTime::Fixed;
    }
}
