use crate::{
    ArchetypeComponent, IntoSystem, Resources, System, SystemId, ThreadLocalExecution, TypeAccess,
    World,
};
use std::{any::TypeId, borrow::Cow};

pub struct ChainSystem<SystemA, SystemB> {
    system_a: SystemA,
    system_b: SystemB,
    name: Cow<'static, str>,
    id: SystemId,
    pub(crate) archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub(crate) resource_access: TypeAccess<TypeId>,
}

impl<SystemA: System, SystemB: System<Input = SystemA::Output>> System
    for ChainSystem<SystemA, SystemB>
{
    type Input = SystemA::Input;
    type Output = SystemB::Output;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn is_initialized(&self) -> bool {
        self.system_a.is_initialized() && self.system_b.is_initialized()
    }

    fn update(&mut self, world: &World) {
        self.archetype_component_access.clear();
        self.resource_access.clear();
        self.system_a.update(world);
        self.system_b.update(world);

        self.archetype_component_access
            .union(self.system_a.archetype_component_access());
        self.resource_access.union(self.system_b.resource_access());
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        ThreadLocalExecution::NextFlush
    }

    unsafe fn run_unsafe(
        &mut self,
        input: Self::Input,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Output> {
        let out = self.system_a.run_unsafe(input, world, resources).unwrap();
        self.system_b.run_unsafe(out, world, resources)
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        self.system_a.run_thread_local(world, resources);
        self.system_b.run_thread_local(world, resources);
    }
}

pub trait IntoChainSystem<AParams, BParams, IntoB, SystemA, SystemB>:
    IntoSystem<AParams, SystemA> + Sized
where
    IntoB: IntoSystem<BParams, SystemB>,
    SystemA: System,
    SystemB: System<Input = SystemA::Output>,
{
    fn chain(self, system: IntoB) -> ChainSystem<SystemA, SystemB>;
}

impl<AParams, BParams, IntoA, IntoB, SystemA, SystemB>
    IntoChainSystem<AParams, BParams, IntoB, SystemA, SystemB> for IntoA
where
    SystemA: System,
    SystemB: System<Input = SystemA::Output>,
    IntoA: IntoSystem<AParams, SystemA>,
    IntoB: IntoSystem<BParams, SystemB>,
{
    fn chain(self, system: IntoB) -> ChainSystem<SystemA, SystemB> {
        let system_a = self.system();
        let system_b = system.system();
        ChainSystem {
            name: Cow::Owned(format!("Chain({}, {})", system_a.name(), system_b.name())),
            system_a,
            system_b,
            archetype_component_access: Default::default(),
            resource_access: Default::default(),
            id: SystemId::new(),
        }
    }
}

pub struct FilledInputSystem<S: System>
where
    S::Input: Clone,
{
    system: S,
    input: S::Input,
}

impl<S: System> System for FilledInputSystem<S>
where
    S::Input: Clone + Send + Sync,
{
    type Input = ();
    type Output = S::Output;

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn id(&self) -> SystemId {
        self.system.id()
    }

    fn is_initialized(&self) -> bool {
        self.system.is_initialized()
    }

    fn update(&mut self, world: &World) {
        self.system.update(world);
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        self.system.archetype_component_access()
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        self.system.resource_access()
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        self.system.thread_local_execution()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: (),
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Output> {
        self.system.run_unsafe(self.input.clone(), world, resources)
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.run_thread_local(world, resources);
    }
}

pub trait FillSystemInput<SParams, S, IntoS>
where
    S: System,
    S::Input: Clone + Send + Sync,
    IntoS: IntoSystem<SParams, S>,
{
    fn fill_input(self, input: S::Input) -> FilledInputSystem<S>;
}

impl<Params, S: System, IntoS> FillSystemInput<Params, S, IntoS> for IntoS
where
    S::Input: Clone + Send + Sync,
    IntoS: IntoSystem<Params, S>,
{
    fn fill_input(self, input: S::Input) -> FilledInputSystem<S> {
        FilledInputSystem {
            system: self.system(),
            input,
        }
    }
}
