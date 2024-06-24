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
    pub(crate) despawned_watched_entities: u32,
}

impl Default for ObserverState {
    fn default() -> Self {
        Self {
            runner: |_, _, _| {},
            last_trigger_id: 0,
            despawned_watched_entities: 0,
            descriptor: Default::default(),
        }
    }
}

impl ObserverState {
    /// Observe the given `event`. This will cause the [`Observer`] to run whenever an event with the given [`ComponentId`]
    /// is triggered.
    pub fn with_event(mut self, event: ComponentId) -> Self {
        self.descriptor.events.push(event);
        self
    }

    /// Observe the given event list. This will cause the [`Observer`] to run whenever an event with any of the given [`ComponentId`]s
    /// is triggered.
    pub fn with_events(mut self, events: impl IntoIterator<Item = ComponentId>) -> Self {
        self.descriptor.events.extend(events);
        self
    }

    /// Observe the given [`Entity`] list. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for any [`Entity`] target in the list.
    pub fn with_entities(mut self, entities: impl IntoIterator<Item = Entity>) -> Self {
        self.descriptor.entities.extend(entities);
        self
    }

    /// Observe the given [`ComponentId`] list. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for any [`ComponentId`] target in the list.
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

/// An [`Observer`] system. Add this [`Component`] to an [`Entity`] to turn it into an "observer".
///
/// Observers listen for a "trigger" of a specific [`Event`]. Events are triggered by calling [`World::trigger`] or [`World::trigger_targets`].
///
/// Note that "buffered" events sent using [`EventReader`] and [`EventWriter`] are _not_ automatically triggered. They must be triggered at a specific
/// point in the schedule.
///
/// # Usage
///
/// The simplest usage
/// of the observer pattern looks like this:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// #[derive(Event)]
/// struct Speak {
///     message: String,
/// }
///
/// world.observe(|trigger: Trigger<Speak>| {
///     println!("{}", trigger.event().message);
/// });
///
/// // Observers currently require a flush() to be registered. In the context of schedules,
/// // this will generally be done for you.
/// world.flush();
///
/// world.trigger(Speak {
///     message: "Hello!".into(),
/// });
/// ```
///
/// Notice that we used [`World::observe`]. This is just a shorthand for spawning an [`Observer`] manually:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct Speak;
/// // These are functionally the same:
/// world.observe(|trigger: Trigger<Speak>| {});
/// world.spawn(Observer::new(|trigger: Trigger<Speak>| {}));
/// ```
///
/// Observers are systems. They can access arbitrary [`World`] data by adding [`SystemParam`]s:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct PrintNames;
/// # #[derive(Component, Debug)]
/// # struct Name;
/// world.observe(|trigger: Trigger<PrintNames>, names: Query<&Name>| {
///     for name in &names {
///         println!("{name:?}");
///     }
/// });
/// ```
///
/// Note that [`Trigger`] must always be the first parameter.
///
/// You can also add [`Commands`], which means you can spawn new entities, insert new components, etc:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct SpawnThing;
/// # #[derive(Component, Debug)]
/// # struct Thing;
/// world.observe(|trigger: Trigger<SpawnThing>, mut commands: Commands| {
///     commands.spawn(Thing);
/// });
/// ```
///
/// Observers can also trigger new events:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct A;
/// # #[derive(Event)]
/// # struct B;
/// world.observe(|trigger: Trigger<A>, mut commands: Commands| {
///     commands.trigger(B);
/// });
/// ```
///
/// When the commands are flushed (including these "nested triggers") they will be
/// recursively evaluated until there are no commands left, meaning nested triggers all
/// evaluate at the same time!
///
/// Events can be triggered for entities, which will be passed to the [`Observer`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// #[derive(Event)]
/// struct Explode;
///
/// world.observe(|trigger: Trigger<Explode>, mut commands: Commands| {
///     println!("Entity {:?} goes BOOM!", trigger.entity());
///     commands.entity(trigger.entity()).despawn();
/// });
///
/// world.flush();
///
/// world.trigger_targets(Explode, entity);
/// ```
///
/// You can trigger multiple entities at once:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let e1 = world.spawn_empty().id();
/// # let e2 = world.spawn_empty().id();
/// # #[derive(Event)]
/// # struct Explode;
/// world.trigger_targets(Explode, [e1, e2]);
/// ```
///
/// Observers can also watch _specific_ entities, which enables you to assign entity-specific logic:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component, Debug)]
/// # struct Name(String);
/// # let mut world = World::default();
/// # let e1 = world.spawn_empty().id();
/// # let e2 = world.spawn_empty().id();
/// # #[derive(Event)]
/// # struct Explode;
/// world.entity_mut(e1).observe(|trigger: Trigger<Explode>, mut commands: Commands| {
///     println!("Boom!");
///     commands.entity(trigger.entity()).despawn();
/// });
///
/// world.entity_mut(e2).observe(|trigger: Trigger<Explode>, mut commands: Commands| {
///     println!("The explosion fizzles! This entity is immune!");
/// });
/// ```
///
/// If all entities watched by a given [`Observer`] are despawned, the [`Observer`] entity will also be despawned.
/// This protects against observer "garbage" building up over time.
///
/// The examples above calling [`EntityWorldMut::observe`] to add entity-specific observer logic are (once again)
/// just shorthand for spawning an [`Observer`] directly:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// # #[derive(Event)]
/// # struct Explode;
/// let mut observer = Observer::new(|trigger: Trigger<Explode>| {});
/// observer.watch_entity(entity);
/// world.spawn(observer);
/// ```
///
/// Note that the [`Observer`] component is not added to the entity it is observing. Observers should always be their own entities!
///
/// You can call [`Observer::watch_entity`] more than once, which allows you to watch multiple entities with the same [`Observer`].
///
/// When first added, [`Observer`] will also create an [`ObserverState`] component, which registers the observer with the [`World`] and
/// serves as the "source of truth" of the observer.
///
/// [`SystemParam`]: crate::system::SystemParam
pub struct Observer<T: 'static, B: Bundle> {
    system: BoxedObserverSystem<T, B>,
    descriptor: ObserverDescriptor,
}

impl<E: Event, B: Bundle> Observer<E, B> {
    /// Creates a new [`Observer`], which defaults to a "global" observer. This means it will run whenever the event `E` is triggered
    /// for _any_ entity (or no entity).
    pub fn new<M>(system: impl IntoObserverSystem<E, B, M>) -> Self {
        Self {
            system: Box::new(IntoObserverSystem::into_system(system)),
            descriptor: Default::default(),
        }
    }

    /// Observe the given `entity`. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for the `entity`.
    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.descriptor.entities.push(entity);
        self
    }

    /// Observe the given `entity`. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for the `entity`.
    /// Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn watch_entity(&mut self, entity: Entity) {
        self.descriptor.entities.push(entity);
    }

    /// Observe the given `component`. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// with the given component target.
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.descriptor.components.push(component);
        self
    }

    /// Observe the given `event`. This will cause the [`Observer`] to run whenever an event with the given [`ComponentId`]
    /// is triggered.
    /// # Safety
    /// The type of the `event` [`ComponentId`] _must_ match the actual value
    /// of the event passed into the observer system.
    pub unsafe fn with_event(mut self, event: ComponentId) -> Self {
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
    // SAFETY: Observer was triggered so must still exist in world
    let observer_cell = unsafe {
        world
            .get_entity(observer_trigger.observer)
            .debug_checked_unwrap()
    };
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

    // SAFETY: Caller ensures `ptr` is castable to `&mut T`
    let trigger: Trigger<E, B> = Trigger::new(unsafe { ptr.deref_mut() }, observer_trigger);
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
