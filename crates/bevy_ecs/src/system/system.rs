use crate::{ArchetypeComponent, Resources, TypeAccess, World};
use std::{any::TypeId, borrow::Cow};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SystemId(pub usize);

impl SystemId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SystemId(rand::random::<usize>())
    }
}

/// An ECS system that can be added to a [Schedule](crate::Schedule)
pub trait System: Send + Sync + 'static {
    type In;
    type Out;
    fn name(&self) -> Cow<'static, str>;
    fn id(&self) -> SystemId;
    fn update_access(&mut self, world: &World);
    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent>;
    fn component_access(&self) -> &TypeAccess<TypeId>;
    fn resource_access(&self) -> &TypeAccess<TypeId>;
    fn is_non_send(&self) -> bool;
    /// # Safety
    /// This might access World and Resources in an unsafe manner. This should only be called in one of the following contexts:
    /// 1. This system is the only system running on the given World and Resources across all threads
    /// 2. This system only runs in parallel with other systems that do not conflict with the `archetype_component_access()` or `resource_access()`
    unsafe fn run_unsafe(
        &mut self,
        input: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out>;
    fn run(
        &mut self,
        input: Self::In,
        world: &mut World,
        resources: &mut Resources,
    ) -> Option<Self::Out> {
        // SAFE: world and resources are exclusively borrowed
        unsafe { self.run_unsafe(input, world, resources) }
    }
    fn apply_buffers(&mut self, world: &mut World, resources: &mut Resources);
    fn initialize(&mut self, world: &mut World, resources: &mut Resources);
}

pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;
