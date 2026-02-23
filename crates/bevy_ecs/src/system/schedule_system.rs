use bevy_utils::prelude::DebugName;

use crate::{
    change_detection::{CheckChangeTicks, Tick},
    error::Result,
    query::FilteredAccessSet,
    system::{input::SystemIn, BoxedSystem, RunSystemError, System, SystemInput},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FromWorld, World},
};

use super::{IntoSystem, SystemParamValidationError, SystemStateFlags};

// ---------------------------------------------------------------------------
// WithInputWrapper  (existing — InMut-style: Inner<'i> = &'i mut T)
// ---------------------------------------------------------------------------

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
        // SAFETY: Upheld by caller.
        unsafe { self.system.run_unsafe(&mut self.value, world) }
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
        // SAFETY: Upheld by caller.
        unsafe { self.system.validate_param_unsafe(world) }
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

// ---------------------------------------------------------------------------
// WithRefInputWrapper  (new — InRef-style: Inner<'i> = &'i T)
// ---------------------------------------------------------------------------

/// Wraps a system whose input is an [`InRef<T>`](crate::system::InRef), storing
/// an owned `T` and lending a shared reference to it on each run.
///
/// Constructed via [`IntoSystem::with_input_ref`].
///
/// This lets multiple systems share the same conceptual read-only value without
/// needing to pipe a value through a chain.
///
/// # Example
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// fn print_entity(InRef(entity): InRef<Entity>) {
///     println!("entity: {entity:?}");
/// }
///
/// let entity = Entity::from_raw(42);
/// schedule.add_systems(print_entity.with_input_ref(entity));
/// ```
pub struct WithRefInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i T>>,
    T: Send + Sync + 'static,
{
    system: S,
    value: T,
}

impl<S, T> WithRefInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i T>>,
    T: Send + Sync + 'static,
{
    /// Wraps the given system with the given input value.
    ///
    /// The value is stored inside the wrapper and a shared reference to it is
    /// passed to the system on every run.
    pub fn new<M>(system: impl IntoSystem<S::In, S::Out, M, System = S>, value: T) -> Self {
        Self {
            system: IntoSystem::into_system(system),
            value,
        }
    }

    /// Returns a shared reference to the stored input value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the stored input value.
    ///
    /// Mutating the value here is reflected in subsequent system runs.
    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<S, T> System for WithRefInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = &'i T>>,
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
        // SAFETY: Upheld by caller.
        // We pass a shared reference; the system cannot mutate the stored value,
        // so this is safe even when multiple read-only systems run in parallel
        // (the scheduler already ensures no aliasing with write access).
        unsafe { self.system.run_unsafe(&self.value, world) }
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
        // SAFETY: Upheld by caller.
        unsafe { self.system.validate_param_unsafe(world) }
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

// ---------------------------------------------------------------------------
// WithClonedInputWrapper  (new — In-style: Inner<'i> = T, requires T: Clone)
// ---------------------------------------------------------------------------

/// Wraps a system whose input is an [`In<T>`](crate::system::In), storing an
/// owned `T` and cloning it on each run to produce the by-value input.
///
/// Constructed via [`IntoSystem::with_cloned_input`].
///
/// Because `In<T>` transfers *ownership* of the value into the system, the
/// stored `T` must be [`Clone`] so the system can run more than once.  If the
/// system only runs once (e.g. `Startup`) you can call `.clone()` at the
/// call site and pass the clone to avoid the bound:
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// fn spawn_creature(In(entity): In<Entity>, mut commands: Commands) {
///     commands.entity(entity).insert(Name::new("Creature"));
/// }
///
/// let entity = Entity::from_raw(42);
/// // Entity is Copy, so Clone is trivially satisfied.
/// schedule.add_systems(spawn_creature.with_cloned_input(entity));
/// ```
pub struct WithClonedInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = T>>,
    T: Clone + Send + Sync + 'static,
{
    system: S,
    value: T,
}

impl<S, T> WithClonedInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = T>>,
    T: Clone + Send + Sync + 'static,
{
    /// Wraps the given system with the given input value.
    ///
    /// On every run, the stored value is cloned and the clone is passed by
    /// value to the system.
    pub fn new<M>(system: impl IntoSystem<S::In, S::Out, M, System = S>, value: T) -> Self {
        Self {
            system: IntoSystem::into_system(system),
            value,
        }
    }

    /// Returns a shared reference to the stored input value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the stored input value.
    ///
    /// Replacing the value here affects what is cloned on the next run.
    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<S, T> System for WithClonedInputWrapper<S, T>
where
    for<'i> S: System<In: SystemInput<Inner<'i> = T>>,
    T: Clone + Send + Sync + 'static,
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
        // Clone the stored value to produce the by-value input for this run.
        // The clone happens here (inside the wrapper), keeping the stored
        // original intact for future runs.
        let input = self.value.clone();
        // SAFETY: Upheld by caller.
        unsafe { self.system.run_unsafe(input, world) }
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
        // SAFETY: Upheld by caller.
        unsafe { self.system.validate_param_unsafe(world) }
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

// ---------------------------------------------------------------------------
// WithInputFromWrapper  (existing — unchanged)
// ---------------------------------------------------------------------------

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
        // SAFETY: Upheld by caller.
        unsafe { self.system.run_unsafe(value, world) }
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
        // SAFETY: Upheld by caller.
        unsafe { self.system.validate_param_unsafe(world) }
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
