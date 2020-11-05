pub use super::Query;
use crate::{
    resource::Resources,
    system::{System, SystemId, ThreadLocalExecution},
    TypeAccess,
};
use bevy_hecs::{ArchetypeComponent, World};
use std::{any::TypeId, borrow::Cow};

#[derive(Debug)]
pub(crate) struct SystemFn<State, F, ThreadLocalF, Init, Update>
where
    F: FnMut(&World, &Resources, &mut State) + Send + Sync,
    ThreadLocalF: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Init: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Update: FnMut(&World, &mut TypeAccess<ArchetypeComponent>, &mut State) + Send + Sync,
    State: Send + Sync,
{
    pub state: State,
    pub func: F,
    pub thread_local_func: ThreadLocalF,
    pub init_func: Init,
    pub thread_local_execution: ThreadLocalExecution,
    pub resource_access: TypeAccess<TypeId>,
    pub name: Cow<'static, str>,
    pub id: SystemId,
    pub archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub update_func: Update,
}

impl<State, F, ThreadLocalF, Init, Update> System for SystemFn<State, F, ThreadLocalF, Init, Update>
where
    F: FnMut(&World, &Resources, &mut State) + Send + Sync,
    ThreadLocalF: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Init: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Update: FnMut(&World, &mut TypeAccess<ArchetypeComponent>, &mut State) + Send + Sync,
    State: Send + Sync,
{
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn update(&mut self, world: &World) {
        (self.update_func)(world, &mut self.archetype_component_access, &mut self.state);
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        self.thread_local_execution
    }

    #[inline]
    fn run(&mut self, world: &World, resources: &Resources) {
        (self.func)(world, resources, &mut self.state);
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        (self.thread_local_func)(world, resources, &mut self.state);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        (self.init_func)(world, resources, &mut self.state);
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn is_initialized(&self) -> bool {
        // TODO: either make this correct or remove everything in this file :)
        false
    }
}

/// Converts `Self` into a thread local system
pub trait IntoThreadLocalSystem {
    fn thread_local_system(self) -> Box<dyn System>;
}

impl<F> IntoThreadLocalSystem for F
where
    F: ThreadLocalSystemFn,
{
    fn thread_local_system(mut self) -> Box<dyn System> {
        Box::new(SystemFn {
            state: (),
            thread_local_func: move |world, resources, _| {
                self.run(world, resources);
            },
            func: |_, _, _| {},
            init_func: |_, _, _| {},
            update_func: |_, _, _| {},
            thread_local_execution: ThreadLocalExecution::Immediate,
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            resource_access: TypeAccess::default(),
            archetype_component_access: TypeAccess::default(),
        })
    }
}

/// A thread local system function
pub trait ThreadLocalSystemFn: Send + Sync + 'static {
    fn run(&mut self, world: &mut World, resource: &mut Resources);
}

impl<F> ThreadLocalSystemFn for F
where
    F: FnMut(&mut World, &mut Resources) + Send + Sync + 'static,
{
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        self(world, resources);
    }
}