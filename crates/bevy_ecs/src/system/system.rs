use bevy_utils::tracing::warn;

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
///
/// Systems are functions with all arguments implementing [SystemParam](crate::system::SystemParam).
///
/// Systems are added to an application using `AppBuilder::add_system(my_system.system())`
/// or similar methods, and will generally run once per pass of the main loop.
///
/// Systems are executed in parallel, in opportunistic order; data access is managed automatically.
/// It's possible to specify explicit execution order between specific systems,
/// see [SystemDescriptor](crate::schedule::SystemDescriptor).
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
    /// This might access World and Resources in an unsafe manner. This should only be called in one
    /// of the following contexts: 1. This system is the only system running on the given World
    /// across all threads 2. This system only runs in parallel with other systems that do not
    /// conflict with the `archetype_component_access()`
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out;
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        // SAFE: world and resources are exclusively borrowed
        unsafe { self.run_unsafe(input, world) }
    }
    fn apply_buffers(&mut self, world: &mut World);
    fn initialize(&mut self, _world: &mut World);
    fn check_system_counter(&mut self, global_system_counter: u32);
}

pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

pub(crate) fn check_system_counter_impl(
    counter: &mut u32,
    global_system_counter: u32,
    system_name: &str,
) {
    let counter_age = global_system_counter.wrapping_sub(*counter);
    let max_age = (u32::MAX / 4) * 3;
    // Clamp to max age
    if counter_age > max_age {
        warn!(
            "Too many intervening systems have run since the last time System '{}' was last run; it may fail to detect changes.",
            system_name
        );
        *counter = global_system_counter.wrapping_sub(max_age);
    }
}
