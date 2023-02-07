use bevy_utils::tracing::warn;
use core::fmt::Debug;

use crate::{
    archetype::ArchetypeComponentId, change_detection::MAX_CHANGE_AGE, component::ComponentId,
    query::Access, world::World,
};

use std::any::TypeId;
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
/// see [`IntoSystemConfig`](crate::schedule::IntoSystemConfig).
pub trait System: Send + Sync + 'static {
    /// The system's input. See [`In`](crate::system::In) for
    /// [`FunctionSystem`](crate::system::FunctionSystem)s.
    type In;
    /// The system's output.
    type Out;
    /// Returns the system's name.
    fn name(&self) -> Cow<'static, str>;
    /// Returns the [`TypeId`] of the underlying system type.
    fn type_id(&self) -> TypeId;
    /// Returns the system's component [`Access`].
    fn component_access(&self) -> &Access<ComponentId>;
    /// Returns the system's archetype component [`Access`].
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId>;
    /// Returns true if the system is [`Send`].
    fn is_send(&self) -> bool;

    /// Returns true if the system must be run exclusively.
    fn is_exclusive(&self) -> bool;

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
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out;
    /// Runs the system with the given input in the world.
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        self.update_archetype_component_access(world);
        // SAFETY: world and resources are exclusively borrowed
        unsafe { self.run_unsafe(input, world) }
    }
    fn apply_buffers(&mut self, world: &mut World);
    /// Initialize the system.
    fn initialize(&mut self, _world: &mut World);
    /// Update the system's archetype component [`Access`].
    fn update_archetype_component_access(&mut self, world: &World);
    fn check_change_tick(&mut self, change_tick: u32);
    /// Returns the system's default [system sets](crate::schedule::SystemSet).
    fn default_system_sets(&self) -> Vec<Box<dyn crate::schedule::SystemSet>> {
        Vec::new()
    }
    /// Gets the system's last change tick
    fn get_last_change_tick(&self) -> u32;
    /// Sets the system's last change tick
    /// # Warning
    /// This is a complex and error-prone operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    fn set_last_change_tick(&mut self, last_change_tick: u32);
}

/// [`System`] types that do not modify the [`World`] when run.
/// This is implemented for any systems whose parameters all implement [`ReadOnlySystemParam`].
///
/// [`ReadOnlySystemParam`]: crate::system::ReadOnlySystemParam
///
/// # Safety
///
/// This must only be implemented for system types which do not mutate the `World`.
pub unsafe trait ReadOnlySystem: System {}

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

impl Debug for dyn System<In = (), Out = ()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "System {}: {{{}}}", self.name(), {
            if self.is_send() {
                if self.is_exclusive() {
                    "is_send is_exclusive"
                } else {
                    "is_send"
                }
            } else if self.is_exclusive() {
                "is_exclusive"
            } else {
                ""
            }
        },)
    }
}
