use alloc::vec::Vec;
use bevy_utils::prelude::DebugName;

use super::{IntoSystem, ReadOnlySystem, RunSystemError, System, SystemParamValidationError};
use crate::{
    schedule::InternedSystemSet,
    system::{input::SystemInput, SystemIn},
    world::unsafe_world_cell::UnsafeWorldCell,
};

/// Customizes the behavior of an [`AdapterSystem`]
///
/// # Examples
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::{Adapt, AdapterSystem, RunSystemError};
///
/// // A system adapter that inverts the result of a system.
/// // NOTE: Instead of manually implementing this, you can just use `bevy_ecs::schedule::common_conditions::not`.
/// pub type NotSystem<S> = AdapterSystem<NotMarker, S>;
///
/// // This struct is used to customize the behavior of our adapter.
/// pub struct NotMarker;
///
/// impl<S> Adapt<S> for NotMarker
/// where
///     S: System,
///     S::Out: std::ops::Not,
/// {
///     type In = S::In;
///     type Out = <S::Out as std::ops::Not>::Output;
///
///     fn adapt(
///         &mut self,
///         input: <Self::In as SystemInput>::Inner<'_>,
///         run_system: impl FnOnce(SystemIn<'_, S>) -> Result<S::Out, RunSystemError>,
///     ) -> Result<Self::Out, RunSystemError> {
///         let result = run_system(input)?;
///         Ok(!result)
///     }
/// }
/// # let mut world = World::new();
/// # let mut system = NotSystem::new(NotMarker, IntoSystem::into_system(|| false), "".into());
/// # system.initialize(&mut world);
/// # assert!(system.run((), &mut world).unwrap());
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not adapt a system of type `{S}`",
    label = "invalid system adapter"
)]
pub trait Adapt<S: System>: Send + Sync + 'static {
    /// The [input](System::In) type for an [`AdapterSystem`].
    type In: SystemInput;
    /// The [output](System::Out) type for an [`AdapterSystem`].
    type Out;

    /// When used in an [`AdapterSystem`], this function customizes how the system
    /// is run and how its inputs/outputs are adapted.
    fn adapt(
        &mut self,
        input: <Self::In as SystemInput>::Inner<'_>,
        run_system: impl FnOnce(SystemIn<'_, S>) -> Result<S::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError>;
}

/// An [`IntoSystem`] creating an instance of [`AdapterSystem`].
#[derive(Clone)]
pub struct IntoAdapterSystem<Func, S> {
    func: Func,
    system: S,
}

impl<Func, S> IntoAdapterSystem<Func, S> {
    /// Creates a new [`IntoSystem`] that uses `func` to adapt `system`, via the [`Adapt`] trait.
    pub const fn new(func: Func, system: S) -> Self {
        Self { func, system }
    }
}

#[doc(hidden)]
pub struct IsAdapterSystemMarker;

impl<Func, S, I, O, M> IntoSystem<Func::In, Func::Out, (IsAdapterSystemMarker, I, O, M)>
    for IntoAdapterSystem<Func, S>
where
    Func: Adapt<S::System>,
    I: SystemInput,
    S: IntoSystem<I, O, M>,
{
    type System = AdapterSystem<Func, S::System>;

    // Required method
    fn into_system(this: Self) -> Self::System {
        let system = IntoSystem::into_system(this.system);
        let name = system.name();
        AdapterSystem::new(this.func, system, name)
    }
}

/// A [`System`] that takes the output of `S` and transforms it by applying `Func` to it.
#[derive(Clone)]
pub struct AdapterSystem<Func, S> {
    func: Func,
    system: S,
    name: DebugName,
}

impl<Func, S> AdapterSystem<Func, S>
where
    Func: Adapt<S>,
    S: System,
{
    /// Creates a new [`System`] that uses `func` to adapt `system`, via the [`Adapt`] trait.
    pub const fn new(func: Func, system: S, name: DebugName) -> Self {
        Self { func, system, name }
    }
}

impl<Func, S> System for AdapterSystem<Func, S>
where
    Func: Adapt<S>,
    S: System,
{
    type In = Func::In;
    type Out = Func::Out;

    fn name(&self) -> DebugName {
        self.name.clone()
    }

    #[inline]
    fn flags(&self) -> super::SystemStateFlags {
        self.system.flags()
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        // SAFETY: `system.run_unsafe` has the same invariants as `self.run_unsafe`.
        self.func.adapt(input, |input| unsafe {
            self.system.run_unsafe(input, world)
        })
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.system.refresh_hotpatch();
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut crate::prelude::World) {
        self.system.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: crate::world::DeferredWorld) {
        self.system.queue_deferred(world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Delegate to other `System` implementations.
        unsafe { self.system.validate_param_unsafe(world) }
    }

    fn initialize(&mut self, world: &mut crate::prelude::World) -> crate::query::FilteredAccessSet {
        self.system.initialize(world)
    }

    fn check_change_tick(&mut self, check: crate::component::CheckChangeTicks) {
        self.system.check_change_tick(check);
    }

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        self.system.default_system_sets()
    }

    fn get_last_run(&self) -> crate::component::Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: crate::component::Tick) {
        self.system.set_last_run(last_run);
    }
}

// SAFETY: The inner system is read-only.
unsafe impl<Func, S> ReadOnlySystem for AdapterSystem<Func, S>
where
    Func: Adapt<S>,
    S: ReadOnlySystem,
{
}

impl<F, S, Out> Adapt<S> for F
where
    F: Send + Sync + 'static + FnMut(S::Out) -> Out,
    S: System,
{
    type In = S::In;
    type Out = Out;

    fn adapt(
        &mut self,
        input: <Self::In as SystemInput>::Inner<'_>,
        run_system: impl FnOnce(SystemIn<'_, S>) -> Result<S::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        run_system(input).map(self)
    }
}
