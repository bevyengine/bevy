pub use super::Query;
use crate::{
    resource::Resources,
    system::{System, SystemId},
    ArchetypeComponent, IntoSystem, TypeAccess, World,
};
use std::{any::TypeId, borrow::Cow};

pub struct ExclusiveSystemFn {
    pub func: Box<dyn FnMut(&mut World, &mut Resources) + Send + Sync + 'static>,
    pub name: Cow<'static, str>,
    pub id: SystemId,
    pub archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub resource_access: TypeAccess<TypeId>,
}

impl System for ExclusiveSystemFn {
    type In = ();
    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn update_access(&mut self, _world: &World) {}

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn is_thread_local(&self) -> bool {
        // Doesn't really matter, exclusive access is a stronger constraint.
        true
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: (),
        _world: &World,
        _resources: &Resources,
    ) -> Option<()> {
        Some(())
    }

    fn run_exclusive(&mut self, world: &mut World, resources: &mut Resources) {
        (self.func)(world, resources);
    }

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

impl<F> IntoSystem<(&mut World, &mut Resources), ExclusiveSystemFn> for F
where
    F: FnMut(&mut World, &mut Resources) + Send + Sync + 'static,
{
    fn system(mut self) -> ExclusiveSystemFn {
        let mut archetype_component_access = TypeAccess::default();
        let mut resource_access = TypeAccess::default();
        archetype_component_access.write_all();
        resource_access.write_all();
        ExclusiveSystemFn {
            func: Box::new(move |world, resources| (self)(world, resources)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            archetype_component_access,
            resource_access,
        }
    }
}

impl<F> IntoSystem<&mut World, ExclusiveSystemFn> for F
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn system(mut self) -> ExclusiveSystemFn {
        let mut archetype_component_access = TypeAccess::default();
        let resource_access = TypeAccess::default();
        archetype_component_access.write_all();
        ExclusiveSystemFn {
            func: Box::new(move |world, _| (self)(world)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            archetype_component_access,
            resource_access,
        }
    }
}

impl<F> IntoSystem<&mut Resources, ExclusiveSystemFn> for F
where
    F: FnMut(&mut Resources) + Send + Sync + 'static,
{
    fn system(mut self) -> ExclusiveSystemFn {
        let archetype_component_access = TypeAccess::default();
        let mut resource_access = TypeAccess::default();
        resource_access.write_all();
        ExclusiveSystemFn {
            func: Box::new(move |_, resources| (self)(resources)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            archetype_component_access,
            resource_access,
        }
    }
}
