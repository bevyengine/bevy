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
    overstep: Duration,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self {
            size: Self::DEFAULT_STEP_SIZE,
            overstep: Duration::ZERO,
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
    }

    ///
    pub fn expend(&mut self) -> usize {
        let mut steps = 0;
        while self.overstep >= self.size {
            self.overstep -= self.size;
            steps += 1;
        }

        steps
    }
}

/// Runs the [`FixedUpdate`] schedule zero or more times, advancing its clock each time.
pub fn run_fixed_update_schedule(world: &mut World) {
    let _ = world.try_schedule_scope(FixedUpdate, |world, schedule| {
        // get number of steps (done in advance on purpose)
        let mut timestep = world.resource_mut::<FixedTimestep>();
        let steps = timestep.expend();
        let step_size = timestep.size();

        // swap context
        let mut time = world.resource_mut::<Time>();
        assert!(matches!(time.context(), TimeContext::Update));
        time.set_context(TimeContext::FixedUpdate);

        // also apply step size change
        time.fixed_timestep_size = step_size;

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
        assert_eq!(acc.expend(), 0);
        assert_eq!(acc.overstep(), Duration::from_secs(1));

        acc.accumulate(Duration::from_secs(1));
        assert_eq!(acc.expend(), 1);
        assert_eq!(acc.overstep(), Duration::ZERO);
    }

    #[test]
    fn repeatedly_expending_time() {
        let mut acc = FixedTimestep::new(Duration::from_secs(1));
        acc.accumulate(Duration::from_millis(3200));
        assert_eq!(acc.expend(), 3);
        assert_eq!(acc.overstep(), Duration::from_millis(200));
    }
}
