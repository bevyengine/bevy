//! Tools to run systems at a regular interval.
//! This can be extremely useful for steady, frame-rate independent gameplay logic and physics.
//!
//! To run a system on a fixed timestep, add it to the [`FixedUpdate`] [`Schedule`](bevy_ecs::schedule::Schedule).
//! This schedule is run in [`RunFixedUpdateLoop`](bevy_app::RunFixedUpdateLoop) near the start of each frame,
//! via the [`run_fixed_update_schedule`] exclusive system.
//!
//! This schedule will be run a number of times each frame,
//! equal to the accumulated divided by the period resource, rounded down,
//! as tracked in the [`FixedTime`] resource.
//! Unused time will be carried over.
//!
//! This does not guarantee that the time elapsed between executions is exact,
//! and systems in this schedule can run 0, 1 or more times on any given frame.
//!
//! For example, a system with a fixed timestep run criteria of 120 times per second will run
//! two times during a ~16.667ms frame, once during a ~8.333ms frame, and once every two frames
//! with ~4.167ms frames. However, the same criteria may not result in exactly 8.333ms passing
//! between each execution.
//!
//! When using fixed time steps, it is advised not to rely on [`Time::delta`] or any of it's
//! variants for game simulation, but rather use the value of [`FixedTime`] instead.

use crate::Time;
use bevy_app::FixedUpdate;
use bevy_ecs::{system::Resource, world::World};
use bevy_utils::Duration;
use thiserror::Error;

/// The amount of time that must pass before the fixed timestep schedule is run again.
///
/// For more information, see the [module-level documentation](self).
///
/// When using bevy's default configuration, this will be updated using the [`Time`]
/// resource. To customize how `Time` is updated each frame, see [`TimeUpdateStrategy`].
///
/// [`TimeUpdateStrategy`]: crate::TimeUpdateStrategy
#[derive(Resource, Debug)]
pub struct FixedTime {
    accumulated: Duration,
    /// The amount of time spanned by each fixed update.
    /// Defaults to 1/60th of a second.
    ///
    /// To configure this value, simply mutate or overwrite this field.
    pub period: Duration,
}

impl FixedTime {
    /// Creates a new [`FixedTime`] struct with a specified period.
    pub fn new(period: Duration) -> Self {
        FixedTime {
            accumulated: Duration::ZERO,
            period,
        }
    }

    /// Creates a new [`FixedTime`] struct with a period specified in seconds.
    pub fn new_from_secs(period: f32) -> Self {
        FixedTime {
            accumulated: Duration::ZERO,
            period: Duration::from_secs_f32(period),
        }
    }

    /// Adds to this instance's accumulated time. `delta_time` should be the amount of in-game time
    /// that has passed since `tick` was last called.
    ///
    /// Note that if you are using the default configuration of bevy, this will be called for you.
    pub fn tick(&mut self, delta_time: Duration) {
        self.accumulated += delta_time;
    }

    /// Returns the current amount of accumulated time.
    ///
    /// Approximately, this represents how far behind the fixed update schedule is from the main schedule.
    pub fn accumulated(&self) -> Duration {
        self.accumulated
    }

    /// Attempts to advance by a single period. This will return [`FixedUpdateError`] if there is not enough
    /// accumulated time -- in other words, if advancing time would put the fixed update schedule
    /// ahead of the main schedule.
    ///
    /// Note that if you are using the default configuration of bevy, this will be called for you.
    pub fn expend(&mut self) -> Result<(), FixedUpdateError> {
        if let Some(new_value) = self.accumulated.checked_sub(self.period) {
            self.accumulated = new_value;
            Ok(())
        } else {
            Err(FixedUpdateError::NotEnoughTime {
                accumulated: self.accumulated,
                period: self.period,
            })
        }
    }
}

impl Default for FixedTime {
    fn default() -> Self {
        FixedTime {
            accumulated: Duration::ZERO,
            period: Duration::from_secs_f32(1. / 60.),
        }
    }
}

/// An error returned when working with [`FixedTime`].
#[derive(Debug, Error)]
pub enum FixedUpdateError {
    /// There is not enough accumulated time to advance the fixed update schedule.
    #[error("At least one period worth of time must be accumulated.")]
    NotEnoughTime {
        /// The amount of time available to advance the fixed update schedule.
        accumulated: Duration,
        /// The length of one fixed update.
        period: Duration,
    },
}

/// Ticks the [`FixedTime`] resource then runs the [`FixedUpdate`].
///
/// For more information, see the [module-level documentation](self).
pub fn run_fixed_update_schedule(world: &mut World) {
    // Tick the time
    let delta_time = world.resource::<Time>().delta();
    let mut fixed_time = world.resource_mut::<FixedTime>();
    fixed_time.tick(delta_time);

    // Run the schedule until we run out of accumulated time
    let _ = world.try_schedule_scope(FixedUpdate, |world, schedule| {
        while world.resource_mut::<FixedTime>().expend().is_ok() {
            schedule.run(world);
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fixed_time_starts_at_zero() {
        let new_time = FixedTime::new_from_secs(42.);
        assert_eq!(new_time.accumulated(), Duration::ZERO);

        let default_time = FixedTime::default();
        assert_eq!(default_time.accumulated(), Duration::ZERO);
    }

    #[test]
    fn fixed_time_ticks_up() {
        let mut fixed_time = FixedTime::default();
        fixed_time.tick(Duration::from_secs(1));
        assert_eq!(fixed_time.accumulated(), Duration::from_secs(1));
    }

    #[test]
    fn enough_accumulated_time_is_required() {
        let mut fixed_time = FixedTime::new(Duration::from_secs(2));
        fixed_time.tick(Duration::from_secs(1));
        assert!(fixed_time.expend().is_err());
        assert_eq!(fixed_time.accumulated(), Duration::from_secs(1));

        fixed_time.tick(Duration::from_secs(1));
        assert!(fixed_time.expend().is_ok());
        assert_eq!(fixed_time.accumulated(), Duration::ZERO);
    }

    #[test]
    fn repeatedly_expending_time() {
        let mut fixed_time = FixedTime::new(Duration::from_secs(1));
        fixed_time.tick(Duration::from_secs_f32(3.2));
        assert!(fixed_time.expend().is_ok());
        assert!(fixed_time.expend().is_ok());
        assert!(fixed_time.expend().is_ok());
        assert!(fixed_time.expend().is_err());
    }
}
