use crate::{
    component::ComponentHooks,
    system::{IntoObserverSystem, ObserverSystem},
};

use super::*;

/// Type for function that is run when an observer is triggered
/// Typically refers to the default runner that runs the contained,
/// but can be overridden for custom behaviour.
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut);

/// Equivalent to [`BoxedSystem`](crate::system::BoxedSystem) for [`ObserverSystem`].
pub type BoxedObserverSystem<E = (), B = ()> = Box<dyn ObserverSystem<E, B>>;

pub(crate) struct ObserverComponent {
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) runner: ObserverRunner,
    pub(crate) system: Option<BoxedObserverSystem>,
    pub(crate) last_event_id: u32,
}

impl Component for ObserverComponent {
    type Storage = SparseStorage;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks
            .on_add(|mut world, entity, _| {
                world.commands().add(move |world: &mut World| {
                    world.register_observer(entity);
                });
            })
            .on_remove(|mut world, entity, _| {
                world.commands().add(move |world: &mut World| {
                    world.unregister_observer(entity);
                });
            });
    }
}

impl ObserverComponent {
    pub(crate) fn from<E: 'static, B: Bundle, M>(
        world: &mut World,
        descriptor: ObserverDescriptor,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> Self {
        let mut system = IntoObserverSystem::into_system(system);
        assert!(
            !system.is_exclusive(),
            "Cannot run exclusive systems in Observers"
        );
        system.initialize(world);
        let system: BoxedObserverSystem<E, B> = Box::new(system);
        Self {
            descriptor,
            runner: |mut world, trigger, ptr| {
                if trigger.source == Entity::PLACEHOLDER {
                    return;
                }
                let world = world.as_unsafe_world_cell();
                let observer_cell =
                // SAFETY: Observer was triggered so must still exist in world
                    unsafe { world.get_entity(trigger.observer).debug_checked_unwrap() };
                // SAFETY: Observer was triggered so must have an `ObserverComponent`
                let mut state = unsafe {
                    observer_cell
                        .get_mut::<ObserverComponent>()
                        .debug_checked_unwrap()
                };
                // SAFETY: We only access world metadata
                let last_event = unsafe { world.world_metadata() }.last_event_id;
                if state.last_event_id == last_event {
                    return;
                }
                state.last_event_id = last_event;

                // SAFETY: Caller ensures `ptr` is castable to `E`
                let observer: Observer<E, B> = Observer::new(unsafe { ptr.deref_mut() }, trigger);
                // SAFETY: System is from component
                let mut system: Box<dyn ObserverSystem<E, B>> = unsafe {
                    let system = state.system.take().debug_checked_unwrap();
                    std::mem::transmute(system)
                };

                system.update_archetype_component_access(world);
                // SAFETY:
                // - `update_archetype_component_access` was just called
                // - there are no outsanding references to world except a private component
                // - system is an `ObserverSystem` so won't mutate world beyond the access of a `DeferredWorld`
                // - system is the same type erased system from above
                unsafe {
                    system.run_unsafe(std::mem::transmute(observer), world);
                    system.queue_deferred(world.into_deferred());
                    state.system = Some(std::mem::transmute(system));
                }
            },
            last_event_id: 0,
            // SAFETY: Same layout
            system: Some(unsafe { std::mem::transmute(system) }),
        }
    }

    pub(crate) fn from_runner(descriptor: ObserverDescriptor, runner: ObserverRunner) -> Self {
        Self {
            descriptor,
            runner,
            last_event_id: 0,
            system: None,
        }
    }
}
