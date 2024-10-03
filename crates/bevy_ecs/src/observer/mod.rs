//! Types for creating and storing [`Observer`]s

mod entity_observer;
mod runner;
mod trigger_event;

pub use runner::*;
pub use trigger_event::*;

use crate::{
    archetype::ArchetypeFlags,
    component::ComponentId,
    entity::EntityHashMap,
    observer::entity_observer::ObservedBy,
    prelude::*,
    system::IntoObserverSystem,
    world::{DeferredWorld, *},
};
use bevy_ptr::Ptr;
use bevy_utils::HashMap;
use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// Type containing triggered [`Event`] information for a given run of an [`Observer`]. This contains the
/// [`Event`] data itself. If it was triggered for a specific [`Entity`], it includes that as well. It also
/// contains event propagation information. See [`Trigger::propagate`] for more information.
pub struct Trigger<'w, E, B: Bundle = ()> {
    event: &'w mut E,
    propagate: &'w mut bool,
    trigger: ObserverTrigger,
    _marker: PhantomData<B>,
}

impl<'w, E, B: Bundle> Trigger<'w, E, B> {
    /// Creates a new trigger for the given event and observer information.
    pub fn new(event: &'w mut E, propagate: &'w mut bool, trigger: ObserverTrigger) -> Self {
        Self {
            event,
            propagate,
            trigger,
            _marker: PhantomData,
        }
    }

    /// Returns the event type of this trigger.
    pub fn event_type(&self) -> ComponentId {
        self.trigger.event_type
    }

    /// Returns a reference to the triggered event.
    pub fn event(&self) -> &E {
        self.event
    }

    /// Returns a mutable reference to the triggered event.
    pub fn event_mut(&mut self) -> &mut E {
        self.event
    }

    /// Returns a pointer to the triggered event.
    pub fn event_ptr(&self) -> Ptr {
        Ptr::from(&self.event)
    }

    /// Returns the [`Entity`] that triggered the observer, could be [`Entity::PLACEHOLDER`].
    pub fn entity(&self) -> Entity {
        self.trigger.entity
    }

    /// Returns the [`Entity`] that observed the triggered event.
    /// This allows you to despawn the observer, ceasing observation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::{Commands, Trigger};
    /// #
    /// # struct MyEvent {
    /// #   done: bool,
    /// # }
    /// #
    /// /// Handle `MyEvent` and if it is done, stop observation.
    /// fn my_observer(trigger: Trigger<MyEvent>, mut commands: Commands) {
    ///     if trigger.event().done {
    ///         commands.entity(trigger.observer()).despawn();
    ///         return;
    ///     }
    ///
    ///     // ...
    /// }
    /// ```
    pub fn observer(&self) -> Entity {
        self.trigger.observer
    }

    /// Enables or disables event propagation, allowing the same event to trigger observers on a chain of different entities.
    ///
    /// The path an event will propagate along is specified by its associated [`Traversal`] component. By default, events
    /// use `()` which ends the path immediately and prevents propagation.
    ///
    /// To enable propagation, you must:
    /// + Set [`Event::Traversal`] to the component you want to propagate along.
    /// + Either call `propagate(true)` in the first observer or set [`Event::AUTO_PROPAGATE`] to `true`.
    ///
    /// You can prevent an event from propagating further using `propagate(false)`.
    ///
    /// [`Traversal`]: crate::traversal::Traversal
    pub fn propagate(&mut self, should_propagate: bool) {
        *self.propagate = should_propagate;
    }

    /// Returns the value of the flag that controls event propagation. See [`propagate`] for more information.
    ///
    /// [`propagate`]: Trigger::propagate
    pub fn get_propagate(&self) -> bool {
        *self.propagate
    }
}

impl<'w, E: Debug, B: Bundle> Debug for Trigger<'w, E, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Trigger")
            .field("event", &self.event)
            .field("propagate", &self.propagate)
            .field("trigger", &self.trigger)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<'w, E, B: Bundle> Deref for Trigger<'w, E, B> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'w, E, B: Bundle> DerefMut for Trigger<'w, E, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.event
    }
}

/// A description of what an [`Observer`] observes.
#[derive(Default, Clone)]
pub struct ObserverDescriptor {
    /// The events the observer is watching.
    events: Vec<ComponentId>,

    /// The components the observer is watching.
    components: Vec<ComponentId>,

    /// The entities the observer is watching.
    entities: Vec<Entity>,
}

impl ObserverDescriptor {
    /// Add the given `events` to the descriptor.
    /// # Safety
    /// The type of each [`ComponentId`] in `events` _must_ match the actual value
    /// of the event passed into the observer.
    pub unsafe fn with_events(mut self, events: Vec<ComponentId>) -> Self {
        self.events = events;
        self
    }

    /// Add the given `components` to the descriptor.
    pub fn with_components(mut self, components: Vec<ComponentId>) -> Self {
        self.components = components;
        self
    }

    /// Add the given `entities` to the descriptor.
    pub fn with_entities(mut self, entities: Vec<Entity>) -> Self {
        self.entities = entities;
        self
    }

    pub(crate) fn merge(&mut self, descriptor: &ObserverDescriptor) {
        self.events.extend(descriptor.events.iter().copied());
        self.components
            .extend(descriptor.components.iter().copied());
        self.entities.extend(descriptor.entities.iter().copied());
    }
}

/// Event trigger metadata for a given [`Observer`],
#[derive(Debug)]
pub struct ObserverTrigger {
    /// The [`Entity`] of the observer handling the trigger.
    pub observer: Entity,

    /// The [`ComponentId`] the trigger targeted.
    pub event_type: ComponentId,

    /// The entity the trigger targeted.
    pub entity: Entity,
}

// Map between an observer entity and its runner
type ObserverMap = EntityHashMap<ObserverRunner>;

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular trigger targeted at a specific component.
#[derive(Default, Debug)]
pub struct CachedComponentObservers {
    // Observers listening to triggers targeting this component
    map: ObserverMap,
    // Observers listening to triggers targeting this component on a specific entity
    entity_map: EntityHashMap<ObserverMap>,
}

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular trigger.
#[derive(Default, Debug)]
pub struct CachedObservers {
    // Observers listening for any time this trigger is fired
    map: ObserverMap,
    // Observers listening for this trigger fired at a specific component
    component_observers: HashMap<ComponentId, CachedComponentObservers>,
    // Observers listening for this trigger fired at a specific entity
    entity_observers: EntityHashMap<ObserverMap>,
}

/// Metadata for observers. Stores a cache mapping trigger ids to the registered observers.
#[derive(Default, Debug)]
pub struct Observers {
    // Cached ECS observers to save a lookup most common triggers.
    on_add: CachedObservers,
    on_insert: CachedObservers,
    on_replace: CachedObservers,
    on_remove: CachedObservers,
    // Map from trigger type to set of observers
    cache: HashMap<ComponentId, CachedObservers>,
}

impl Observers {
    pub(crate) fn get_observers(&mut self, event_type: ComponentId) -> &mut CachedObservers {
        match event_type {
            ON_ADD => &mut self.on_add,
            ON_INSERT => &mut self.on_insert,
            ON_REPLACE => &mut self.on_replace,
            ON_REMOVE => &mut self.on_remove,
            _ => self.cache.entry(event_type).or_default(),
        }
    }

    pub(crate) fn try_get_observers(&self, event_type: ComponentId) -> Option<&CachedObservers> {
        match event_type {
            ON_ADD => Some(&self.on_add),
            ON_INSERT => Some(&self.on_insert),
            ON_REPLACE => Some(&self.on_replace),
            ON_REMOVE => Some(&self.on_remove),
            _ => self.cache.get(&event_type),
        }
    }

    /// This will run the observers of the given `event_type`, targeting the given `entity` and `components`.
    pub(crate) fn invoke<T>(
        mut world: DeferredWorld,
        event_type: ComponentId,
        entity: Entity,
        components: impl Iterator<Item = ComponentId>,
        data: &mut T,
        propagate: &mut bool,
    ) {
        // SAFETY: You cannot get a mutable reference to `observers` from `DeferredWorld`
        let (mut world, observers) = unsafe {
            let world = world.as_unsafe_world_cell();
            // SAFETY: There are no outstanding world references
            world.increment_trigger_id();
            let observers = world.observers();
            let Some(observers) = observers.try_get_observers(event_type) else {
                return;
            };
            // SAFETY: The only outstanding reference to world is `observers`
            (world.into_deferred(), observers)
        };

        let mut trigger_observer = |(&observer, runner): (&Entity, &ObserverRunner)| {
            (runner)(
                world.reborrow(),
                ObserverTrigger {
                    observer,
                    event_type,
                    entity,
                },
                data.into(),
                propagate,
            );
        };
        // Trigger observers listening for any kind of this trigger
        observers.map.iter().for_each(&mut trigger_observer);

        // Trigger entity observers listening for this kind of trigger
        if entity != Entity::PLACEHOLDER {
            if let Some(map) = observers.entity_observers.get(&entity) {
                map.iter().for_each(&mut trigger_observer);
            }
        }

        // Trigger observers listening to this trigger targeting a specific component
        components.for_each(|id| {
            if let Some(component_observers) = observers.component_observers.get(&id) {
                component_observers
                    .map
                    .iter()
                    .for_each(&mut trigger_observer);

                if entity != Entity::PLACEHOLDER {
                    if let Some(map) = component_observers.entity_map.get(&entity) {
                        map.iter().for_each(&mut trigger_observer);
                    }
                }
            }
        });
    }

    pub(crate) fn is_archetype_cached(event_type: ComponentId) -> Option<ArchetypeFlags> {
        match event_type {
            ON_ADD => Some(ArchetypeFlags::ON_ADD_OBSERVER),
            ON_INSERT => Some(ArchetypeFlags::ON_INSERT_OBSERVER),
            ON_REPLACE => Some(ArchetypeFlags::ON_REPLACE_OBSERVER),
            ON_REMOVE => Some(ArchetypeFlags::ON_REMOVE_OBSERVER),
            _ => None,
        }
    }

    pub(crate) fn update_archetype_flags(
        &self,
        component_id: ComponentId,
        flags: &mut ArchetypeFlags,
    ) {
        if self.on_add.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_ADD_OBSERVER);
        }

        if self
            .on_insert
            .component_observers
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_INSERT_OBSERVER);
        }

        if self
            .on_replace
            .component_observers
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_REPLACE_OBSERVER);
        }

        if self
            .on_remove
            .component_observers
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_REMOVE_OBSERVER);
        }
    }
}

impl World {
    /// Spawns a "global" [`Observer`] and returns its [`Entity`].
    pub fn observe<E: Event, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> EntityWorldMut {
        self.spawn(Observer::new(system))
    }

    /// Triggers the given [`Event`], which will run any [`Observer`]s watching for it.
    ///
    /// While event types commonly implement [`Copy`],
    /// those that don't will be consumed and will no longer be accessible.
    /// If you need to use the event after triggering it, use [`World::trigger_ref`] instead.
    pub fn trigger(&mut self, event: impl Event) {
        TriggerEvent { event, targets: () }.trigger(self);
    }

    /// Triggers the given [`Event`] as a mutable reference, which will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    pub fn trigger_ref(&mut self, event: &mut impl Event) {
        TriggerEvent { event, targets: () }.trigger_ref(self);
    }

    /// Triggers the given [`Event`] for the given `targets`, which will run any [`Observer`]s watching for it.
    ///
    /// While event types commonly implement [`Copy`],
    /// those that don't will be consumed and will no longer be accessible.
    /// If you need to use the event after triggering it, use [`World::trigger_targets_ref`] instead.
    pub fn trigger_targets(&mut self, event: impl Event, targets: impl TriggerTargets) {
        TriggerEvent { event, targets }.trigger(self);
    }

    /// Triggers the given [`Event`] as a mutable reference for the given `targets`,
    /// which will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger_targets`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    pub fn trigger_targets_ref(&mut self, event: &mut impl Event, targets: impl TriggerTargets) {
        TriggerEvent { event, targets }.trigger_ref(self);
    }

    /// Register an observer to the cache, called when an observer is created
    pub(crate) fn register_observer(&mut self, observer_entity: Entity) {
        // SAFETY: References do not alias.
        let (observer_state, archetypes, observers) = unsafe {
            let observer_state: *const ObserverState =
                self.get::<ObserverState>(observer_entity).unwrap();
            // Populate ObservedBy for each observed entity.
            for watched_entity in &(*observer_state).descriptor.entities {
                let mut entity_mut = self.entity_mut(*watched_entity);
                let mut observed_by = entity_mut.entry::<ObservedBy>().or_default();
                observed_by.0.push(observer_entity);
            }
            (&*observer_state, &mut self.archetypes, &mut self.observers)
        };
        let descriptor = &observer_state.descriptor;

        for &event_type in &descriptor.events {
            let cache = observers.get_observers(event_type);

            if descriptor.components.is_empty() && descriptor.entities.is_empty() {
                cache.map.insert(observer_entity, observer_state.runner);
            } else if descriptor.components.is_empty() {
                // Observer is not targeting any components so register it as an entity observer
                for &watched_entity in &observer_state.descriptor.entities {
                    let map = cache.entity_observers.entry(watched_entity).or_default();
                    map.insert(observer_entity, observer_state.runner);
                }
            } else {
                // Register observer for each watched component
                for &component in &descriptor.components {
                    let observers =
                        cache
                            .component_observers
                            .entry(component)
                            .or_insert_with(|| {
                                if let Some(flag) = Observers::is_archetype_cached(event_type) {
                                    archetypes.update_flags(component, flag, true);
                                }
                                CachedComponentObservers::default()
                            });
                    if descriptor.entities.is_empty() {
                        // Register for all triggers targeting the component
                        observers.map.insert(observer_entity, observer_state.runner);
                    } else {
                        // Register for each watched entity
                        for &watched_entity in &descriptor.entities {
                            let map = observers.entity_map.entry(watched_entity).or_default();
                            map.insert(observer_entity, observer_state.runner);
                        }
                    }
                }
            }
        }
    }

    /// Remove the observer from the cache, called when an observer gets despawned
    pub(crate) fn unregister_observer(&mut self, entity: Entity, descriptor: ObserverDescriptor) {
        let archetypes = &mut self.archetypes;
        let observers = &mut self.observers;

        for &event_type in &descriptor.events {
            let cache = observers.get_observers(event_type);
            if descriptor.components.is_empty() && descriptor.entities.is_empty() {
                cache.map.remove(&entity);
            } else if descriptor.components.is_empty() {
                for watched_entity in &descriptor.entities {
                    // This check should be unnecessary since this observer hasn't been unregistered yet
                    let Some(observers) = cache.entity_observers.get_mut(watched_entity) else {
                        continue;
                    };
                    observers.remove(&entity);
                    if observers.is_empty() {
                        cache.entity_observers.remove(watched_entity);
                    }
                }
            } else {
                for component in &descriptor.components {
                    let Some(observers) = cache.component_observers.get_mut(component) else {
                        continue;
                    };
                    if descriptor.entities.is_empty() {
                        observers.map.remove(&entity);
                    } else {
                        for watched_entity in &descriptor.entities {
                            let Some(map) = observers.entity_map.get_mut(watched_entity) else {
                                continue;
                            };
                            map.remove(&entity);
                            if map.is_empty() {
                                observers.entity_map.remove(watched_entity);
                            }
                        }
                    }

                    if observers.map.is_empty() && observers.entity_map.is_empty() {
                        cache.component_observers.remove(component);
                        if let Some(flag) = Observers::is_archetype_cached(event_type) {
                            if let Some(by_component) = archetypes.by_component.get(component) {
                                for archetype in by_component.keys() {
                                    let archetype = &mut archetypes.archetypes[archetype.index()];
                                    if archetype.contains(*component) {
                                        let no_longer_observed = archetype
                                            .components()
                                            .all(|id| !cache.component_observers.contains_key(&id));

                                        if no_longer_observed {
                                            archetype.flags.set(flag, false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use bevy_ptr::OwningPtr;

    use crate as bevy_ecs;
    use crate::{
        observer::{EmitDynamicTrigger, Observer, ObserverDescriptor, ObserverState, OnReplace},
        prelude::*,
        traversal::Traversal,
    };

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct S;

    #[derive(Event)]
    struct EventA;

    #[derive(Event)]
    struct EventWithData {
        counter: usize,
    }

    #[derive(Resource, Default)]
    struct Order(Vec<&'static str>);

    impl Order {
        #[track_caller]
        fn observed(&mut self, name: &'static str) {
            self.0.push(name);
        }
    }

    #[derive(Component)]
    struct Parent(Entity);

    impl Traversal for &'_ Parent {
        fn traverse(item: Self::Item<'_>) -> Option<Entity> {
            Some(item.0)
        }
    }

    #[derive(Component)]
    struct EventPropagating;

    impl Event for EventPropagating {
        type Traversal = &'static Parent;

        const AUTO_PROPAGATE: bool = true;
    }

    #[test]
    fn observer_order_spawn_despawn() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<Order>| res.observed("add"));
        world.observe(|_: Trigger<OnInsert, A>, mut res: ResMut<Order>| res.observed("insert"));
        world.observe(|_: Trigger<OnReplace, A>, mut res: ResMut<Order>| res.observed("replace"));
        world.observe(|_: Trigger<OnRemove, A>, mut res: ResMut<Order>| res.observed("remove"));

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(
            vec!["add", "insert", "replace", "remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_order_insert_remove() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<Order>| res.observed("add"));
        world.observe(|_: Trigger<OnInsert, A>, mut res: ResMut<Order>| res.observed("insert"));
        world.observe(|_: Trigger<OnReplace, A>, mut res: ResMut<Order>| res.observed("replace"));
        world.observe(|_: Trigger<OnRemove, A>, mut res: ResMut<Order>| res.observed("remove"));

        let mut entity = world.spawn_empty();
        entity.insert(A);
        entity.remove::<A>();
        entity.flush();
        assert_eq!(
            vec!["add", "insert", "replace", "remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_order_insert_remove_sparse() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(|_: Trigger<OnAdd, S>, mut res: ResMut<Order>| res.observed("add"));
        world.observe(|_: Trigger<OnInsert, S>, mut res: ResMut<Order>| res.observed("insert"));
        world.observe(|_: Trigger<OnReplace, S>, mut res: ResMut<Order>| res.observed("replace"));
        world.observe(|_: Trigger<OnRemove, S>, mut res: ResMut<Order>| res.observed("remove"));

        let mut entity = world.spawn_empty();
        entity.insert(S);
        entity.remove::<S>();
        entity.flush();
        assert_eq!(
            vec!["add", "insert", "replace", "remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_order_replace() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let entity = world.spawn(A).id();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<Order>| res.observed("add"));
        world.observe(|_: Trigger<OnInsert, A>, mut res: ResMut<Order>| res.observed("insert"));
        world.observe(|_: Trigger<OnReplace, A>, mut res: ResMut<Order>| res.observed("replace"));
        world.observe(|_: Trigger<OnRemove, A>, mut res: ResMut<Order>| res.observed("remove"));

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();

        let mut entity = world.entity_mut(entity);
        entity.insert(A);
        entity.flush();
        assert_eq!(vec!["replace", "insert"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_order_recursive() {
        let mut world = World::new();
        world.init_resource::<Order>();
        world.observe(
            |obs: Trigger<OnAdd, A>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("add_a");
                commands.entity(obs.entity()).insert(B);
            },
        );
        world.observe(
            |obs: Trigger<OnRemove, A>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("remove_a");
                commands.entity(obs.entity()).remove::<B>();
            },
        );

        world.observe(
            |obs: Trigger<OnAdd, B>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("add_b");
                commands.entity(obs.entity()).remove::<A>();
            },
        );
        world.observe(|_: Trigger<OnRemove, B>, mut res: ResMut<Order>| {
            res.observed("remove_b");
        });

        let entity = world.spawn(A).flush();
        let entity = world.get_entity(entity).unwrap();
        assert!(!entity.contains::<A>());
        assert!(!entity.contains::<B>());
        assert_eq!(
            vec!["add_a", "add_b", "remove_a", "remove_b"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_trigger_ref() {
        let mut world = World::new();

        world.observe(|mut trigger: Trigger<EventWithData>| trigger.event_mut().counter += 1);
        world.observe(|mut trigger: Trigger<EventWithData>| trigger.event_mut().counter += 2);
        world.observe(|mut trigger: Trigger<EventWithData>| trigger.event_mut().counter += 4);
        // This flush is required for the last observer to be called when triggering the event,
        // due to `World::observe` returning `WorldEntityMut`.
        world.flush();

        let mut event = EventWithData { counter: 0 };
        world.trigger_ref(&mut event);
        assert_eq!(7, event.counter);
    }

    #[test]
    fn observer_trigger_targets_ref() {
        let mut world = World::new();

        world.observe(|mut trigger: Trigger<EventWithData, A>| trigger.event_mut().counter += 1);
        world.observe(|mut trigger: Trigger<EventWithData, B>| trigger.event_mut().counter += 2);
        world.observe(|mut trigger: Trigger<EventWithData, A>| trigger.event_mut().counter += 4);
        // This flush is required for the last observer to be called when triggering the event,
        // due to `World::observe` returning `WorldEntityMut`.
        world.flush();

        let mut event = EventWithData { counter: 0 };
        let component_a = world.register_component::<A>();
        world.trigger_targets_ref(&mut event, component_a);
        assert_eq!(5, event.counter);
    }

    #[test]
    fn observer_multiple_listeners() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<Order>| res.observed("add_1"));
        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<Order>| res.observed("add_2"));

        world.spawn(A).flush();
        assert_eq!(vec!["add_1", "add_2"], world.resource::<Order>().0);
        // Our A entity plus our two observers
        assert_eq!(world.entities().len(), 3);
    }

    #[test]
    fn observer_multiple_events() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let on_remove = world.register_component::<OnRemove>();
        world.spawn(
            // SAFETY: OnAdd and OnRemove are both unit types, so this is safe
            unsafe {
                Observer::new(|_: Trigger<OnAdd, A>, mut res: ResMut<Order>| {
                    res.observed("add/remove");
                })
                .with_event(on_remove)
            },
        );

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(
            vec!["add/remove", "add/remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_multiple_components() {
        let mut world = World::new();
        world.init_resource::<Order>();
        world.register_component::<A>();
        world.register_component::<B>();

        world.observe(|_: Trigger<OnAdd, (A, B)>, mut res: ResMut<Order>| res.observed("add_ab"));

        let entity = world.spawn(A).id();
        world.entity_mut(entity).insert(B);
        world.flush();
        assert_eq!(vec!["add_ab", "add_ab"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_despawn() {
        let mut world = World::new();

        let observer = world
            .observe(|_: Trigger<OnAdd, A>| panic!("Observer triggered after being despawned."))
            .id();
        world.despawn(observer);
        world.spawn(A).flush();
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/14961
    #[test]
    fn observer_despawn_archetype_flags() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let entity = world.spawn((A, B)).flush();

        world.observe(|_: Trigger<OnRemove, A>, mut res: ResMut<Order>| res.observed("remove_a"));

        let observer = world
            .observe(|_: Trigger<OnRemove, B>| panic!("Observer triggered after being despawned."))
            .flush();
        world.despawn(observer);

        world.despawn(entity);

        assert_eq!(vec!["remove_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_multiple_matches() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(|_: Trigger<OnAdd, (A, B)>, mut res: ResMut<Order>| res.observed("add_ab"));

        world.spawn((A, B)).flush();
        assert_eq!(vec!["add_ab"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_no_target() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventA>| panic!("Trigger routed to non-targeted entity."));
        world.observe(move |obs: Trigger<EventA>, mut res: ResMut<Order>| {
            assert_eq!(obs.entity(), Entity::PLACEHOLDER);
            res.observed("event_a");
        });

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger(EventA);
        world.flush();
        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_entity_routing() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventA>| panic!("Trigger routed to non-targeted entity."));
        let entity = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventA>, mut res: ResMut<Order>| res.observed("a_1"))
            .id();
        world.observe(move |obs: Trigger<EventA>, mut res: ResMut<Order>| {
            assert_eq!(obs.entity(), entity);
            res.observed("a_2");
        });

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventA, entity);
        world.flush();
        assert_eq!(vec!["a_2", "a_1"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_dynamic_component() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let component_id = world.register_component::<A>();
        world.spawn(
            Observer::new(|_: Trigger<OnAdd>, mut res: ResMut<Order>| res.observed("event_a"))
                .with_component(component_id),
        );

        let mut entity = world.spawn_empty();
        OwningPtr::make(A, |ptr| {
            // SAFETY: we registered `component_id` above.
            unsafe { entity.insert_by_id(component_id, ptr) };
        });
        let entity = entity.flush();

        world.trigger_targets(EventA, entity);
        world.flush();
        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_dynamic_trigger() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let event_a = world.register_component::<EventA>();

        world.spawn(ObserverState {
            // SAFETY: we registered `event_a` above and it matches the type of TriggerA
            descriptor: unsafe { ObserverDescriptor::default().with_events(vec![event_a]) },
            runner: |mut world, _trigger, _ptr, _propagate| {
                world.resource_mut::<Order>().observed("event_a");
            },
            ..Default::default()
        });

        world.commands().queue(
            // SAFETY: we registered `event_a` above and it matches the type of TriggerA
            unsafe { EmitDynamicTrigger::new_with_id(event_a, EventA, ()) },
        );
        world.flush();
        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(Parent(parent))
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
            })
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, child);
        world.flush();
        assert_eq!(vec!["child", "parent"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_redundant_dispatch_same_entity() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(Parent(parent))
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
            })
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, [child, child]);
        world.flush();
        assert_eq!(
            vec!["child", "parent", "child", "parent"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_redundant_dispatch_parent_child() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(Parent(parent))
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
            })
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, [child, parent]);
        world.flush();
        assert_eq!(
            vec!["child", "parent", "parent"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_halt() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(Parent(parent))
            .observe_entity(
                |mut trigger: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                    res.observed("child");
                    trigger.propagate(false);
                },
            )
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, child);
        world.flush();
        assert_eq!(vec!["child"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_join() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child_a = world
            .spawn(Parent(parent))
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_a");
            })
            .id();

        let child_b = world
            .spawn(Parent(parent))
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_b");
            })
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, [child_a, child_b]);
        world.flush();
        assert_eq!(
            vec!["child_a", "parent", "child_b", "parent"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_no_next() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let entity = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("event");
            })
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, entity);
        world.flush();
        assert_eq!(vec!["event"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_parallel_propagation() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent_a = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent_a");
            })
            .id();

        let child_a = world
            .spawn(Parent(parent_a))
            .observe_entity(
                |mut trigger: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                    res.observed("child_a");
                    trigger.propagate(false);
                },
            )
            .id();

        let parent_b = world
            .spawn_empty()
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent_b");
            })
            .id();

        let child_b = world
            .spawn(Parent(parent_b))
            .observe_entity(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_b");
            })
            .id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, [child_a, child_b]);
        world.flush();
        assert_eq!(
            vec!["child_a", "child_b", "parent_b"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_world() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(|_: Trigger<EventPropagating>, mut res: ResMut<Order>| res.observed("event"));

        let grandparent = world.spawn_empty().id();
        let parent = world.spawn(Parent(grandparent)).id();
        let child = world.spawn(Parent(parent)).id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, child);
        world.flush();
        assert_eq!(vec!["event", "event", "event"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_world_skipping() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.observe(
            |trigger: Trigger<EventPropagating>, query: Query<&A>, mut res: ResMut<Order>| {
                if query.get(trigger.entity()).is_ok() {
                    res.observed("event");
                }
            },
        );

        let grandparent = world.spawn(A).id();
        let parent = world.spawn(Parent(grandparent)).id();
        let child = world.spawn((A, Parent(parent))).id();

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventPropagating, child);
        world.flush();
        assert_eq!(vec!["event", "event"], world.resource::<Order>().0);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/14467
    // Fails prior to https://github.com/bevyengine/bevy/pull/15398
    #[test]
    fn observer_on_remove_during_despawn_spawn_empty() {
        let mut world = World::new();

        // Observe the removal of A - this will run during despawn
        world.observe(|_: Trigger<OnRemove, A>, mut cmd: Commands| {
            // Spawn a new entity - this reserves a new ID and requires a flush
            // afterward before Entities::free can be called.
            cmd.spawn_empty();
        });

        let ent = world.spawn(A).id();

        // Despawn our entity, which runs the OnRemove observer and allocates a
        // new Entity.
        // Should not panic - if it does, then Entities was not flushed properly
        // after the observer's spawn_empty.
        world.despawn(ent);
    }

    #[test]
    fn observer_invalid_params() {
        #[derive(Event)]
        struct EventA;

        #[derive(Resource)]
        struct ResA;

        #[derive(Resource)]
        struct ResB;

        let mut world = World::new();
        // This fails because `ResA` is not present in the world
        world.observe(|_: Trigger<EventA>, _: Res<ResA>, mut commands: Commands| {
            commands.insert_resource(ResB);
        });
        world.trigger(EventA);

        assert!(world.get_resource::<ResB>().is_none());
    }

    #[test]
    fn observer_apply_deferred_from_param_set() {
        #[derive(Event)]
        struct EventA;

        #[derive(Resource)]
        struct ResA;

        let mut world = World::new();
        world.observe(
            |_: Trigger<EventA>, mut params: ParamSet<(Query<Entity>, Commands)>| {
                params.p1().insert_resource(ResA);
            },
        );
        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger(EventA);
        world.flush();

        assert!(world.get_resource::<ResA>().is_some());
    }
}
