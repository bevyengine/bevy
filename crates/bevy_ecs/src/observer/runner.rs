use crate as bevy_ecs;
use crate::{
    component::{ComponentHooks, StorageType},
    system::ObserverSystem,
};

use super::*;

/// Type for function that is run when an observer is triggered.
/// Typically refers to the default runner that runs the system stored in the associated [`ObserverSystemComponent`],
/// but can be overridden for custom behaviour.
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut);

/// Equivalent to [`BoxedSystem`](crate::system::BoxedSystem) for [`ObserverSystem`].
pub type BoxedObserverSystem<T = (), B = ()> = Box<dyn ObserverSystem<T, B>>;

pub(crate) struct ObserverComponent {
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) runner: ObserverRunner,
    pub(crate) last_trigger_id: u32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
// This used to be in `ObserverComponent` but MIRI recently got a new lint that complained about the type erasure
pub(crate) struct ObserverSystemComponent<T: 'static, B: Bundle>(BoxedObserverSystem<T, B>);

impl Component for ObserverComponent {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let descriptor = std::mem::take(
                &mut world
                    .entity_mut(entity)
                    .get_mut::<ObserverComponent>()
                    .unwrap()
                    .as_mut()
                    .descriptor,
            );
            world.commands().add(move |world: &mut World| {
                world.unregister_observer(entity, descriptor);
            });
        });
    }
}

impl ObserverComponent {
    pub(crate) fn from<T: 'static, B: Bundle, M>(
        world: &mut World,
        descriptor: ObserverDescriptor,
        system: impl IntoObserverSystem<T, B, M>,
    ) -> (Self, ObserverSystemComponent<T, B>) {
        let mut system = IntoObserverSystem::into_system(system);
        assert!(
            !system.is_exclusive(),
            "Cannot run exclusive systems in Observers"
        );
        system.initialize(world);
        let system: BoxedObserverSystem<T, B> = Box::new(system);
        (
            Self {
                descriptor,
                runner: |mut world, trigger, ptr| {
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

                    // TODO: Move this check into the observer cache to avoid dynamic dispatch
                    // SAFETY: We only access world metadata
                    let last_trigger = unsafe { world.world_metadata() }.last_trigger_id();
                    if state.last_trigger_id == last_trigger {
                        return;
                    }
                    state.last_trigger_id = last_trigger;

                    let observer: Observer<T, B> =
                    // SAFETY: Caller ensures `ptr` is castable to `&mut T`
                        Observer::new(unsafe { ptr.deref_mut() }, trigger);
                    // SAFETY: Observer was triggered so must have an `ObserverSystemComponent`
                    let system = unsafe {
                        &mut observer_cell
                            .get_mut::<ObserverSystemComponent<T, B>>()
                            .debug_checked_unwrap()
                            .0
                    };

                    system.update_archetype_component_access(world);

                    // SAFETY:
                    // - `update_archetype_component_access` was just called
                    // - there are no outstanding references to world except a private component
                    // - system is an `ObserverSystem` so won't mutate world beyond the access of a `DeferredWorld`
                    // - system is the same type erased system from above
                    unsafe {
                        system.run_unsafe(std::mem::transmute(observer), world);
                        system.queue_deferred(world.into_deferred());
                    }
                },
                last_trigger_id: 0,
            },
            ObserverSystemComponent(system),
        )
    }

    pub(crate) fn from_runner(descriptor: ObserverDescriptor, runner: ObserverRunner) -> Self {
        Self {
            descriptor,
            runner,
            last_trigger_id: 0,
        }
    }
}
