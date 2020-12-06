use crate::Time;
use bevy_ecs::{ArchetypeComponent, ShouldRun, System, SystemId, ThreadLocalExecution, TypeAccess};
use std::{any::TypeId, borrow::Cow};

pub struct FixedTimestep {
    step: f64,
    accumulator: f64,
    looping: bool,
    system_id: SystemId,
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
    type Input = ();
    type Output = ShouldRun;

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
        _input: Self::Input,
        _world: &bevy_ecs::World,
        resources: &bevy_ecs::Resources,
    ) -> Option<Self::Output> {
        let time = resources.get::<Time>().unwrap();
        Some(self.update(&time))
    }

    fn run_thread_local(
        &mut self,
        _world: &mut bevy_ecs::World,
        _resources: &mut bevy_ecs::Resources,
    ) {
    }

    fn initialize(&mut self, _world: &mut bevy_ecs::World, _resources: &mut bevy_ecs::Resources) {}
}
