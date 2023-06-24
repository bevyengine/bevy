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
use bevy_ecs::world::World;

/// Ticks the [`FixedTime`] resource then runs the [`FixedUpdate`].
pub fn run_fixed_update_schedule(world: &mut World) {
    // Run the schedule until we run out of accumulated time
    let _ = world.try_schedule_scope(FixedUpdate, |world, schedule| {
        while world.resource_mut::<Time>().expend_fixed() {
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
