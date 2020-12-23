use crate::Time;
use bevy_ecs::{ArchetypeComponent, ShouldRun, System, SystemId, ThreadLocalExecution, TypeAccess};
use bevy_utils::HashMap;
use std::{any::TypeId, borrow::Cow};

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
    step: f64,
    accumulator: f64,
    looping: bool,
    system_id: SystemId,
    label: Option<String>, // TODO: consider making this a TypedLabel
    resource_access: TypeAccess<TypeId>,
    archetype_access: TypeAccess<ArchetypeComponent>,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self {
            system_id: SystemId::new(),
            step: 1.0 / 60.0,
            accumulator: 0.0,
            looping: false,
            label: None,
            resource_access: Default::default(),
            archetype_access: Default::default(),
        }
    }
}

impl FixedTimestep {
    pub fn step(step: f64) -> Self {
        Self {
            step,
            ..Default::default()
        }
    }

    pub fn steps_per_second(rate: f64) -> Self {
        Self {
            step: 1.0 / rate,
            ..Default::default()
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn update(&mut self, time: &Time) -> ShouldRun {
        if !self.looping {
            self.accumulator += time.delta_seconds_f64();
        }

        if self.accumulator >= self.step {
            self.accumulator -= self.step;
            self.looping = true;
            ShouldRun::YesAndLoop
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

    fn id(&self) -> SystemId {
        self.system_id
    }

    fn update(&mut self, _world: &bevy_ecs::World) {}

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        ThreadLocalExecution::Immediate
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: Self::In,
        _world: &bevy_ecs::World,
        resources: &bevy_ecs::Resources,
    ) -> Option<Self::Out> {
        let time = resources.get::<Time>().unwrap();
        let result = self.update(&time);
        if let Some(ref label) = self.label {
            let mut fixed_timesteps = resources.get_mut::<FixedTimesteps>().unwrap();
            let state = fixed_timesteps.fixed_timesteps.get_mut(label).unwrap();
            state.step = self.step;
            state.accumulator = self.accumulator;
        }

        Some(result)
    }

    fn run_thread_local(
        &mut self,
        _world: &mut bevy_ecs::World,
        _resources: &mut bevy_ecs::Resources,
    ) {
    }

    fn initialize(&mut self, _world: &mut bevy_ecs::World, resources: &mut bevy_ecs::Resources) {
        self.resource_access.add_read(TypeId::of::<Time>());
        if let Some(ref label) = self.label {
            let mut fixed_timesteps = resources.get_mut::<FixedTimesteps>().unwrap();
            fixed_timesteps.fixed_timesteps.insert(
                label.clone(),
                FixedTimestepState {
                    accumulator: 0.0,
                    step: self.step,
                },
            );
        }
    }
}
