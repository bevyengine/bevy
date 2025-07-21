//! Logic for evaluating observers, and storing functions inside of observers.

use core::any::Any;

use crate::{
    error::ErrorContext,
    observer::ObserverTrigger,
    prelude::*,
    query::DebugCheckedUnwrap,
    system::{ObserverSystem, RunSystemError},
    world::DeferredWorld,
};
use bevy_ptr::PtrMut;

/// Type for function that is run when an observer is triggered.
///
/// Typically refers to the default runner that runs the system stored in the associated [`Observer`] component,
/// but can be overridden for custom behavior.
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut, propagate: &mut bool);

pub(super) fn observer_system_runner<E: Event, B: Bundle, S: ObserverSystem<E, B>>(
    mut world: DeferredWorld,
    observer_trigger: ObserverTrigger,
    ptr: PtrMut,
    propagate: &mut bool,
) {
    let world = world.as_unsafe_world_cell();
    // SAFETY: Observer was triggered so must still exist in world
    let observer_cell = unsafe {
        world
            .get_entity(observer_trigger.observer)
            .debug_checked_unwrap()
    };
    // SAFETY: Observer was triggered so must have an `Observer`
    let mut state = unsafe { observer_cell.get_mut::<Observer>().debug_checked_unwrap() };

    // TODO: Move this check into the observer cache to avoid dynamic dispatch
    let last_trigger = world.last_trigger_id();
    if state.last_trigger_id == last_trigger {
        return;
    }
    state.last_trigger_id = last_trigger;

    let trigger: On<E, B> = On::new(
        // SAFETY: Caller ensures `ptr` is castable to `&mut T`
        unsafe { ptr.deref_mut() },
        propagate,
        observer_trigger,
    );

    // SAFETY:
    // - observer was triggered so must have an `Observer` component.
    // - observer cannot be dropped or mutated until after the system pointer is already dropped.
    let system: *mut dyn ObserverSystem<E, B> = unsafe {
        let system: &mut dyn Any = state.system.as_mut();
        let system = system.downcast_mut::<S>().debug_checked_unwrap();
        &mut *system
    };

    // SAFETY:
    // - there are no outstanding references to world except a private component
    // - system is an `ObserverSystem` so won't mutate world beyond the access of a `DeferredWorld`
    //   and is never exclusive
    // - system is the same type erased system from above
    unsafe {
        #[cfg(feature = "hotpatching")]
        if world
            .get_resource_ref::<crate::HotPatchChanges>()
            .map(|r| {
                r.last_changed()
                    .is_newer_than((*system).get_last_run(), world.change_tick())
            })
            .unwrap_or(true)
        {
            (*system).refresh_hotpatch();
        };

        if let Err(RunSystemError::Failed(err)) = (*system)
            .validate_param_unsafe(world)
            .map_err(From::from)
            .and_then(|()| (*system).run_unsafe(trigger, world))
        {
            let handler = state
                .error_handler
                .unwrap_or_else(|| world.default_error_handler());
            handler(
                err,
                ErrorContext::Observer {
                    name: (*system).name(),
                    last_run: (*system).get_last_run(),
                },
            );
        };
        (*system).queue_deferred(world.into_deferred());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::{ignore, DefaultErrorHandler},
        event::Event,
        observer::On,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    #[should_panic(expected = "I failed!")]
    fn test_fallible_observer() {
        fn system(_: On<TriggerEvent>) -> Result {
            Err("I failed!".into())
        }

        let mut world = World::default();
        world.add_observer(system);
        Schedule::default().run(&mut world);
        world.trigger(TriggerEvent);
    }

    #[test]
    fn test_fallible_observer_ignored_errors() {
        #[derive(Resource, Default)]
        struct Ran(bool);

        fn system(_: On<TriggerEvent>, mut ran: ResMut<Ran>) -> Result {
            ran.0 = true;
            Err("I failed!".into())
        }

        // Using observer error handler
        let mut world = World::default();
        world.init_resource::<Ran>();
        world.spawn(Observer::new(system).with_error_handler(ignore));
        world.trigger(TriggerEvent);
        assert!(world.resource::<Ran>().0);

        // Using world error handler
        let mut world = World::default();
        world.init_resource::<Ran>();
        world.spawn(Observer::new(system));
        // Test that the correct handler is used when the observer was added
        // before the default handler
        world.insert_resource(DefaultErrorHandler(ignore));
        world.trigger(TriggerEvent);
        assert!(world.resource::<Ran>().0);
    }

    #[test]
    #[should_panic]
    fn exclusive_system_cannot_be_observer() {
        fn system(_: On<TriggerEvent>, _world: &mut World) {}
        let mut world = World::default();
        world.add_observer(system);
    }
}
