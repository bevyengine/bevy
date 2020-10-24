use crate::resource::Resources;
use bevy_hecs::{ArchetypeComponent, ComponentId, TypeAccess, World};
use std::borrow::Cow;

#[cfg(feature = "dynamic-api")]
use crate::StatefulQuery;
#[cfg(feature = "dynamic-api")]
use bevy_hecs::DynamicQuery;

/// Determines the strategy used to run the `run_thread_local` function in a [System]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ThreadLocalExecution {
    Immediate,
    NextFlush,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SystemId(pub usize);

impl SystemId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SystemId(rand::random::<usize>())
    }
}

/// An ECS system that can be added to a [Schedule](crate::Schedule)
pub trait System: Send + Sync {
    fn name(&self) -> Cow<'static, str>;
    fn id(&self) -> SystemId;
    fn update(&mut self, world: &World);
    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent>;
    fn resource_access(&self) -> &TypeAccess<ComponentId>;
    fn thread_local_execution(&self) -> ThreadLocalExecution;
    fn run(&mut self, world: &World, resources: &Resources);
    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources);
    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

#[cfg(feature = "dynamic-api")]
pub struct DynamicSystem<S> {
    pub name: String,
    pub state: S,
    system_id: SystemId,
    system_archetype_component_access: TypeAccess<ArchetypeComponent>,
    query_archetype_component_accesses: Vec<TypeAccess<ArchetypeComponent>>,
    resource_access: TypeAccess<ComponentId>,
    settings: DynamicSystemSettings<S>,
}

#[cfg(feature = "dynamic-api")]
pub struct DynamicSystemSettings<S> {
    pub workload: fn(&mut S, &Resources, &mut [StatefulQuery<DynamicQuery, DynamicQuery>]),
    pub queries: Vec<DynamicQuery>,
    pub thread_local_execution: ThreadLocalExecution,
    pub thread_local_system: fn(&mut S, &mut World, &mut Resources),
    pub init_function: fn(&mut S, &mut World, &mut Resources),
    pub resource_access: TypeAccess<ComponentId>,
}

#[cfg(feature = "dynamic-api")]
impl<S> Default for DynamicSystemSettings<S> {
    fn default() -> Self {
        Self {
            workload: |_, _, _| (),
            queries: Default::default(),
            thread_local_execution: ThreadLocalExecution::NextFlush,
            thread_local_system: |_, _, _| (),
            init_function: |_, _, _| (),
            resource_access: Default::default(),
        }
    }
}

#[cfg(feature = "dynamic-api")]
impl<S> DynamicSystem<S> {
    pub fn new(name: String, state: S) -> Self {
        DynamicSystem {
            name,
            state,
            system_id: SystemId::new(),
            resource_access: Default::default(),
            system_archetype_component_access: Default::default(),
            query_archetype_component_accesses: Default::default(),
            settings: Default::default(),
        }
    }

    pub fn settings(mut self, settings: DynamicSystemSettings<S>) -> Self {
        self.settings = settings;
        self
    }
}

#[cfg(feature = "dynamic-api")]
impl<S: Send + Sync> System for DynamicSystem<S> {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        self.name.clone().into()
    }

    fn id(&self) -> SystemId {
        self.system_id
    }

    fn update(&mut self, world: &World) {
        let Self {
            query_archetype_component_accesses,
            system_archetype_component_access,
            settings,
            ..
        } = self;

        // Clear previous archetype access list
        system_archetype_component_access.clear();

        for (query, component_access) in settings
            .queries
            .iter()
            .zip(query_archetype_component_accesses.iter_mut())
        {
            // Update the component access with the archetypes in the world
            component_access.clear();
            query
                .access
                .get_world_archetype_access(world, Some(component_access));

            // Make sure the query doesn't collide with any existing queries
            if component_access
                .get_conflict(system_archetype_component_access)
                .is_some()
            {
                panic!("Dynamic system has conflicting queries.");
            }
        }
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.system_archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<ComponentId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        self.settings.thread_local_execution
    }

    fn run(&mut self, world: &World, resources: &Resources) {
        let mut queries = self
            .settings
            .queries
            .iter()
            .zip(self.query_archetype_component_accesses.iter())
            // TODO: Try to avoid cloning the query here
            .map(|(query, access)| StatefulQuery::new(world, access, query.clone()))
            .collect::<Vec<_>>();

        (self.settings.workload)(&mut self.state, resources, queries.as_mut_slice());
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        (self.settings.thread_local_system)(&mut self.state, world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        // Initialize the archetype component accesses with blank accesses
        for _ in &self.settings.queries {
            self.query_archetype_component_accesses
                .push(TypeAccess::<ArchetypeComponent>::default());
        }

        (self.settings.init_function)(&mut self.state, world, resources);
    }
}
