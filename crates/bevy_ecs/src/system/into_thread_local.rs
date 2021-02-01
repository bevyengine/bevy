pub use super::Query;
use crate::{
    resource::Resources,
    system::{System, SystemId, ThreadLocalExecution},
    ArchetypeComponent, IntoSystem, TypeAccess, World,
};
use std::{any::TypeId, borrow::Cow};

pub struct ThreadLocalSystemFn {
    pub func: Box<dyn FnMut(&mut World, &mut Resources) + Send + Sync + 'static>,
    pub resource_access: TypeAccess<TypeId>,
    pub archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub name: Cow<'static, str>,
    pub id: SystemId,
}

impl System for ThreadLocalSystemFn {
    type In = ();
    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn update(&mut self, _world: &World) {}

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        ThreadLocalExecution::Immediate
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: (),
        _world: &World,
        _resources: &Resources,
    ) -> Option<()> {
        Some(())
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        (self.func)(world, resources);
    }

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}

    fn id(&self) -> SystemId {
        self.id
    }
}

impl<F> IntoSystem<(&mut World, &mut Resources), ThreadLocalSystemFn> for F
where
    F: FnMut(&mut World, &mut Resources) + Send + Sync + 'static,
{
    fn system(mut self) -> ThreadLocalSystemFn {
        ThreadLocalSystemFn {
            func: Box::new(move |world, resources| (self)(world, resources)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            resource_access: TypeAccess::default(),
            archetype_component_access: TypeAccess::default(),
        }
    }
}
