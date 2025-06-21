use alloc::vec::Vec;
use bevy_utils::prelude::DebugName;
use core::marker::PhantomData;

use crate::{
    component::{CheckChangeTicks, ComponentId, Tick},
    error::Result,
    never::Never,
    prelude::{Bundle, On},
    query::FilteredAccessSet,
    schedule::{Fallible, Infallible},
    system::{input::SystemIn, System},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use super::{IntoSystem, SystemParamValidationError};

/// Implemented for [`System`]s that have [`On`] as the first argument.
pub trait ObserverSystem<E: 'static, B: Bundle, Out = Result>:
    System<In = On<'static, E, B>, Out = Out> + Send + 'static
{
}

impl<E: 'static, B: Bundle, Out, T> ObserverSystem<E, B, Out> for T where
    T: System<In = On<'static, E, B>, Out = Out> + Send + 'static
{
}

/// Implemented for systems that convert into [`ObserverSystem`].
///
/// # Usage notes
///
/// This trait should only be used as a bound for trait implementations or as an
/// argument to a function. If an observer system needs to be returned from a
/// function or stored somewhere, use [`ObserverSystem`] instead of this trait.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot become an `ObserverSystem`",
    label = "the trait `IntoObserverSystem` is not implemented",
    note = "for function `ObserverSystem`s, ensure the first argument is `On<T>` and any subsequent ones are `SystemParam`"
)]
pub trait IntoObserverSystem<E: 'static, B: Bundle, M, Out = Result>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B, Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

impl<E, B, M, S, Out> IntoObserverSystem<E, B, (Fallible, M), Out> for S
where
    S: IntoSystem<On<'static, E, B>, Out, M> + Send + 'static,
    S::System: ObserverSystem<E, B, Out>,
    E: 'static,
    B: Bundle,
{
    type System = S::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

impl<E, B, M, S> IntoObserverSystem<E, B, (Infallible, M), Result> for S
where
    S: IntoSystem<On<'static, E, B>, (), M> + Send + 'static,
    S::System: ObserverSystem<E, B, ()>,
    E: Send + Sync + 'static,
    B: Bundle,
{
    type System = InfallibleObserverWrapper<E, B, S::System, ()>;

    fn into_system(this: Self) -> Self::System {
        InfallibleObserverWrapper::new(IntoSystem::into_system(this))
    }
}
impl<E, B, M, S> IntoObserverSystem<E, B, (Never, M), Result> for S
where
    S: IntoSystem<On<'static, E, B>, Never, M> + Send + 'static,
    E: Send + Sync + 'static,
    B: Bundle,
{
    type System = InfallibleObserverWrapper<E, B, S::System, Never>;

    fn into_system(this: Self) -> Self::System {
        InfallibleObserverWrapper::new(IntoSystem::into_system(this))
    }
}

/// A wrapper that converts an observer system that returns `()` into one that returns `Ok(())`.
pub struct InfallibleObserverWrapper<E, B, S, Out> {
    observer: S,
    _marker: PhantomData<(E, B, Out)>,
}

impl<E, B, S, Out> InfallibleObserverWrapper<E, B, S, Out> {
    /// Create a new `InfallibleObserverWrapper`.
    pub fn new(observer: S) -> Self {
        Self {
            observer,
            _marker: PhantomData,
        }
    }
}

impl<E, B, S, Out> System for InfallibleObserverWrapper<E, B, S, Out>
where
    S: ObserverSystem<E, B, Out>,
    E: Send + Sync + 'static,
    B: Bundle,
    Out: Send + Sync + 'static,
{
    type In = On<'static, E, B>;
    type Out = Result;

    #[inline]
    fn name(&self) -> DebugName {
        self.observer.name()
    }

    #[inline]
    fn flags(&self) -> super::SystemStateFlags {
        self.observer.flags()
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        self.observer.run_unsafe(input, world);
        Ok(())
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.observer.refresh_hotpatch();
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.observer.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.observer.queue_deferred(world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        self.observer.validate_param_unsafe(world)
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet<ComponentId> {
        self.observer.initialize(world)
    }

    #[inline]
    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.observer.check_change_tick(check);
    }

    #[inline]
    fn get_last_run(&self) -> Tick {
        self.observer.get_last_run()
    }

    #[inline]
    fn set_last_run(&mut self, last_run: Tick) {
        self.observer.set_last_run(last_run);
    }

    fn default_system_sets(&self) -> Vec<crate::schedule::InternedSystemSet> {
        self.observer.default_system_sets()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event::Event,
        observer::On,
        system::{In, IntoSystem},
        world::World,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    fn test_piped_observer_systems_no_input() {
        fn a(_: On<TriggerEvent>) {}
        fn b() {}

        let mut world = World::new();
        world.add_observer(a.pipe(b));
    }

    #[test]
    fn test_piped_observer_systems_with_inputs() {
        fn a(_: On<TriggerEvent>) -> u32 {
            3
        }
        fn b(_: In<u32>) {}

        let mut world = World::new();
        world.add_observer(a.pipe(b));
    }
}
