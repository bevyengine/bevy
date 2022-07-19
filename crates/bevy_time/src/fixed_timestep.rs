use crate::Time;
use bevy_ecs::{
    archetype::ArchetypeComponentId,
    component::ComponentId,
    query::Access,
    schedule::ShouldRun,
    system::{IntoSystem, Res, ResMut, System},
    world::World,
};
use bevy_utils::HashMap;
use std::borrow::Cow;

/// The internal state of each [`FixedTimestep`].
#[derive(Debug)]
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

/// A global resource that tracks the individual [`FixedTimestepState`]s
/// for every labeled [`FixedTimestep`].
#[derive(Default)]
pub struct FixedTimesteps {
    fixed_timesteps: HashMap<String, FixedTimestepState>,
}

impl FixedTimesteps {
    /// Gets the [`FixedTimestepState`] for a given label.
    pub fn get(&self, name: &str) -> Option<&FixedTimestepState> {
        self.fixed_timesteps.get(name)
    }
}

/// A system run criteria that enables systems or stages to run at a fixed timestep between executions.
///
/// This does not guarentee that the time elapsed between executions is exactly the provided
/// fixed timestep, but will guarentee that the execution will run multiple times per game tick
/// until the number of repetitions is as expected.
///
/// For example, a system with a fixed timestep run criteria of 120 times per second will run
/// two times during a ~16.667ms frame, once during a ~8.333ms frame, and once every two frames
/// with ~4.167ms frames. However, the same criteria may not result in exactly 8.333ms passing
/// between each execution.
///
/// When using this run criteria, it is advised not to rely on [`Time::delta`] or any of it's
/// variants for game simulation, but rather use the constant time delta used to initialize the
/// [`FixedTimestep`] instead.
///
/// For more fine tuned information about the execution status of a given fixed timestep,
/// use the [`FixedTimesteps`] resource.
pub struct FixedTimestep {
    state: LocalFixedTimestepState,
    internal_system: Box<dyn System<In = (), Out = ShouldRun>>,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self {
            state: LocalFixedTimestepState::default(),
            internal_system: Box::new(IntoSystem::into_system(Self::prepare_system(
                Default::default(),
            ))),
        }
    }
}

impl FixedTimestep {
    /// Creates a [`FixedTimestep`] that ticks once every `step` seconds.
    pub fn step(step: f64) -> Self {
        Self {
            state: LocalFixedTimestepState {
                step,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Creates a [`FixedTimestep`] that ticks once every `rate` times per second.
    pub fn steps_per_second(rate: f64) -> Self {
        Self {
            state: LocalFixedTimestepState {
                step: 1.0 / rate,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Sets the label for the timestep. Setting a label allows a timestep
    /// to be observed by the global [`FixedTimesteps`] resource.
    #[must_use]
    pub fn with_label(mut self, label: &str) -> Self {
        self.state.label = Some(label.to_string());
        self
    }

    fn prepare_system(
        mut state: LocalFixedTimestepState,
    ) -> impl FnMut(Res<Time>, ResMut<FixedTimesteps>) -> ShouldRun {
        move |time, mut fixed_timesteps| {
            let should_run = state.update(&time);
            if let Some(ref label) = state.label {
                let res_state = fixed_timesteps.fixed_timesteps.get_mut(label).unwrap();
                res_state.step = state.step;
                res_state.accumulator = state.accumulator;
            }

            should_run
        }
    }
}

#[derive(Clone)]
struct LocalFixedTimestepState {
    label: Option<String>, // TODO: consider making this a TypedLabel
    step: f64,
    accumulator: f64,
    looping: bool,
}

impl Default for LocalFixedTimestepState {
    fn default() -> Self {
        Self {
            step: 1.0 / 60.0,
            accumulator: 0.0,
            label: None,
            looping: false,
        }
    }
}

impl LocalFixedTimestepState {
    fn update(&mut self, time: &Time) -> ShouldRun {
        if !self.looping {
            self.accumulator += time.delta_seconds_f64();
        }

        if self.accumulator >= self.step {
            self.accumulator -= self.step;
            self.looping = true;
            ShouldRun::YesAndCheckAgain
        } else {
            self.looping = false;
            ShouldRun::No
        }
    }
}

impl System for FixedTimestep {
    type In = ();
    type Out = ShouldRun;

    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<FixedTimestep>())
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.internal_system.archetype_component_access()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.internal_system.component_access()
    }

    fn is_send(&self) -> bool {
        self.internal_system.is_send()
    }

    unsafe fn run_unsafe(&mut self, _input: (), world: &World) -> ShouldRun {
        // SAFETY: this system inherits the internal system's component access and archetype component
        // access, which means the caller has ensured running the internal system is safe
        self.internal_system.run_unsafe((), world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.internal_system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.internal_system = Box::new(IntoSystem::into_system(Self::prepare_system(
            self.state.clone(),
        )));
        self.internal_system.initialize(world);
        if let Some(ref label) = self.state.label {
            let mut fixed_timesteps = world.resource_mut::<FixedTimesteps>();
            fixed_timesteps.fixed_timesteps.insert(
                label.clone(),
                FixedTimestepState {
                    accumulator: 0.0,
                    step: self.state.step,
                },
            );
        }
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        self.internal_system
            .update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.internal_system.check_change_tick(change_tick);
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
    const LABEL: &str = "test_step";

    #[test]
    fn test() {
        let mut world = World::default();
        let mut time = Time::default();
        let instance = Instant::now();
        time.update_with_instant(instance);
        world.insert_resource(time);
        world.insert_resource(FixedTimesteps::default());
        world.insert_resource::<Count>(0);
        let mut schedule = Schedule::default();

        schedule.add_stage(
            "update",
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.5).with_label(LABEL))
                .with_system(fixed_update),
        );

        // if time does not progress, the step does not run
        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(0, *world.resource::<Count>());
        assert_eq!(0., get_accumulator_deciseconds(&world));

        // let's progress less than one step
        advance_time(&mut world, instance, 0.4);
        schedule.run(&mut world);
        assert_eq!(0, *world.resource::<Count>());
        assert_eq!(4., get_accumulator_deciseconds(&world));

        // finish the first step with 0.1s above the step length
        advance_time(&mut world, instance, 0.6);
        schedule.run(&mut world);
        assert_eq!(1, *world.resource::<Count>());
        assert_eq!(1., get_accumulator_deciseconds(&world));

        // runs multiple times if the delta is multiple step lengths
        advance_time(&mut world, instance, 1.7);
        schedule.run(&mut world);
        assert_eq!(3, *world.resource::<Count>());
        assert_eq!(2., get_accumulator_deciseconds(&world));
    }

    fn fixed_update(mut count: ResMut<Count>) {
        *count += 1;
    }

    fn advance_time(world: &mut World, instance: Instant, seconds: f32) {
        world
            .resource_mut::<Time>()
            .update_with_instant(instance.add(Duration::from_secs_f32(seconds)));
    }

    fn get_accumulator_deciseconds(world: &World) -> f64 {
        world
            .resource::<FixedTimesteps>()
            .get(LABEL)
            .unwrap()
            .accumulator
            .mul(10.)
            .round()
    }
}
