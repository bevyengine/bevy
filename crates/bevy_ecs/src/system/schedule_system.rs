use alloc::borrow::Cow;

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    query::Access,
    result::Result,
    system::{input::SystemIn, BoxedSystem, System},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use super::IntoSystem;

// This is breaking some things because the system type set is now wrong.
/// Wrapper for a system that returns `()` to return `Ok(())` to make it into a [`ScheduleSystem`]
pub struct OkWrapperSystem<S: System<In = (), Out = ()>>(S);

impl<S: System<In = (), Out = ()>> OkWrapperSystem<S> {
    /// Create a new `OkWrapperSystem`
    pub fn new(system: S) -> Self {
        Self(IntoSystem::into_system(system))
    }
}

impl<S: System<In = (), Out = ()>> System for OkWrapperSystem<S> {
    type In = ();
    type Out = Result;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        self.0.component_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.0.archetype_component_access()
    }

    #[inline]
    fn is_send(&self) -> bool {
        self.0.is_send()
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        self.0.is_exclusive()
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        self.0.has_deferred()
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        self.0.run_unsafe(input, world);
        Ok(())
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.0.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.0.queue_deferred(world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(&mut self, world: UnsafeWorldCell) -> bool {
        self.0.validate_param_unsafe(world)
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.0.initialize(world);
    }

    #[inline]
    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.0.update_archetype_component_access(world);
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: Tick) {
        self.0.check_change_tick(change_tick);
    }

    #[inline]
    fn get_last_run(&self) -> Tick {
        self.0.get_last_run()
    }

    #[inline]
    fn set_last_run(&mut self, last_run: Tick) {
        self.0.set_last_run(last_run);
    }
}

/// Type alias for a `BoxedSystem` that a `Schedule` can store.
pub type ScheduleSystem = BoxedSystem<(), Result>;
