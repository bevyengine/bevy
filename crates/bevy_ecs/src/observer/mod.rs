//! Types for creating and storing [`Observer`]s

mod builder;
mod entity_observer;
mod runner;

use std::marker::PhantomData;

pub use builder::*;
pub(crate) use entity_observer::*;
pub use runner::*;

use crate::{
    archetype::ArchetypeFlags,
    query::DebugCheckedUnwrap,
    system::{EmitEcsEvent, IntoObserverSystem},
    world::*,
};

use bevy_ptr::{Ptr, PtrMut};
use bevy_utils::{EntityHashMap, HashMap};

use crate::{component::ComponentId, prelude::*, world::DeferredWorld};

/// Type used in callbacks registered for observers.
pub struct Observer<'w, E, B: Bundle = ()> {
    data: &'w mut E,
    trigger: ObserverTrigger,
    _marker: PhantomData<B>,
}

impl<'w, E, B: Bundle> Observer<'w, E, B> {
    pub(crate) fn new(data: &'w mut E, trigger: ObserverTrigger) -> Self {
        Self {
            data,
            trigger,
            _marker: PhantomData,
        }
    }

    /// Returns the event id for the triggering event
    pub fn event(&self) -> ComponentId {
        self.trigger.event
    }

    /// Returns a reference to the data associated with the event that triggered the observer.
    pub fn data(&self) -> &E {
        self.data
    }

    /// Returns a mutable reference to the data associated with the event that triggered the observer.
    pub fn data_mut(&mut self) -> &mut E {
        self.data
    }

    /// Returns a pointer to the data associated with the event that triggered the observer.
    pub fn data_ptr(&self) -> Ptr {
        Ptr::from(&self.data)
    }

    /// Returns the entity that triggered the observer, panics if the event was send without a source.
    pub fn source(&self) -> Entity {
        self.trigger.source.expect("No source set for this event")
    }

    /// Returns the entity that triggered the observer if it was set.
    pub fn get_source(&self) -> Option<Entity> {
        self.trigger.source
    }
}

#[derive(Default, Clone)]
pub(crate) struct ObserverDescriptor {
    events: Vec<ComponentId>,
    components: Vec<ComponentId>,
    sources: Vec<Entity>,
}

/// Metadata for the source triggering an [`Observer`],
pub struct ObserverTrigger {
    observer: Entity,
    event: ComponentId,
    source: Option<Entity>,
}

// Map between an observer entity and it's runner
type ObserverMap = EntityHashMap<Entity, ObserverRunner>;

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular event targeted at a specific component.
#[derive(Default, Debug)]
pub struct CachedComponentObservers {
    // Observers listening to events targeting this component
    map: ObserverMap,
    // Observers listening to events targeting this component on a specific entity
    entity_map: EntityHashMap<Entity, ObserverMap>,
}

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular event.
#[derive(Default, Debug)]
pub struct CachedObservers {
    // Observers listening for any time this event is fired
    map: ObserverMap,
    // Observers listening for this event fired at a specific component
    component_observers: HashMap<ComponentId, CachedComponentObservers>,
    // Observers listening for this event fired at a specific entity
    entity_observers: EntityHashMap<Entity, ObserverMap>,
}

/// Metadata for observers. Stores a cache mapping event ids to the registered observers.
#[derive(Default, Debug)]
pub struct Observers {
    // Cached ECS observers to save a lookup most common events.
    on_add: CachedObservers,
    on_insert: CachedObservers,
    on_remove: CachedObservers,
    // Map from event type to set of observers
    cache: HashMap<ComponentId, CachedObservers>,
}

impl Observers {
    pub(crate) fn get_observers(&mut self, event: ComponentId) -> &mut CachedObservers {
        match event {
            ON_ADD => &mut self.on_add,
            ON_INSERT => &mut self.on_insert,
            ON_REMOVE => &mut self.on_remove,
            _ => self.cache.entry(event).or_default(),
        }
    }

    pub(crate) fn try_get_observers(&self, event: ComponentId) -> Option<&CachedObservers> {
        match event {
            ON_ADD => Some(&self.on_add),
            ON_INSERT => Some(&self.on_insert),
            ON_REMOVE => Some(&self.on_remove),
            _ => self.cache.get(&event),
        }
    }

    pub(crate) fn invoke<E>(
        mut world: DeferredWorld,
        event: ComponentId,
        source: Option<Entity>,
        components: impl Iterator<Item = ComponentId>,
        data: &mut E,
    ) {
        // SAFETY: You cannot get a mutable reference to `observers` from `DeferredWorld`
        let (mut world, observers) = unsafe {
            let world = world.as_unsafe_world_cell();
            // SAFETY: There are no outsanding world references
            world.increment_event_id();
            let observers = world.observers();
            let Some(observers) = observers.try_get_observers(event) else {
                return;
            };
            // SAFETY: The only outsanding reference to world is `observers`
            (world.into_deferred(), observers)
        };

        let mut trigger_observer = |(&observer, runner): (&Entity, &ObserverRunner)| {
            (runner)(
                world.reborrow(),
                ObserverTrigger {
                    observer,
                    event,
                    source,
                },
                data.into(),
            );
        };

        // Trigger observers listening for any kind of this event
        observers.map.iter().for_each(&mut trigger_observer);

        // Trigger entity observers listening for this kind of event
        if let Some(source) = source {
            if let Some(map) = observers.entity_observers.get(&source) {
                map.iter().for_each(&mut trigger_observer);
            }
        }

        // Trigger observers listening to this event targeting a specific component
        components.for_each(|id| {
            if let Some(component_observers) = observers.component_observers.get(&id) {
                component_observers
                    .map
                    .iter()
                    .for_each(&mut trigger_observer);

                if let Some(source) = source {
                    if let Some(map) = component_observers.entity_map.get(&source) {
                        map.iter().for_each(&mut trigger_observer);
                    }
                }
            }
        });
    }

    pub(crate) fn is_archetype_cached(event: ComponentId) -> Option<ArchetypeFlags> {
        match event {
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
    /// Construct an [`ObserverBuilder`]
    pub fn observer_builder<E: Component>(&mut self) -> ObserverBuilder<E> {
        self.init_component::<E>();
        ObserverBuilder::new(self.commands())
    }

    /// Spawn an [`Observer`] and returns it's [`Entity`].
    pub fn observer<E: Component, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> Entity {
        B::component_ids(&mut self.components, &mut self.storages, &mut |_| {});
        ObserverBuilder::new(self.commands()).run(system)
    }

    /// Constructs an [`EventBuilder`].
    pub fn ecs_event<E: Component>(&mut self, event: E) -> EventBuilder<E> {
        self.init_component::<E>();
        EventBuilder::new(event, self.commands())
    }

    pub(crate) fn register_observer(&mut self, entity: Entity) {
        // SAFETY: References do not alias.
        let (observer_component, archetypes, observers) = unsafe {
            let observer_component: *const ObserverComponent =
                self.get::<ObserverComponent>(entity).unwrap();
            (
                &*observer_component,
                &mut self.archetypes,
                &mut self.observers,
            )
        };
        let descriptor = &observer_component.descriptor;

        for &event in &descriptor.events {
            let cache = observers.get_observers(event);

            if descriptor.components.is_empty() && descriptor.sources.is_empty() {
                cache.map.insert(entity, observer_component.runner);
            } else if descriptor.components.is_empty() {
                // Observer is not targeting any components so register it as an entity observer
                for &source in &observer_component.descriptor.sources {
                    let map = cache.entity_observers.entry(source).or_default();
                    map.insert(entity, observer_component.runner);
                }
            } else {
                // Register observer for each source component
                for &component in &descriptor.components {
                    let observers =
                        cache
                            .component_observers
                            .entry(component)
                            .or_insert_with(|| {
                                if let Some(flag) = Observers::is_archetype_cached(event) {
                                    archetypes.update_flags(component, flag, true);
                                }
                                CachedComponentObservers::default()
                            });
                    if descriptor.sources.is_empty() {
                        // Register for all events targeting the component
                        observers.map.insert(entity, observer_component.runner);
                    } else {
                        // Register for each targeted entity
                        for &source in &descriptor.sources {
                            let map = observers.entity_map.entry(source).or_default();
                            map.insert(entity, observer_component.runner);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn unregister_observer(&mut self, entity: Entity, descriptor: ObserverDescriptor) {
        let archetypes = &mut self.archetypes;
        let observers = &mut self.observers;

        for &event in &descriptor.events {
            let cache = observers.get_observers(event);
            if descriptor.components.is_empty() && descriptor.sources.is_empty() {
                cache.map.remove(&entity);
            } else if descriptor.components.is_empty() {
                for source in &descriptor.sources {
                    // This check should be unnecessary since this observer hasn't been unregistered yet
                    let Some(observers) = cache.entity_observers.get_mut(source) else {
                        continue;
                    };
                    observers.remove(&entity);
                    if observers.is_empty() {
                        cache.entity_observers.remove(source);
                    }
                }
            } else {
                for component in &descriptor.components {
                    let Some(observers) = cache.component_observers.get_mut(component) else {
                        continue;
                    };
                    if descriptor.sources.is_empty() {
                        observers.map.remove(&entity);
                    } else {
                        for source in &descriptor.sources {
                            let Some(map) = observers.entity_map.get_mut(source) else {
                                continue;
                            };
                            map.remove(&entity);
                            if map.is_empty() {
                                observers.entity_map.remove(source);
                            }
                        }
                    }

                    if observers.map.is_empty() && observers.entity_map.is_empty() {
                        cache.component_observers.remove(component);
                        if let Some(flag) = Observers::is_archetype_cached(event) {
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
    use crate::component::ComponentDescriptor;
    use crate::observer::EventBuilder;
    use crate::prelude::*;

    use super::ObserverBuilder;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(Component)]
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

        world.observer(|_: Observer<OnAdd, A>, mut res: ResMut<R>| res.assert_order(0));
        world.observer(|_: Observer<OnInsert, A>, mut res: ResMut<R>| res.assert_order(1));
        world.observer(|_: Observer<OnRemove, A>, mut res: ResMut<R>| res.assert_order(2));

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(3, world.resource::<R>().0);
    }

    #[test]
    fn observer_order_insert_remove() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observer(|_: Observer<OnAdd, A>, mut res: ResMut<R>| res.assert_order(0));
        world.observer(|_: Observer<OnInsert, A>, mut res: ResMut<R>| res.assert_order(1));
        world.observer(|_: Observer<OnRemove, A>, mut res: ResMut<R>| res.assert_order(2));

        let mut entity = world.spawn_empty();
        entity.insert(A);
        entity.remove::<A>();
        entity.flush();
        assert_eq!(3, world.resource::<R>().0);
    }

    #[test]
    fn observer_order_recursive() {
        let mut world = World::new();
        world.init_resource::<R>();
        world.observer(
            |obs: Observer<OnAdd, A>, mut res: ResMut<R>, mut commands: Commands| {
                res.assert_order(0);
                commands.entity(obs.source()).insert(B);
            },
        );
        world.observer(
            |obs: Observer<OnRemove, A>, mut res: ResMut<R>, mut commands: Commands| {
                res.assert_order(2);
                commands.entity(obs.source()).remove::<B>();
            },
        );

        world.observer(
            |obs: Observer<OnAdd, B>, mut res: ResMut<R>, mut commands: Commands| {
                res.assert_order(1);
                commands.entity(obs.source()).remove::<A>();
            },
        );
        world.observer(|_: Observer<OnRemove, B>, mut res: ResMut<R>| {
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

        world.observer(|_: Observer<OnAdd, A>, mut res: ResMut<R>| res.0 += 1);
        world.observer(|_: Observer<OnAdd, A>, mut res: ResMut<R>| res.0 += 1);

        world.spawn(A).flush();
        assert_eq!(2, world.resource::<R>().0);
    }

    #[test]
    fn observer_multiple_events() {
        let mut world = World::new();
        world.init_resource::<R>();
        world.init_component::<A>();

        world
            .observer_builder::<OnAdd>()
            .on_event::<OnRemove>()
            .run(|_: Observer<_, A>, mut res: ResMut<R>| res.0 += 1);

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

        world.observer(|_: Observer<OnAdd, (A, B)>, mut res: ResMut<R>| res.0 += 1);

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
            .observer(|_: Observer<OnAdd, A>| panic!("Observer triggered after being despawned."));
        world.despawn(observer);
        world.spawn(A).flush();
    }

    #[test]
    fn observer_multiple_matches() {
        let mut world = World::new();
        world.init_resource::<R>();

        world.observer(|_: Observer<OnAdd, (A, B)>, mut res: ResMut<R>| res.0 += 1);

        world.spawn((A, B)).flush();
        assert_eq!(1, world.resource::<R>().0);
    }

    #[test]
    fn observer_no_source() {
        let mut world = World::new();
        world.init_resource::<R>();
        world.init_component::<EventA>();

        world
            .spawn_empty()
            .observe(|_: Observer<EventA>| panic!("Event routed to non-targeted entity."));
        world.observer(move |obs: Observer<EventA>, mut res: ResMut<R>| {
            assert!(obs.get_source().is_none());
            res.0 += 1;
        });

        world.ecs_event(EventA).emit();
        world.flush();
        assert_eq!(1, world.resource::<R>().0);
    }

    #[test]
    fn observer_entity_routing() {
        let mut world = World::new();
        world.init_resource::<R>();
        world.init_component::<EventA>();

        world
            .spawn_empty()
            .observe(|_: Observer<EventA>| panic!("Event routed to non-targeted entity."));
        let entity = world
            .spawn_empty()
            .observe(|_: Observer<EventA>, mut res: ResMut<R>| res.0 += 1)
            .id();
        world.observer(move |obs: Observer<EventA>, mut res: ResMut<R>| {
            assert_eq!(obs.source(), entity);
            res.0 += 1;
        });

        world.ecs_event(EventA).entity(entity).emit();
        world.flush();
        assert_eq!(2, world.resource::<R>().0);
    }

    #[test]
    fn observer_dynamic_component() {
        let mut world = World::new();
        world.init_resource::<R>();

        let component_id = world.init_component_with_descriptor(ComponentDescriptor::new::<A>());
        world
            .observer_builder()
            .component_ids(&[component_id])
            .run(|_: Observer<OnAdd>, mut res: ResMut<R>| res.0 += 1);

        let mut entity = world.spawn_empty();
        OwningPtr::make(A, |ptr| {
            // SAFETY: we registered `component_id` above.
            unsafe { entity.insert_by_id(component_id, ptr) };
        });
        let entity = entity.flush();

        world.ecs_event(EventA).entity(entity).emit();
        world.flush();
        assert_eq!(1, world.resource::<R>().0);
    }

    #[test]
    fn observer_dynamic_event() {
        let mut world = World::new();
        world.init_resource::<R>();

        let event = world.init_component_with_descriptor(ComponentDescriptor::new::<EventA>());
        // SAFETY: we registered `event` above
        unsafe { ObserverBuilder::new_with_id(event, world.commands()) }
            .run(|_: Observer<EventA>, mut res: ResMut<R>| res.0 += 1);

        // SAFETY: we registered `event` above
        unsafe { EventBuilder::new_with_id(event, EventA, world.commands()) }.emit();
        world.flush();
        assert_eq!(1, world.resource::<R>().0);
    }
}
