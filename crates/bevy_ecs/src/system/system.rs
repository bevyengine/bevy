use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    world::World,
};
use std::borrow::Cow;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SystemId(pub usize);

impl SystemId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SystemId(rand::random::<usize>())
    }
}

/// An ECS system that can be added to a [Schedule](crate::schedule::Schedule)
pub trait System: Send + Sync + 'static {
    type In;
    type Out;
    fn name(&self) -> Cow<'static, str>;
    fn id(&self) -> SystemId;
    fn new_archetype(&mut self, archetype: &Archetype);
    fn component_access(&self) -> &Access<ComponentId>;
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId>;
    fn is_send(&self) -> bool;
    /// # Safety
    /// This might access World and Resources in an unsafe manner. This should only be called in one of the following contexts:
    /// 1. This system is the only system running on the given World across all threads
    /// 2. This system only runs in parallel with other systems that do not conflict with the `archetype_component_access()`
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out;
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        // SAFE: world and resources are exclusively borrowed
        unsafe { self.run_unsafe(input, world) }
    }
    fn apply_buffers(&mut self, world: &mut World);
    fn initialize(&mut self, _world: &mut World);
}

pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;
