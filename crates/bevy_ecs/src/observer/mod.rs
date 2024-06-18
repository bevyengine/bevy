//! Types for creating and storing [`Observer`]s

mod entity_observer;
mod runner;
mod trigger_event;

pub use runner::*;
pub use trigger_event::*;

use crate::observer::entity_observer::ObservedBy;
use crate::{archetype::ArchetypeFlags, system::IntoObserverSystem, world::*};
use crate::{component::ComponentId, prelude::*, world::DeferredWorld};
use bevy_ptr::Ptr;
use bevy_utils::{EntityHashMap, HashMap};
use std::marker::PhantomData;

/// Type containing triggered [`Event`] information for a given run of an [`Observer`]. This contains the
/// [`Event`] data itself. If it was triggered for a specific [`Entity`], it includes that as well.
pub struct Trigger<'w, E, B: Bundle = ()> {
    event: &'w mut E,
    trigger: ObserverTrigger,
    _marker: PhantomData<B>,
}

impl<'w, E, B: Bundle> Trigger<'w, E, B> {
    /// Creates a new trigger for the given event and observer information.
    pub fn new(event: &'w mut E, trigger: ObserverTrigger) -> Self {
        Self {
            event,
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

    /// Returns the entity that triggered the observer, could be [`Entity::PLACEHOLDER`].
    pub fn entity(&self) -> Entity {
        self.trigger.entity
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
    /// Add the given `triggers` to the descriptor.
    pub fn with_triggers(mut self, triggers: Vec<ComponentId>) -> Self {
        self.events = triggers;
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
type ObserverMap = EntityHashMap<Entity, ObserverRunner>;

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular trigger targeted at a specific component.
#[derive(Default, Debug)]
pub struct CachedComponentObservers {
    // Observers listening to triggers targeting this component
    map: ObserverMap,
    // Observers listening to triggers targeting this component on a specific entity
    entity_map: EntityHashMap<Entity, ObserverMap>,
}

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular trigger.
#[derive(Default, Debug)]
pub struct CachedObservers {
    // Observers listening for any time this trigger is fired
    map: ObserverMap,
    // Observers listening for this trigger fired at a specific component
    component_observers: HashMap<ComponentId, CachedComponentObservers>,
    // Observers listening for this trigger fired at a specific entity
    entity_observers: EntityHashMap<Entity, ObserverMap>,
}

/// Metadata for observers. Stores a cache mapping trigger ids to the registered observers.
#[derive(Default, Debug)]
pub struct Observers {
    // Cached ECS observers to save a lookup most common triggers.
    on_add: CachedObservers,
    on_insert: CachedObservers,
    on_remove: CachedObservers,
    // Map from trigger type to set of observers
    cache: HashMap<ComponentId, CachedObservers>,
}

impl Observers {
    pub(crate) fn get_observers(&mut self, event_type: ComponentId) -> &mut CachedObservers {
        match event_type {
            ON_ADD => &mut self.on_add,
            ON_INSERT => &mut self.on_insert,
            ON_REMOVE => &mut self.on_remove,
            _ => self.cache.entry(event_type).or_default(),
        }
    }

    pub(crate) fn try_get_observers(&self, event_type: ComponentId) -> Option<&CachedObservers> {
        match event_type {
            ON_ADD => Some(&self.on_add),
            ON_INSERT => Some(&self.on_insert),
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
            .on_remove
            .component_observers
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_REMOVE_OBSERVER);
        }
    }
}

impl World {
    /// Spawn a "global" [`Observer`] and returns it's [`Entity`].
    pub fn observe<E: Event, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> EntityWorldMut {
        self.spawn(Observer::new(system))
    }

    /// Triggers the given `event`, which will run any observers watching for it.
    pub fn trigger(&mut self, event: impl Event) {
        TriggerEvent { event, targets: () }.apply(self);
    }

    /// Triggers the given `event` for the given `targets`, which will run any observers watching for it.
    pub fn trigger_targets(&mut self, event: impl Event, targets: impl TriggerTargets) {
        TriggerEvent { event, targets }.apply(self);
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
                            archetypes.update_flags(*component, flag, false);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ptr::OwningPtr;

    use crate as bevy_ecs;
    use crate::observer::{EmitDynamicTrigger, Observer, ObserverDescriptor, ObserverState};
    use crate::prelude::*;

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

    #[derive(Resource, Default)]
    struct R(usize);

    impl R {
        #[track_caller]
        fn assert_order(&mut self, count: usize) {
            assert_eq!(count, self.0);
            self.0 += 1;
        }
    }

    #[test]
    fn observer_order_spawn_despawn() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<R>| res.assert_order(0));
        world.observe(|_: Trigger<OnInsert, A>, mut res: ResMut<R>| res.assert_order(1));
        world.observe(|_: Trigger<OnRemove, A>, mut res: ResMut<R>| res.assert_order(2));

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(3, world.resource::<R>().0);
    }

    #[test]
    fn observer_order_insert_remove() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<R>| res.assert_order(0));
        world.observe(|_: Trigger<OnInsert, A>, mut res: ResMut<R>| res.assert_order(1));
        world.observe(|_: Trigger<OnRemove, A>, mut res: ResMut<R>| res.assert_order(2));

        let mut entity = world.spawn_empty();
        entity.insert(A);
        entity.remove::<A>();
        entity.flush();
        assert_eq!(3, world.resource::<R>().0);
    }

    #[test]
    fn observer_order_insert_remove_sparse() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observe(|_: Trigger<OnAdd, S>, mut res: ResMut<R>| res.assert_order(0));
        world.observe(|_: Trigger<OnInsert, S>, mut res: ResMut<R>| res.assert_order(1));
        world.observe(|_: Trigger<OnRemove, S>, mut res: ResMut<R>| res.assert_order(2));

        let mut entity = world.spawn_empty();
        entity.insert(S);
        entity.remove::<S>();
        entity.flush();
        assert_eq!(3, world.resource::<R>().0);
    }

    #[test]
    fn observer_order_recursive() {
        let mut world = World::new();
        world.init_resource::<R>();
        world.observe(
            |obs: Trigger<OnAdd, A>, mut res: ResMut<R>, mut commands: Commands| {
                res.assert_order(0);
                commands.entity(obs.entity()).insert(B);
            },
        );
        world.observe(
            |obs: Trigger<OnRemove, A>, mut res: ResMut<R>, mut commands: Commands| {
                res.assert_order(2);
                commands.entity(obs.entity()).remove::<B>();
            },
        );

        world.observe(
            |obs: Trigger<OnAdd, B>, mut res: ResMut<R>, mut commands: Commands| {
                res.assert_order(1);
                commands.entity(obs.entity()).remove::<A>();
            },
        );
        world.observe(|_: Trigger<OnRemove, B>, mut res: ResMut<R>| {
            res.assert_order(3);
        });

        let entity = world.spawn(A).flush();
        let entity = world.get_entity(entity).unwrap();
        assert!(!entity.contains::<A>());
        assert!(!entity.contains::<B>());
        assert_eq!(4, world.resource::<R>().0);
    }

    #[test]
    fn observer_multiple_listeners() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<R>| res.0 += 1);
        world.observe(|_: Trigger<OnAdd, A>, mut res: ResMut<R>| res.0 += 1);

        world.spawn(A).flush();
        assert_eq!(2, world.resource::<R>().0);
        // Our A entity plus our two observers
        assert_eq!(world.entities().len(), 3);
    }

    #[test]
    fn observer_multiple_events() {
        let mut world = World::new();
        world.init_resource::<R>();
        let on_remove = world.init_component::<OnRemove>();
        world.spawn(
            Observer::new(|_: Trigger<OnAdd, A>, mut res: ResMut<R>| res.0 += 1)
                .with_event(on_remove),
        );

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(2, world.resource::<R>().0);
    }

    #[test]
    fn observer_multiple_components() {
        let mut world = World::new();
        world.init_resource::<R>();
        world.init_component::<A>();
        world.init_component::<B>();

        world.observe(|_: Trigger<OnAdd, (A, B)>, mut res: ResMut<R>| res.0 += 1);

        let entity = world.spawn(A).id();
        world.entity_mut(entity).insert(B);
        world.flush();
        assert_eq!(2, world.resource::<R>().0);
    }

    #[test]
    fn observer_despawn() {
        let mut world = World::new();
        world.init_resource::<R>();

        let observer = world
            .observe(|_: Trigger<OnAdd, A>| panic!("Observer triggered after being despawned."))
            .id();
        world.despawn(observer);
        world.spawn(A).flush();
    }

    #[test]
    fn observer_multiple_matches() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observe(|_: Trigger<OnAdd, (A, B)>, mut res: ResMut<R>| res.0 += 1);

        world.spawn((A, B)).flush();
        assert_eq!(1, world.resource::<R>().0);
    }

    #[test]
    fn observer_no_target() {
        let mut world = World::new();
        world.init_resource::<R>();

        world
            .spawn_empty()
            .observe(|_: Trigger<EventA>| panic!("Trigger routed to non-targeted entity."));
        world.observe(move |obs: Trigger<EventA>, mut res: ResMut<R>| {
            assert_eq!(obs.entity(), Entity::PLACEHOLDER);
            res.0 += 1;
        });

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger(EventA);
        world.flush();
        assert_eq!(1, world.resource::<R>().0);
    }

    #[test]
    fn observer_entity_routing() {
        let mut world = World::new();
        world.init_resource::<R>();

        world
            .spawn_empty()
            .observe(|_: Trigger<EventA>| panic!("Trigger routed to non-targeted entity."));
        let entity = world
            .spawn_empty()
            .observe(|_: Trigger<EventA>, mut res: ResMut<R>| res.0 += 1)
            .id();
        world.observe(move |obs: Trigger<EventA>, mut res: ResMut<R>| {
            assert_eq!(obs.entity(), entity);
            res.0 += 1;
        });

        // TODO: ideally this flush is not necessary, but right now observe() returns WorldEntityMut
        // and therefore does not automatically flush.
        world.flush();
        world.trigger_targets(EventA, entity);
        world.flush();
        assert_eq!(2, world.resource::<R>().0);
    }

    #[test]
    fn observer_dynamic_component() {
        let mut world = World::new();
        world.init_resource::<R>();

        let component_id = world.init_component::<A>();
        world.spawn(
            Observer::new(|_: Trigger<OnAdd>, mut res: ResMut<R>| res.0 += 1)
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
        assert_eq!(1, world.resource::<R>().0);
    }

    #[test]
    fn observer_dynamic_trigger() {
        let mut world = World::new();
        world.init_resource::<R>();
        let event_a = world.init_component::<EventA>();

        world.spawn(ObserverState {
            descriptor: ObserverDescriptor::default().with_triggers(vec![event_a]),
            runner: |mut world, _trigger, _ptr| {
                world.resource_mut::<R>().0 += 1;
            },
            ..Default::default()
        });

        world.commands().add(
            // SAFETY: we registered `trigger` above and it matches the type of TriggerA
            unsafe { EmitDynamicTrigger::new_with_id(event_a, EventA, ()) },
        );
        world.flush();
        assert_eq!(1, world.resource::<R>().0);
    }
}
