use bevy_utils::prelude::DebugName;

use crate::{
    component::{CheckChangeTicks, Tick},
    error::Result,
    query::FilteredAccessSet,
    system::{input::SystemIn, BoxedSystem, RunSystemError, System, SystemInput},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FromWorld, World},
};

use super::{IntoSystem, SystemParamValidationError, SystemStateFlags};

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

    fn name(&self) -> DebugName {
        self.system.name()
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        self.system.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        self.system.run_unsafe(&mut self.value, world)
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.system.refresh_hotpatch();
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

    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        self.system.initialize(world)
    }

    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.system.check_change_tick(check);
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

    fn name(&self) -> DebugName {
        self.system.name()
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        self.system.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        let value = self
            .value
            .as_mut()
            .expect("System input value was not found. Did you forget to initialize the system before running it?");
        self.system.run_unsafe(value, world)
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.system.refresh_hotpatch();
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

    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        if self.value.is_none() {
            self.value = Some(T::from_world(world));
        }
        self.system.initialize(world)
    }

    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.system.check_change_tick(check);
    }

    fn get_last_run(&self) -> Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system.set_last_run(last_run);
    }
}

/// Type alias for a `BoxedSystem` that a `Schedule` can store.
pub type ScheduleSystem = BoxedSystem<(), ()>;
