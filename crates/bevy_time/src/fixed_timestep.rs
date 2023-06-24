//! A tool for implementing gameplay logic in way that results in consistent behaviour,
//! independent of framerate.
//!
//! To run systems on a fixed timestep, add them to the [`FixedUpdate`] schedule.
//! This schedule will be run during [`RunFixedUpdateLoop`](bevy_app::RunFixedUpdateLoop), by the
//! exclusive system [`run_fixed_update_schedule`].
//!
//! [`FixedUpdate`] can run zero or more times each frame. Each [timestep](FixedTimestep::size)
//! time that elapses in [`Time`] will queue another step to run.
//!
//! However, no guarantees about the real time that elapses between these runs can be made.

use bevy_app::FixedUpdate;
use bevy_ecs::{system::Resource, world::World};
use bevy_utils::{default, Duration};

use crate::{Time, TimeContext};

/// The step size (and some metadata) for the `FixedUpdate` schedule.
#[derive(Resource, Debug, Clone, Copy)]
pub struct FixedTimestep {
    size: Duration,
    steps: u32,
    overstep: Duration,
    max_steps_per_update: u32,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self {
            size: Self::DEFAULT_STEP_SIZE,
            steps: 0,
            overstep: Duration::ZERO,
            max_steps_per_update: u32::MAX,
        }
    }
}

impl FixedTimestep {
    /// The default step size.
    pub const DEFAULT_STEP_SIZE: Duration = Duration::from_micros(15625);

    /// Constructs a new `FixedTimestep` from a [`Duration`].
    pub fn new(size: Duration) -> Self {
        assert!(!size.is_zero(), "timestep is zero");
        Self { size, ..default() }
    }

    /// Constructs a new `FixedTimestep` from an [`f64`] number of seconds.
    pub fn from_secs(seconds: f64) -> Self {
        Self::new(Duration::from_secs_f64(seconds))
    }

    /// Constructs a new `FixedTimestep` from a nominal [`f64`] number of steps per second.
    pub fn from_hz(hz: f64) -> Self {
        assert!(hz.is_sign_positive(), "Hz less than or equal to zero");
        assert!(hz.is_finite(), "Hz is infinite");
        Self::from_secs(1.0 / hz)
    }

    /// Returns the step size as a [`Duration`].
    #[inline]
    pub fn size(&self) -> Duration {
        self.size
    }

    /// Sets the step size, given as a [`Duration`].
    ///
    /// # Panics
    ///
    /// Panics if `step_size` is a zero-length duration.
    pub fn set_size(&mut self, size: Duration) {
        assert!(!size.is_zero(), "timestep is zero");
        self.size = size;
    }

    /// Returns the number of steps accumulated.
    #[inline]
    pub fn steps(&self) -> u32 {
        self.steps
    }

    /// Returns the amount of time accumulated toward new steps, as a [`Duration`].
    #[inline]
    pub fn overstep(&self) -> Duration {
        self.overstep
    }

    /// Returns the amount of time accumulated toward new steps,
    /// as an [`f32`] fraction of the timestep.
    #[inline]
    pub fn overstep_percentage(&self) -> f32 {
        self.overstep.as_secs_f32() / self.size.as_secs_f32()
    }

    /// Returns the amount of time accumulated toward new steps,
    /// as an [`f64`] fraction of the timestep.
    pub fn overstep_percentage_f64(&self) -> f64 {
        self.overstep.as_secs_f64() / self.size.as_secs_f64()
    }

    /// Adds `time` to an internal accumulator.
    pub fn accumulate(&mut self, time: Duration) {
        self.overstep += time;
        while self.overstep >= self.size {
            self.overstep -= self.size;
            self.steps += 1;
        }
    }

    /// Consumes one step and returns the number remaining.
    /// Returns `None` if there was no step that could be expended.
    pub fn expend(&mut self) -> Option<u32> {
        let remaining = self.steps.checked_sub(1);
        self.steps = self.steps.saturating_sub(1);
        remaining
    }

    /// Returns the maximum number of `FixedUpdate` steps that can be run in one update.
    pub fn max_steps_per_update(&self) -> u32 {
        self.max_steps_per_update
    }

    /// Sets the maximum number of `FixedUpdate` steps that can be run in one update.
    pub fn set_max_steps_per_update(&mut self, steps: u32) {
        self.max_steps_per_update = steps;
    }
}

/// Runs the [`FixedUpdate`] schedule zero or more times, advancing its clock each time.
pub fn run_fixed_update_schedule(world: &mut World) {
    let _ = world.try_schedule_scope(FixedUpdate, |world, schedule| {
        // swap context
        let mut time = world.resource_mut::<Time>();
        time.set_context(TimeContext::FixedUpdate);

        // solve for number of steps (done in advance on purpose)
        let mut timestep = world.resource_mut::<FixedTimestep>();
        let mut steps = 0;
        while timestep.expend().is_some() && steps < timestep.max_steps_per_update {
            steps += 1;
        }

        // run schedule however many times
        for _ in 0..steps {
            let mut time = world.resource_mut::<Time>();
            assert!(matches!(time.context(), TimeContext::FixedUpdate));
            time.update();
            schedule.run(world);
        }

        // swap context
        let mut time = world.resource_mut::<Time>();
        time.set_context(TimeContext::Update);
    });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fixed_time_starts_at_zero() {
        let acc = FixedTimestep::from_secs(42.0);
        assert_eq!(acc.overstep(), Duration::ZERO);

        let acc = FixedTimestep::default();
        assert_eq!(acc.overstep(), Duration::ZERO);
    }

    #[test]
    fn fixed_time_ticks_up() {
        let mut acc = FixedTimestep::new(Duration::from_secs(2));
        acc.accumulate(Duration::from_secs(1));
        assert_eq!(acc.overstep(), Duration::from_secs(1));
    }

    #[test]
    fn enough_accumulated_time_is_required() {
        let mut acc = FixedTimestep::new(Duration::from_secs(2));
        acc.accumulate(Duration::from_secs(1));
        assert!(acc.expend().is_none());
        assert_eq!(acc.overstep(), Duration::from_secs(1));

        acc.accumulate(Duration::from_secs(1));
        assert_eq!(acc.overstep(), Duration::ZERO);
        assert_eq!(acc.steps(), 1);
        assert!(acc.expend().is_some());
    }

    #[test]
    fn repeatedly_expending_time() {
        let mut acc = FixedTimestep::new(Duration::from_secs(1));
        acc.accumulate(Duration::from_secs_f32(3.2));
        assert!(acc.expend().is_some());
        assert!(acc.expend().is_some());
        assert!(acc.expend().is_some());
        assert!(acc.expend().is_none());
    }
}
