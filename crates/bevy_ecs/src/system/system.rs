use bevy_utils::tracing::warn;

use crate::{
    archetype::ArchetypeComponentId, change_detection::MAX_CHANGE_AGE, component::ComponentId,
    query::Access, schedule::SystemLabelId, world::World,
};
use std::borrow::Cow;

/// An ECS system that can be added to a [`Schedule`](crate::schedule::Schedule)
///
/// Systems are functions with all arguments implementing
/// [`SystemParam`](crate::system::SystemParam).
///
/// Systems are added to an application using `App::add_system(my_system)`
/// or similar methods, and will generally run once per pass of the main loop.
///
/// Systems are executed in parallel, in opportunistic order; data access is managed automatically.
/// It's possible to specify explicit execution order between specific systems,
/// see [`SystemDescriptor`](crate::schedule::SystemDescriptor).
pub trait System: Send + Sync + 'static {
    /// The system's input. See [`In`](crate::system::In) for
    /// [`FunctionSystem`](crate::system::FunctionSystem)s.
    type In;
    /// The system's output.
    type Out;
    /// Returns the system's name.
    fn name(&self) -> Cow<'static, str>;
    /// If this system is a [`ChainSystem`], returns the name of the first executed system.
    /// Returns the name of this system otherwise.
    fn in_system_name(&self) -> Cow<'static, str> {
        self.name()
    }
    /// If this system is a [`ChainSystem`], returns the name of the last executed system.
    /// Returns the name of this system otherwise.
    fn out_system_name(&self) -> Cow<'static, str> {
        self.name()
    }
    /// Returns the system's component [`Access`].
    fn component_access(&self) -> &Access<ComponentId>;
    /// Returns the system's archetype component [`Access`].
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId>;
    /// Returns true if the system is [`Send`].
    fn is_send(&self) -> bool;
    /// Runs the system with the given input in the world. Unlike [`System::run`], this function
    /// takes a shared reference to [`World`] and may therefore break Rust's aliasing rules, making
    /// it unsafe to call.
    ///
    /// # Safety
    ///
    /// This might access world and resources in an unsafe manner. This should only be called in one
    /// of the following contexts:
    ///     1. This system is the only system running on the given world across all threads.
    ///     2. This system only runs in parallel with other systems that do not conflict with the
    ///        [`System::archetype_component_access()`].
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World, run_meta: RunMeta)
        -> Self::Out;
    /// Runs the system with the given input in the world.
    fn run(&mut self, input: Self::In, world: &mut World, run_meta: RunMeta) -> Self::Out {
        self.update_archetype_component_access(world);
        // SAFETY: world and resources are exclusively borrowed
        unsafe { self.run_unsafe(input, world, run_meta) }
    }
    fn apply_buffers(&mut self, world: &mut World);
    /// Initialize the system.
    fn initialize(&mut self, _world: &mut World);
    /// Update the system's archetype component [`Access`].
    fn update_archetype_component_access(&mut self, world: &World);
    fn check_change_tick(&mut self, change_tick: u32);
    /// The default labels for the system
    fn default_labels(&self) -> Vec<SystemLabelId> {
        Vec::new()
    }
}

/// A convenience type alias for a boxed [`System`] trait object.
pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

pub(crate) fn check_system_change_tick(
    last_change_tick: &mut u32,
    change_tick: u32,
    system_name: &str,
) {
    let age = change_tick.wrapping_sub(*last_change_tick);
    // This comparison assumes that `age` has not overflowed `u32::MAX` before, which will be true
    // so long as this check always runs before that can happen.
    if age > MAX_CHANGE_AGE {
        warn!(
            "System '{}' has not run for {} ticks. \
            Changes older than {} ticks will not be detected.",
            system_name,
            age,
            MAX_CHANGE_AGE - 1,
        );
        *last_change_tick = change_tick.wrapping_sub(MAX_CHANGE_AGE);
    }
}

#[derive(Debug, Clone)]
pub struct RunMeta {
    pub previous_system_name: Option<Cow<'static, str>>,
    pub next_system_name: Option<Cow<'static, str>>,
}

impl Default for RunMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl RunMeta {
    pub fn new() -> Self {
        Self {
            previous_system_name: None,
            next_system_name: None,
        }
    }
    pub fn with_previous_system(&self, previous_system_name: Cow<'static, str>) -> Self {
        Self {
            previous_system_name: Some(previous_system_name),
            next_system_name: self.next_system_name.clone(),
        }
    }
    pub fn with_next_system(&self, next_system_name: Cow<'static, str>) -> Self {
        Self {
            previous_system_name: self.previous_system_name.clone(),
            next_system_name: Some(next_system_name),
        }
    }
}
