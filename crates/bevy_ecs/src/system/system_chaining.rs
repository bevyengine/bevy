use crate::{
    ArchetypeComponent, Resources, System, SystemId, ThreadLocalExecution, TypeAccess, World,
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

impl<SystemA: System, SystemB: System<In = SystemA::Out>> System for ChainSystem<SystemA, SystemB> {
    type In = SystemA::In;
    type Out = SystemB::Out;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
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
        input: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        let out = self.system_a.run_unsafe(input, world, resources).unwrap();
        self.system_b.run_unsafe(out, world, resources)
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        self.system_a.run_thread_local(world, resources);
        self.system_b.run_thread_local(world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        self.system_a.initialize(world, resources);
        self.system_b.initialize(world, resources);
    }
}

pub trait IntoChainSystem<SystemB>: System + Sized
where
    SystemB: System<In = Self::Out>,
{
    fn chain(self, system: SystemB) -> ChainSystem<Self, SystemB>;
}

impl<SystemA, SystemB> IntoChainSystem<SystemB> for SystemA
where
    SystemA: System,
    SystemB: System<In = SystemA::Out>,
{
    fn chain(self, system: SystemB) -> ChainSystem<SystemA, SystemB> {
        ChainSystem {
            name: Cow::Owned(format!("Chain({}, {})", self.name(), system.name())),
            system_a: self,
            system_b: system,
            archetype_component_access: Default::default(),
            resource_access: Default::default(),
            id: SystemId::new(),
        }
    }
}
