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
use bevy_utils::{tracing::warn, Duration};
use thiserror::Error;

/// The amount of time that must pass before the fixed timestep schedule is run again.
#[derive(Resource, Debug)]
pub struct FixedTime {
    accumulated: Duration,
    /// Defaults to 1/60th of a second.
    /// To configure this value, simply mutate or overwrite this resource.
    pub period: Duration,
    /// The maximum number of times that the [`FixedUpdate`] schedule will be allowed to run per main schedule pass.
    /// Defaults to 3.
    ///
    /// If this value is set to `None`, the schedule will run as many times as possible.
    /// Be careful when setting this value to `None`, as it can cause the app to stop rendering
    /// as the fixed update schedule will run an increasing number of times per frame as it falls behind.
    pub max_ticks_per_frame: Option<u8>,
}

impl FixedTime {
    /// Creates a new [`FixedTime`] struct
    pub fn new(period: Duration) -> Self {
        FixedTime {
            accumulated: Duration::ZERO,
            period,
            max_ticks_per_frame: Some(3),
        }
    }

    /// Creates a new [`FixedTime`] struct with a period specified in `f32` seconds
    pub fn new_from_secs(period: f32) -> Self {
        FixedTime {
            accumulated: Duration::ZERO,
            period: Duration::from_secs_f32(period),
            max_ticks_per_frame: Some(3),
        }
    }

    /// Adds the `delta_time` to the accumulated time so far.
    pub fn tick(&mut self, delta_time: Duration) {
        self.accumulated += delta_time;
    }

    /// Returns the current amount of accumulated time
    pub fn accumulated(&self) -> Duration {
        self.accumulated
    }

    /// Expends one `period` of accumulated time.
    ///
    /// [`Err(FixedUpdateError`)] will be returned if there is
    /// not enough accumulated time to span an entire period.
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
        FixedTime::new_from_secs(1. / 60.)
    }
}

/// An error returned when working with [`FixedTime`].
#[derive(Debug, Error)]
pub enum FixedUpdateError {
    #[error("At least one period worth of time must be accumulated.")]
    NotEnoughTime {
        accumulated: Duration,
        period: Duration,
    },
}

/// Ticks the [`FixedTime`] resource then runs the [`FixedUpdate`].
pub fn run_fixed_update_schedule(world: &mut World) {
    // Tick the time
    let delta_time = world.resource::<Time>().delta();
    let mut fixed_time = world.resource_mut::<FixedTime>();
    fixed_time.tick(delta_time);

    let mut ticks_this_frame = 0;

    // Copy this out to appease the borrow checker
    let maybe_max_ticks = fixed_time.max_ticks_per_frame;

    // Run the schedule until we run out of accumulated time
    let _ = world.try_schedule_scope(FixedUpdate, |world, schedule| {
        while world.resource_mut::<FixedTime>().expend().is_ok() {
            ticks_this_frame +=  1;

            // This is required to avoid entering a death spiral where the fixed update schedule falls behind
            // and needs to run more and more times per frame to catch up.
            if let Some(max_ticks) = maybe_max_ticks {
                if ticks_this_frame > max_ticks {
                    warn!("The fixed update schedule is falling behind. Consider increasing the period or decreasing the amount of work done in the fixed update schedule.");
                    break;
                }
            }

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
