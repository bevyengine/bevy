use crate::{
    component::{ComponentHooks, ComponentId, StorageType},
    observer::{ObserverDescriptor, ObserverTrigger},
    prelude::*,
    query::DebugCheckedUnwrap,
    system::{IntoObserverSystem, ObserverSystem},
    world::DeferredWorld,
};
use bevy_ptr::PtrMut;

/// Contains [`Observer`] information. This defines how a given observer behaves. It is the
/// "source of truth" for a given observer entity's behavior.
pub struct ObserverState {
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) runner: ObserverRunner,
    pub(crate) last_trigger_id: u32,
    pub(crate) despawned_sources: u32,
}

impl Default for ObserverState {
    fn default() -> Self {
        Self {
            runner: |_, _, _| {},
            last_trigger_id: 0,
            despawned_sources: 0,
            descriptor: Default::default(),
        }
    }
}

impl ObserverState {
    /// Adds the given `event`
    pub fn with_event(mut self, event: ComponentId) -> Self {
        self.descriptor.events.push(event);
        self
    }

    /// Adds the given `events`
    pub fn with_events(mut self, events: impl IntoIterator<Item = ComponentId>) -> Self {
        self.descriptor.events.extend(events);
        self
    }

    /// Adds the given [`Entity`] `sources`
    pub fn with_sources(mut self, sources: impl IntoIterator<Item = Entity>) -> Self {
        self.descriptor.sources.extend(sources);
        self
    }

    /// Adds the given [`ComponentId`] `components`
    pub fn with_components(mut self, components: impl IntoIterator<Item = ComponentId>) -> Self {
        self.descriptor.components.extend(components);
        self
    }
}

impl Component for ObserverState {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            world.commands().add(move |world: &mut World| {
                world.register_observer(entity);
            });
        });
        hooks.on_remove(|mut world, entity, _| {
            let descriptor = std::mem::take(
                &mut world
                    .entity_mut(entity)
                    .get_mut::<ObserverState>()
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

/// Type for function that is run when an observer is triggered.
/// Typically refers to the default runner that runs the system stored in the associated [`ObserverSystemComponent`],
/// but can be overridden for custom behaviour.
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut);

/// An [`Observer`] system. Add this [`Component`] to an [`Entity`] to give it observer behaviors for the given system.
pub struct Observer<T: 'static, B: Bundle> {
    system: BoxedObserverSystem<T, B>,
    descriptor: ObserverDescriptor,
}

impl<E: Event, B: Bundle> Observer<E, B> {
    /// Creates a new [`Observer`], which defaults to a "global" observer.
    pub fn new<M>(system: impl IntoObserverSystem<E, B, M>) -> Self {
        Self {
            system: Box::new(IntoObserverSystem::into_system(system)),
            descriptor: Default::default(),
        }
    }

    /// Observe the given `entity`.
    pub fn with_source(mut self, entity: Entity) -> Self {
        self.descriptor.sources.push(entity);
        self
    }

    /// Observe the given `entity`. Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn add_source(&mut self, entity: Entity) {
        self.descriptor.sources.push(entity);
    }

    /// Observe the given `component`.
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.descriptor.components.push(component);
        self
    }

    /// Observe the given [`trigger`].
    pub fn with_event(mut self, event: ComponentId) -> Self {
        self.descriptor.events.push(event);
        self
    }
}

impl<E: Event, B: Bundle> Component for Observer<E, B> {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            world.commands().add(move |world: &mut World| {
                let event_type = world.init_component::<E>();
                let mut components = Vec::new();
                B::component_ids(&mut world.components, &mut world.storages, &mut |id| {
                    components.push(id);
                });
                let mut descriptor = ObserverDescriptor {
                    events: vec![event_type],
                    components,
                    ..Default::default()
                };

                // Initialize System
                let system: *mut dyn ObserverSystem<E, B> =
                    if let Some(mut observe) = world.get_mut::<Self>(entity) {
                        descriptor.merge(&observe.descriptor);
                        &mut *observe.system
                    } else {
                        return;
                    };
                // SAFETY: World reference is exclusive and initialize does not touch system, so references do not alias
                unsafe {
                    (*system).initialize(world);
                }

                {
                    let mut entity = world.entity_mut(entity);
                    if let crate::world::Entry::Vacant(entry) = entity.entry::<ObserverState>() {
                        entry.insert(ObserverState {
                            descriptor,
                            runner: observer_system_runner::<E, B>,
                            ..Default::default()
                        });
                    }
                }
            });
        });
    }
}

/// Equivalent to [`BoxedSystem`](crate::system::BoxedSystem) for [`ObserverSystem`].
pub type BoxedObserverSystem<E = (), B = ()> = Box<dyn ObserverSystem<E, B>>;

fn observer_system_runner<E: Event, B: Bundle>(
    mut world: DeferredWorld,
    observer_trigger: ObserverTrigger,
    ptr: PtrMut,
) {
    let world = world.as_unsafe_world_cell();
    let observer_cell =
    // SAFETY: Observer was triggered so must still exist in world
        unsafe { world.get_entity(observer_trigger.observer).debug_checked_unwrap() };
    // SAFETY: Observer was triggered so must have an `ObserverState`
    let mut state = unsafe {
        observer_cell
            .get_mut::<ObserverState>()
            .debug_checked_unwrap()
    };

    // TODO: Move this check into the observer cache to avoid dynamic dispatch
    // SAFETY: We only access world metadata
    let last_trigger = unsafe { world.world_metadata() }.last_trigger_id();
    if state.last_trigger_id == last_trigger {
        return;
    }
    state.last_trigger_id = last_trigger;

    let trigger: Trigger<E, B> =
        // SAFETY: Caller ensures `ptr` is castable to `&mut T`
            Trigger::new(unsafe { ptr.deref_mut() }, observer_trigger);
    // SAFETY: the static lifetime is encapsulated in Trigger / cannot leak out.
    // Additionally, IntoObserverSystem is only implemented for functions starting
    // with for<'a> Trigger<'a>, meaning users cannot specify Trigger<'static> manually,
    // allowing the Trigger<'static> to be moved outside of the context of the system.
    // This transmute is obviously not ideal, but it is safe. Ideally we can remove the
    // static constraint from ObserverSystem, but so far we have not found a way.
    let trigger: Trigger<'static, E, B> = unsafe { std::mem::transmute(trigger) };
    // SAFETY: Observer was triggered so must have an `ObserverSystemComponent`
    let system = unsafe {
        &mut observer_cell
            .get_mut::<Observer<E, B>>()
            .debug_checked_unwrap()
            .system
    };

    system.update_archetype_component_access(world);

    // SAFETY:
    // - `update_archetype_component_access` was just called
    // - there are no outstanding references to world except a private component
    // - system is an `ObserverSystem` so won't mutate world beyond the access of a `DeferredWorld`
    // - system is the same type erased system from above
    unsafe {
        system.run_unsafe(trigger, world);
        system.queue_deferred(world.into_deferred());
    }
}
