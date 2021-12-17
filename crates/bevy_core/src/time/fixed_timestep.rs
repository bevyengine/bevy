use crate::Time;
use bevy_ecs::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    schedule::ShouldRun,
    system::{ConfigurableSystem, IntoSystem, Local, Res, ResMut, System},
    world::World,
};
use bevy_utils::HashMap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct FixedTimestepState {
    pub step: f64,
    pub accumulator: f64,
}

impl FixedTimestepState {
    /// The amount of time each step takes
    pub fn step(&self) -> f64 {
        self.step
    }

    /// The number of steps made in a second
    pub fn steps_per_second(&self) -> f64 {
        1.0 / self.step
    }

    /// The amount of time (in seconds) left over from the last step
    pub fn accumulator(&self) -> f64 {
        self.accumulator
    }

    /// The percentage of "step" stored inside the accumulator. Calculated as accumulator / step
    pub fn overstep_percentage(&self) -> f64 {
        self.accumulator / self.step
    }
}

#[derive(Default)]
pub struct FixedTimesteps {
    fixed_timesteps: HashMap<String, FixedTimestepState>,
}

impl FixedTimesteps {
    pub fn get(&self, name: &str) -> Option<&FixedTimestepState> {
        self.fixed_timesteps.get(name)
    }
}

pub struct FixedTimestep {
    state: LocalFixedTimestepState,
    internal_system: Box<dyn System<In = (), Out = ShouldRun>>,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self {
            state: LocalFixedTimestepState::default(),
            internal_system: Box::new(Self::prepare_system.system()),
        }
    }
}

impl FixedTimestep {
    pub fn step(step: f64) -> Self {
        Self {
            state: LocalFixedTimestepState {
                step,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn steps_per_second(rate: f64) -> Self {
        Self {
            state: LocalFixedTimestepState {
                step: 1.0 / rate,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.state.label = Some(label.to_string());
        self
    }

    fn prepare_system(
        mut state: Local<LocalFixedTimestepState>,
        time: Res<Time>,
        mut fixed_timesteps: ResMut<FixedTimesteps>,
    ) -> ShouldRun {
        let should_run = state.update(&time);
        if let Some(ref label) = state.label {
            let res_state = fixed_timesteps.fixed_timesteps.get_mut(label).unwrap();
            res_state.step = state.step;
            res_state.accumulator = state.accumulator;
        }

        should_run
    }
}

#[derive(Clone)]
pub struct LocalFixedTimestepState {
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

    fn new_archetype(&mut self, archetype: &Archetype) {
        self.internal_system.new_archetype(archetype);
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
        // SAFE: this system inherits the internal system's component access and archetype component
        // access, which means the caller has ensured running the internal system is safe
        self.internal_system.run_unsafe((), world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.internal_system.apply_buffers(world)
    }

    fn initialize(&mut self, world: &mut World) {
        self.internal_system =
            Box::new(Self::prepare_system.config(|c| c.0 = Some(self.state.clone())));
        self.internal_system.initialize(world);
        if let Some(ref label) = self.state.label {
            let mut fixed_timesteps = world.get_resource_mut::<FixedTimesteps>().unwrap();
            fixed_timesteps.fixed_timesteps.insert(
                label.clone(),
                FixedTimestepState {
                    accumulator: 0.0,
                    step: self.state.step,
                },
            );
        }
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
        assert_eq!(0, *world.get_resource::<Count>().unwrap());
        assert_eq!(0., get_accumulator_deciseconds(&world));

        // let's progress less than one step
        advance_time(&mut world, instance, 0.4);
        schedule.run(&mut world);
        assert_eq!(0, *world.get_resource::<Count>().unwrap());
        assert_eq!(4., get_accumulator_deciseconds(&world));

        // finish the first step with 0.1s above the step length
        advance_time(&mut world, instance, 0.6);
        schedule.run(&mut world);
        assert_eq!(1, *world.get_resource::<Count>().unwrap());
        assert_eq!(1., get_accumulator_deciseconds(&world));

        // runs multiple times if the delta is multiple step lengths
        advance_time(&mut world, instance, 1.7);
        schedule.run(&mut world);
        assert_eq!(3, *world.get_resource::<Count>().unwrap());
        assert_eq!(2., get_accumulator_deciseconds(&world));
    }

    fn fixed_update(mut count: ResMut<Count>) {
        *count += 1;
    }

    fn advance_time(world: &mut World, instance: Instant, seconds: f32) {
        world
            .get_resource_mut::<Time>()
            .unwrap()
            .update_with_instant(instance.add(Duration::from_secs_f32(seconds)));
    }

    fn get_accumulator_deciseconds(world: &World) -> f64 {
        world
            .get_resource::<FixedTimesteps>()
            .unwrap()
            .get(LABEL)
            .unwrap()
            .accumulator
            .mul(10.)
            .round()
    }
}
