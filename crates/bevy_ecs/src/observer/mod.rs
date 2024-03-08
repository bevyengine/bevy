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
    entity::EntityLocation,
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

    /// Returns the entity that triggered the observer.
    pub fn source(&self) -> Entity {
        self.trigger.source
    }

    /// Returns the location of the entity that triggered the observer.
    pub fn location(&self) -> EntityLocation {
        self.trigger.location
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
    location: EntityLocation,
    event: ComponentId,
    source: Entity,
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
        source: Entity,
        location: EntityLocation,
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
                    location,
                    source,
                },
                data.into(),
            );
        };

        observers.map.iter().for_each(&mut trigger_observer);

        if let Some(map) = observers.entity_observers.get(&source) {
            map.iter().for_each(&mut trigger_observer);
        }

        components.for_each(|id| {
            if let Some(component_observers) = observers.component_observers.get(&id) {
                component_observers
                    .map
                    .iter()
                    .for_each(&mut trigger_observer);
                if let Some(entity_observers) = component_observers.entity_map.get(&source) {
                    entity_observers.iter().for_each(&mut trigger_observer);
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
        callback: impl IntoObserverSystem<E, B, M>,
    ) -> Entity {
        B::component_ids(&mut self.components, &mut self.storages, &mut |_| {});
        ObserverBuilder::new(self.commands()).run(callback)
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
            // Observer is not targetting any components so register it as an entity observer
            if descriptor.components.is_empty() {
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
                        // Register for all events targetting the component
                        observers.map.insert(entity, observer_component.runner);
                    } else {
                        // Register for each targetted entity
                        for &source in &descriptor.sources {
                            let map = observers.entity_map.entry(source).or_default();
                            map.insert(entity, observer_component.runner);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn unregister_observer(&mut self, entity: Entity) {
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

        for &event in &observer_component.descriptor.events {
            let cache = observers.get_observers(event);
            if descriptor.components.is_empty() {
                for source in &observer_component.descriptor.sources {
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
