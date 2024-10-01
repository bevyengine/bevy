use alloc::sync::Arc;
use core::marker::PhantomData;
use std::sync::Mutex;

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    event::Event,
    prelude::{Bundle, Trigger, World},
    query::Access,
    system::System,
    world::{unsafe_world_cell::UnsafeWorldCell, Command, DeferredWorld},
};

use super::{IntoSystem, SystemIn};

/// Implemented for [`System`]s that have a [`Trigger`] as the first argument.
pub trait ObserverSystem<E: Event, B: Bundle, Out = ()>:
    System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static
{
}

impl<
        E: Event,
        B: Bundle,
        Out,
        T: System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static,
    > ObserverSystem<E, B, Out> for T
{
}

/// Implemented for systems that convert into [`ObserverSystem`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot become an `ObserverSystem`",
    label = "the trait `IntoObserverSystem` is not implemented",
    note = "for function `ObserverSystem`s, ensure the first argument is a `Trigger<T>` and any subsequent ones are `SystemParam`"
)]
pub trait IntoObserverSystem<E: Event, B: Bundle, M, Out = ()>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B, Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

#[doc(hidden)]
pub struct ObserverMarker;

impl<E, B, Out, Marker, S> IntoObserverSystem<E, B, (ObserverMarker, Marker), Out> for S
where
    E: Event,
    B: Bundle,
    S: IntoSystem<Trigger<'static, E, B>, Out, Marker, System: ObserverSystem<E, B, Out>>
        + Send
        + 'static,
{
    type System = S::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

#[doc(hidden)]
pub struct ExclusiveObserverMarker;

impl<E, B, Marker, S> IntoObserverSystem<E, B, (ExclusiveObserverMarker, Marker), ()> for S
where
    E: Event,
    B: Bundle,
    S: IntoSystem<(), (), Marker, System: System<In = (), Out = ()>> + Send + 'static,
{
    type System = ExclusiveObserverWrapper<E, B, S::System>;

    fn into_system(this: Self) -> Self::System {
        ExclusiveObserverWrapper {
            _marker: PhantomData,
            system: Arc::new(Mutex::new(IntoSystem::into_system(this))),
        }
    }
}

/// A [`System`] wrapper type that queues the held system for execution.
pub struct ExclusiveObserverWrapper<E: Event, B: Bundle, S: System<In = (), Out = ()>> {
    _marker: PhantomData<(E, B)>,
    system: Arc<Mutex<S>>,
}

impl<E: Event, B: Bundle, S: System<In = (), Out = ()>> System
    for ExclusiveObserverWrapper<E, B, S>
{
    type In = Trigger<'static, E, B>;
    type Out = ();

    fn name(&self) -> alloc::borrow::Cow<'static, str> {
        self.system.lock().unwrap().name()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        // The wrapper doesn't access any components.
        const { &Access::new() }
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        // The wrapper doesn't access any components.
        const { &Access::new() }
    }

    fn is_send(&self) -> bool {
        self.system.lock().unwrap().is_send()
    }

    fn is_exclusive(&self) -> bool {
        // The wrapper only queues the system, it doesn't run it directly.
        false
    }

    fn has_deferred(&self) -> bool {
        // The wrapper queues the system to run later.
        true
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        _world: UnsafeWorldCell,
    ) -> Self::Out {
        // The wrapped system is queued, not run directly.
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.system.lock().unwrap().run((), world);
    }

    fn queue_deferred(&mut self, mut world: DeferredWorld) {
        let system = Arc::clone(&self.system);
        world.commands().queue(RunSystemArc(system));
    }

    unsafe fn validate_param_unsafe(&self, world: UnsafeWorldCell) -> bool {
        self.system.lock().unwrap().validate_param_unsafe(world)
    }

    fn validate_param(&mut self, world: &World) -> bool {
        self.system.lock().unwrap().validate_param(world)
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.lock().unwrap().initialize(world);
    }

    fn update_archetype_component_access(&mut self, _world: UnsafeWorldCell) {
        // The wrapper doesn't access any components.
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        // TODO: is this correct?
        self.system.lock().unwrap().check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> Tick {
        // TODO: is this correct?
        self.system.lock().unwrap().get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        // TODO: is this correct?
        self.system.lock().unwrap().set_last_run(last_run);
    }
}

struct RunSystemArc<S: System<In = (), Out = ()>>(Arc<Mutex<S>>);

impl<S: System<In = (), Out = ()>> Command for RunSystemArc<S> {
    fn apply(self, world: &mut World) {
        self.0.lock().unwrap().run((), world);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        event::Event,
        observer::Trigger,
        system::{In, IntoSystem, ResMut, Resource},
        world::World,
    };

    #[derive(Resource)]
    struct Foo(pub u32);

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    fn test_piped_observer_systems_no_input() {
        fn a(_: Trigger<TriggerEvent>) {}
        fn b() {}

        let mut world = World::new();
        world.observe(a.pipe(b));
    }

    #[test]
    fn test_piped_observer_systems_with_inputs() {
        fn a(_: Trigger<TriggerEvent>) -> u32 {
            3
        }
        fn b(_: In<u32>) {}

        let mut world = World::new();
        world.observe(a.pipe(b));
    }

    #[test]
    fn test_exclusive() {
        fn foo(world: &mut World) {
            world.get_resource_or_insert_with(|| Foo(0)).0 += 1;
        }

        let mut world = World::new();
        world.observe::<TriggerEvent, (), _>(foo);
        world.flush();

        assert!(world.get_resource::<Foo>().is_none());
        world.trigger(TriggerEvent);
        world.flush();

        assert_eq!(world.get_resource::<Foo>().unwrap().0, 1);
    }

    #[test]
    fn test_nonexclusive() {
        fn foo(mut foo: ResMut<Foo>) {
            foo.0 += 1;
        }

        let mut world = World::new();
        world.insert_resource(Foo(0));
        world.observe::<TriggerEvent, (), _>(foo);
        world.flush();

        assert_eq!(world.get_resource::<Foo>().unwrap().0, 0);
        world.trigger(TriggerEvent);
        world.flush();

        assert_eq!(world.get_resource::<Foo>().unwrap().0, 1);
    }
}
