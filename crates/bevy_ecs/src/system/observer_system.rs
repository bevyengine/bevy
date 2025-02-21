use alloc::{borrow::Cow, vec::Vec};
use core::marker::PhantomData;

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    prelude::{Bundle, Trigger},
    query::Access,
    result::Result,
    schedule::{Fallible, Infallible},
    system::{input::SystemIn, System},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use super::IntoSystem;

/// Implemented for [`System`]s that have a [`Trigger`] as the first argument.
pub trait ObserverSystem<E: 'static, B: Bundle, Out = Result>:
    System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static
{
}

impl<E: 'static, B: Bundle, Out, T> ObserverSystem<E, B, Out> for T where
    T: System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static
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
    note = "for function `ObserverSystem`s, ensure the first argument is a `Trigger<T>` and any subsequent ones are `SystemParam`"
)]
pub trait IntoObserverSystem<E: 'static, B: Bundle, M, Out = Result>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B, Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

impl<E, B, M, Out, S> IntoObserverSystem<E, B, (Fallible, M), Out> for S
where
    S: IntoSystem<Trigger<'static, E, B>, Out, M> + Send + 'static,
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
    S: IntoSystem<Trigger<'static, E, B>, (), M> + Send + 'static,
    S::System: ObserverSystem<E, B, ()>,
    E: Send + Sync + 'static,
    B: Bundle,
{
    type System = InfallibleObserverWrapper<E, B, S::System>;

    fn into_system(this: Self) -> Self::System {
        InfallibleObserverWrapper::new(IntoSystem::into_system(this))
    }
}

/// A wrapper that converts an observer system that returns `()` into one that returns `Ok(())`.
pub struct InfallibleObserverWrapper<E, B, S> {
    observer: S,
    _marker: PhantomData<(E, B)>,
}

impl<E, B, S> InfallibleObserverWrapper<E, B, S> {
    /// Create a new `InfallibleObserverWrapper`.
    pub fn new(observer: S) -> Self {
        Self {
            observer,
            _marker: PhantomData,
        }
    }
}

impl<E, B, S> System for InfallibleObserverWrapper<E, B, S>
where
    S: ObserverSystem<E, B, ()>,
    E: Send + Sync + 'static,
    B: Bundle,
{
    type In = Trigger<'static, E, B>;
    type Out = Result;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.observer.name()
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        self.observer.component_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.observer.archetype_component_access()
    }

    #[inline]
    fn is_send(&self) -> bool {
        self.observer.is_send()
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        self.observer.is_exclusive()
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        self.observer.has_deferred()
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

    #[inline]
    fn run(&mut self, input: SystemIn<'_, Self>, world: &mut World) -> Self::Out {
        self.observer.run(input, world);
        Ok(())
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
    unsafe fn validate_param_unsafe(&mut self, world: UnsafeWorldCell) -> bool {
        self.observer.validate_param_unsafe(world)
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.observer.initialize(world);
    }

    #[inline]
    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.observer.update_archetype_component_access(world);
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: Tick) {
        self.observer.check_change_tick(change_tick);
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
        observer::Trigger,
        system::{In, IntoSystem},
        world::World,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    fn test_piped_observer_systems_no_input() {
        fn a(_: Trigger<TriggerEvent>) {}
        fn b() {}

        let mut world = World::new();
        world.add_observer(a.pipe(b));
    }

    #[test]
    fn test_piped_observer_systems_with_inputs() {
        fn a(_: Trigger<TriggerEvent>) -> u32 {
            3
        }
        fn b(_: In<u32>) {}

        let mut world = World::new();
        world.add_observer(a.pipe(b));
    }
}
