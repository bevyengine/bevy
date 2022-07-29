use std::fmt::Debug;

use bevy_app::{App, CoreStage, IntoSubSchedule};
use bevy_ecs::{
    change_detection::Mut,
    schedule::{ScheduleLabel, ScheduleLabelId, Stage, StageLabel},
    world::World,
};
use bevy_utils::HashMap;

use crate::Time;

/// Extends [`App`] with methods for constructing schedules with a fixed time-step.
pub trait TimestepAppExt {
    /// Adds a new fixed time-step schedule to the stage identified by `stage_label`.
    /// If `timestep` has a label, it is applied to the schedule as well.
    ///
    /// # Panics
    /// If there is already a fixed schedule identified by `label`.
    fn add_fixed_schedule_to_stage<S: Stage>(
        &mut self,
        stage_label: impl StageLabel,
        timestep: FixedTimestep,
        schedule: S,
    ) -> &mut Self;
    /// Adds a new fixed time-step schedule to [`CoreStage::Update`].
    /// If `timestep` has a label, it is applied to the schedule as well.
    ///
    /// # Panics
    /// If there is already a fixed schedule identified by `label`.
    fn add_fixed_schedule(&mut self, timestep: FixedTimestep, schedule: impl Stage) -> &mut Self {
        self.add_fixed_schedule_to_stage(CoreStage::Update, timestep, schedule)
    }
}

impl TimestepAppExt for App {
    #[track_caller]
    fn add_fixed_schedule_to_stage<S: Stage>(
        &mut self,
        stage: impl StageLabel,
        FixedTimestep { label, step }: FixedTimestep,
        schedule: S,
    ) -> &mut Self {
        // If it has a label, add it to the resource so it can be modified or peeked later.
        if let Some(label) = label {
            let mut timesteps: Mut<FixedTimesteps> =
                self.world.get_resource_or_insert_with(Default::default);
            let state = FixedTimestepState {
                step,
                accumulator: 0.0,
            };

            // Insert the state into the map.
            // Panic if there already was one.
            if timesteps.insert(label, state).is_some() {
                #[inline(never)]
                #[track_caller]
                fn panic(label: impl Debug) -> ! {
                    panic!("there is already a fixed timestep labeled '{label:?}'");
                }
                panic(label)
            }

            let runner = move |s: &mut dyn Stage, w: &mut World| {
                let mut state = *w.resource::<FixedTimesteps>().get(label).unwrap();

                // Core looping functionality.
                let time = w.resource::<Time>();
                state.accumulator += time.delta_seconds_f64();
                while state.accumulator > state.step {
                    state.accumulator -= state.step;

                    s.run(w);
                }

                // Update the resource (we've only been operating on a copy).
                w.resource_mut::<FixedTimesteps>().insert(label, state);
            };
            self.add_sub_schedule(schedule.label(label).with_runner(stage, runner));
        }
        // If there's no label, we can keep everything local
        // since there's no way to refer to the schedule again anyway.
        else {
            let mut state = FixedTimestepState {
                step,
                accumulator: 0.0,
            };
            let runner = move |sched: &mut dyn Stage, w: &mut World| {
                // Core looping functionality.
                let time = w.resource::<Time>();
                state.accumulator += time.delta_seconds_f64();
                while state.accumulator > state.step {
                    state.accumulator -= state.step;

                    sched.run(w);
                }
            };
            self.add_sub_schedule(schedule.with_runner(stage, runner));
        }

        self
    }
}

/// The internal state of each [`FixedTimestep`].
#[derive(Debug, Clone, Copy)]
pub struct FixedTimestepState {
    step: f64,
    accumulator: f64,
}

impl FixedTimestepState {
    /// The amount of time each step takes.
    pub fn step(&self) -> f64 {
        self.step
    }

    /// The number of steps made in a second.
    pub fn steps_per_second(&self) -> f64 {
        1.0 / self.step
    }

    /// The amount of time (in seconds) left over from the last step.
    pub fn accumulator(&self) -> f64 {
        self.accumulator
    }

    /// The percentage of "step" stored inside the accumulator. Calculated as accumulator / step.
    pub fn overstep_percentage(&self) -> f64 {
        self.accumulator / self.step
    }
}

/// A global resource that tracks the state for every labeled [`FixedTimestep`].
#[derive(Default)]
pub struct FixedTimesteps {
    fixed_timesteps: HashMap<ScheduleLabelId, FixedTimestepState>,
}

impl FixedTimesteps {
    /// Gets the [`FixedTimestepState`] for a given label.
    pub fn get(&self, label: impl ScheduleLabel) -> Option<&FixedTimestepState> {
        self.fixed_timesteps.get(&label.as_label())
    }

    fn insert(
        &mut self,
        label: impl ScheduleLabel,
        state: FixedTimestepState,
    ) -> Option<FixedTimestepState> {
        self.fixed_timesteps.insert(label.as_label(), state)
    }
}

/// Enables a sub-schedule to run at a fixed timestep between executions.
///
/// This does not guarantee that the time elapsed between executions is exactly the provided
/// fixed timestep, but will guarantee that the execution will run multiple times per game tick
/// until the number of repetitions is as expected.
///
/// For example, a schedule with a fixed timestep of 120 times per second will run
/// two times during a ~16.667ms frame, once during a ~8.333ms frame, and once every two frames
/// with ~4.167ms frames. However, the same criteria may not result in exactly 8.333ms passing
/// between each execution.
///
/// When using this pattern, it is advised not to rely on [`Time::delta`] or any of its
/// variants for game simulation, but rather use the constant time delta used to initialize the
/// [`FixedTimestep`] instead.
///
/// For more fine tuned information about the execution status of a given fixed timestep,
/// use the [`FixedTimesteps`] resource.
pub struct FixedTimestep {
    label: Option<ScheduleLabelId>,
    step: f64,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self::steps_per_second(60.0)
    }
}

impl FixedTimestep {
    /// Creates a [`FixedTimestep`] that ticks once every `step` seconds.
    pub fn step(step: f64) -> Self {
        Self { step, label: None }
    }

    /// Creates a [`FixedTimestep`] that ticks once every `rate` times per second.
    pub fn steps_per_second(rate: f64) -> Self {
        Self::step(rate.recip())
    }

    /// Sets the label for the timestep. Setting a label allows a timestep
    /// to be observed by the global [`FixedTimesteps`] resource.
    #[must_use]
    pub fn with_label(mut self, label: impl ScheduleLabel) -> Self {
        self.label = Some(label.as_label());
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_ecs::prelude::*;
    use bevy_utils::Instant;
    use std::ops::{Add, Mul};
    use std::time::Duration;

    type Count = usize;

    #[derive(ScheduleLabel)]
    struct FixedUpdate;

    #[test]
    fn test() {
        let mut app = App::new();
        let mut time = Time::default();
        let instance = Instant::now();
        time.update_with_instant(instance);
        app.insert_resource(time);
        app.insert_resource::<Count>(0);

        // Add a new fixed timestep that runs every 0.5 seconds.
        app.add_fixed_schedule(
            FixedTimestep::step(0.5).with_label(FixedUpdate),
            SystemStage::single_threaded().with_system(fixed_update),
        );

        // if time does not progress, the step does not run
        app.update();
        app.update();
        assert_eq!(0, *app.world.resource::<Count>());
        assert_eq!(0., get_accumulator_deciseconds(&app.world));

        // let's progress less than one step
        advance_time(&mut app.world, instance, 0.4);
        app.update();
        assert_eq!(0, *app.world.resource::<Count>());
        assert_eq!(4., get_accumulator_deciseconds(&app.world));

        // finish the first step with 0.1s above the step length
        advance_time(&mut app.world, instance, 0.6);
        app.update();
        assert_eq!(1, *app.world.resource::<Count>());
        assert_eq!(1., get_accumulator_deciseconds(&app.world));

        // runs multiple times if the delta is multiple step lengths
        advance_time(&mut app.world, instance, 1.7);
        app.update();
        assert_eq!(3, *app.world.resource::<Count>());
        assert_eq!(2., get_accumulator_deciseconds(&app.world));
    }

    fn fixed_update(mut count: ResMut<Count>) {
        *count += 1;
    }

    fn advance_time(world: &mut World, instance: Instant, seconds: f64) {
        world
            .resource_mut::<Time>()
            .update_with_instant(instance.add(Duration::from_secs_f64(seconds)));
    }

    fn get_accumulator_deciseconds(world: &World) -> f64 {
        world
            .resource::<FixedTimesteps>()
            .get(FixedUpdate)
            .unwrap()
            .accumulator
            .mul(10.)
            .round()
    }
}
