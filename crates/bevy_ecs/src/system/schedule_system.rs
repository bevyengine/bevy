use alloc::{borrow::Cow, vec::Vec};

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    error::Result,
    query::{Access, FilteredAccessSet},
    system::{input::SystemIn, BoxedSystem, System, SystemInput},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FromWorld, World},
};

use super::{IntoSystem, SystemParamValidationError};

/// A wrapper system to change a system that returns `()` to return `Ok(())` to make it into a [`ScheduleSystem`]
pub struct InfallibleSystemWrapper<S: System<In = ()>>(S);

impl<S: System<In = ()>> InfallibleSystemWrapper<S> {
    /// Create a new `OkWrapperSystem`
    pub fn new(system: S) -> Self {
        Self(IntoSystem::into_system(system))
    }
}

impl<S: System<In = ()>> System for InfallibleSystemWrapper<S> {
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
    fn component_access_set(&self) -> &FilteredAccessSet<ComponentId> {
        self.0.component_access_set()
    }

    #[inline(always)]
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
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
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

    fn default_system_sets(&self) -> Vec<crate::schedule::InternedSystemSet> {
        self.0.default_system_sets()
    }
}

/// See [`IntoSystem::with_input`] for details.
pub struct WithInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i mut T>>,
    T: Send + Sync + 'static,
{
    system: S,
    value: T,
}

impl<S, T> WithInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i mut T>>,
    T: Send + Sync + 'static,
{
    /// Wraps the given system with the given input value.
    pub fn new<M>(system: impl IntoSystem<S::In, S::Out, M, System = S>, value: T) -> Self {
        Self {
            system: IntoSystem::into_system(system),
            value,
        }
    }

    /// Returns a reference to the input value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the input value.
    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<S, T> System for WithInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i mut T>>,
    T: Send + Sync + 'static,
{
    type In = ();

    type Out = S::Out;

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.system.component_access()
    }

    fn component_access_set(&self) -> &FilteredAccessSet<ComponentId> {
        self.system.component_access_set()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.system.archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.system.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.system.is_exclusive()
    }

    fn has_deferred(&self) -> bool {
        self.system.has_deferred()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        self.system.run_unsafe(&mut self.value, world)
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.system.apply_deferred(world);
    }

    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.system.queue_deferred(world);
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        self.system.validate_param_unsafe(world)
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.system.update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.system.check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system.set_last_run(last_run);
    }
}

/// Constructed in [`IntoSystem::with_input_from`].
pub struct WithInputFromWrapper<S, T> {
    system: S,
    value: Option<T>,
}

impl<S, T> WithInputFromWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i mut T>>,
    T: Send + Sync + 'static,
{
    /// Wraps the given system.
    pub fn new<M>(system: impl IntoSystem<S::In, S::Out, M, System = S>) -> Self {
        Self {
            system: IntoSystem::into_system(system),
            value: None,
        }
    }

    /// Returns a reference to the input value, if it has been initialized.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Returns a mutable reference to the input value, if it has been initialized.
    pub fn value_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }
}

impl<S, T> System for WithInputFromWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i mut T>>,
    T: FromWorld + Send + Sync + 'static,
{
    type In = ();

    type Out = S::Out;

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.system.component_access()
    }

    fn component_access_set(&self) -> &FilteredAccessSet<ComponentId> {
        self.system.component_access_set()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.system.archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.system.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.system.is_exclusive()
    }

    fn has_deferred(&self) -> bool {
        self.system.has_deferred()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        let value = self
            .value
            .as_mut()
            .expect("System input value was not found. Did you forget to initialize the system before running it?");
        self.system.run_unsafe(value, world)
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.system.apply_deferred(world);
    }

    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.system.queue_deferred(world);
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        self.system.validate_param_unsafe(world)
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
        if self.value.is_none() {
            self.value = Some(T::from_world(world));
        }
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.system.update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.system.check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system.set_last_run(last_run);
    }
}

/// Type alias for a `BoxedSystem` that a `Schedule` can store.
pub type ScheduleSystem = BoxedSystem<(), Result>;
