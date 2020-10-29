use crate::resource::Resources;
use bevy_hecs::{ArchetypeComponent, TypeAccess, World};
use std::{any::TypeId, borrow::Cow};

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
    fn resource_access(&self) -> &TypeAccess<TypeId>;
    fn thread_local_execution(&self) -> ThreadLocalExecution;
    fn run(&mut self, world: &World, resources: &Resources);
    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources);
    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}
