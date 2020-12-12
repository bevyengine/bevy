use crate::{ArchetypeComponent, IntoSystem, Resources, System, SystemId, TypeAccess, World};
use std::{any::TypeId, borrow::Cow};

pub struct ChainSystem<SystemA, SystemB> {
    system_a: SystemA,
    system_b: SystemB,
    name: Cow<'static, str>,
    id: SystemId,
    archetype_component_access: TypeAccess<ArchetypeComponent>,
    resource_access: TypeAccess<TypeId>,
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

        // TODO shouldn't this be access of both systems combined?
        self.archetype_component_access
            .extend(self.system_a.archetype_component_access());
        self.resource_access.extend(self.system_b.resource_access());
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn is_thread_local(&self) -> bool {
        self.system_a.is_thread_local() || self.system_b.is_thread_local()
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

    fn run_exclusive(&mut self, world: &mut World, resources: &mut Resources) {
        self.system_a.run_exclusive(world, resources);
        self.system_b.run_exclusive(world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        self.system_a.initialize(world, resources);
        self.system_b.initialize(world, resources);
    }
}

pub trait IntoChainSystem<AParams, BParams, IntoB, SystemA, SystemB>:
    IntoSystem<AParams, SystemA> + Sized
where
    IntoB: IntoSystem<BParams, SystemB>,
    SystemA: System,
    SystemB: System<In = SystemA::Out>,
{
    fn chain(self, system: IntoB) -> ChainSystem<SystemA, SystemB>;
}

impl<AParams, BParams, IntoA, IntoB, SystemA, SystemB>
    IntoChainSystem<AParams, BParams, IntoB, SystemA, SystemB> for IntoA
where
    SystemA: System,
    SystemB: System<In = SystemA::Out>,
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
