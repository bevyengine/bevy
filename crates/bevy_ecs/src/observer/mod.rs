//! Types for creating and storing [`Observer`]s

mod builder;
mod runner;

pub use builder::*;
pub use runner::*;

use crate::{
    archetype::ArchetypeFlags,
    component::{ComponentInfo, SparseStorage},
    entity::EntityLocation,
    query::DebugCheckedUnwrap,
    system::{EmitEcsEvent, IntoObserverSystem},
    world::*,
};

use bevy_ptr::{Ptr, PtrMut};
use bevy_utils::{EntityHashMap, HashMap};

use crate::{component::ComponentId, prelude::*, world::DeferredWorld};

/// Trait used to mark components used as ECS events.
pub trait EcsEvent: Component {}

impl<E: Component> EcsEvent for E {}

/// Type used in callbacks registered for observers.
/// TODO: Proper docs and examples
pub struct Observer<'w, E> {
    data: &'w mut E,
    trigger: ObserverTrigger,
}

impl<'w, E> Observer<'w, E> {
    pub(crate) fn new(data: &'w mut E, trigger: ObserverTrigger) -> Self {
        Self { data, trigger }
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

#[derive(Default, Debug)]
pub(crate) struct CachedObservers {
    component_observers: HashMap<ComponentId, EntityHashMap<Entity, ObserverRunner>>,
    entity_observers: EntityHashMap<Entity, EntityHashMap<Entity, ObserverRunner>>,
}

/// Metadata for observers. Stores a cache mapping event ids to the registered observers.
#[derive(Default, Debug)]
pub struct Observers {
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

    pub(crate) fn try_get_observers_mut(
        &mut self,
        event: ComponentId,
    ) -> Option<&mut CachedObservers> {
        match event {
            ON_ADD => Some(&mut self.on_add),
            ON_INSERT => Some(&mut self.on_insert),
            ON_REMOVE => Some(&mut self.on_remove),
            _ => self.cache.get_mut(&event),
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
        let (mut world, observers) = unsafe {
            let world = world.as_unsafe_world_cell();
            world.increment_event_id();
            let observers = world.observers();
            let Some(observers) = observers.try_get_observers(event) else {
                return;
            };
            (world.into_deferred(), observers)
        };

        // Run entity observers for source
        if let Some(observers) = observers.entity_observers.get(&source) {
            observers.iter().for_each(|(&observer, runner)| {
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
            });
        }
        // Run component observers for ANY
        if let Some(observers) = observers.component_observers.get(&ANY) {
            observers.iter().for_each(|(&observer, runner)| {
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
            });
        }
        // Run component observers for each component
        for component in components {
            if let Some(observers) = observers.component_observers.get(&component) {
                observers.iter().for_each(|(&observer, runner)| {
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
                });
            }
        }
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

/// Component to signify an entity observer being attached to an entity
/// Can be modelled by parent-child relationship if/when that is enforced
pub(crate) struct AttachObserver(pub(crate) Entity);

impl Component for AttachObserver {
    type Storage = SparseStorage;

    // When `AttachObserver` is inserted onto an event add it to `ObservedBy`
    // or insert `ObservedBy` if it doesn't exist
    fn init_component_info(info: &mut ComponentInfo) {
        info.on_insert(|mut world, entity, _| {
            let attached_observer = world.get::<AttachObserver>(entity).unwrap().0;
            if let Some(mut observed_by) = world.get_mut::<ObservedBy>(entity) {
                observed_by.0.push(attached_observer);
            } else {
                world
                    .commands()
                    .entity(entity)
                    .insert(ObservedBy(vec![attached_observer]));
            }
        });
    }
}

/// Tracks a list of entity observers for the attached entity
pub(crate) struct ObservedBy(Vec<Entity>);

impl Component for ObservedBy {
    type Storage = SparseStorage;

    fn init_component_info(info: &mut ComponentInfo) {
        info.on_remove(|mut world, entity, _| {
            let mut component = world.get_mut::<ObservedBy>(entity).unwrap();
            let observed_by = std::mem::take(&mut component.0);
            observed_by.iter().for_each(|&e| {
                world.commands().entity(e).despawn();
            });
        });
    }
}

/// Type used to construct and emit a [`EcsEvent`]
pub struct EventBuilder<'w, E> {
    event: Option<ComponentId>,
    commands: Commands<'w, 'w>,
    targets: Vec<Entity>,
    components: Vec<ComponentId>,
    data: Option<E>,
}

impl<'w, E: EcsEvent> EventBuilder<'w, E> {
    /// Constructs a new builder that will write it's event to `world`'s command queue
    #[must_use]
    pub fn new(data: E, commands: Commands<'w, 'w>) -> Self {
        Self {
            event: None,
            commands,
            targets: Vec::new(),
            components: Vec::new(),
            data: Some(data),
        }
    }

    #[must_use]
    pub fn event_id(&mut self, id: ComponentId) -> &mut Self {
        self.event = Some(id);
        self
    }

    /// Adds `target` to the list of entities targeted by `self`
    #[must_use]
    pub fn entity(&mut self, target: Entity) -> &mut Self {
        self.targets.push(target);
        self
    }

    /// Adds `component_id` to the list of components targeted by `self`
    #[must_use]
    pub fn component(&mut self, component_id: ComponentId) -> &mut Self {
        self.components.push(component_id);
        self
    }

    /// Add the event to the command queue of world
    pub fn emit(&mut self) {
        self.commands.add(EmitEcsEvent::<E> {
            event: self.event,
            data: std::mem::take(&mut self.data).unwrap(),
            entities: std::mem::take(&mut self.targets),
            components: std::mem::take(&mut self.components),
        });
    }
}

impl<'w, 's> Commands<'w, 's> {
    /// Constructs an [`EventBuilder`] for an [`EcsEvent`].
    pub fn event<E: EcsEvent>(&mut self, event: E) -> EventBuilder<E> {
        EventBuilder::new(event, self.reborrow())
    }
}

impl World {
    /// Construct an [`ObserverBuilder`]
    pub fn observer_builder<E: EcsEvent>(&mut self) -> ObserverBuilder<E> {
        ObserverBuilder::new(self)
    }

    /// Spawn an [`Observer`] and returns it's [`Entity`]
    pub fn observer<E: EcsEvent, M>(&mut self, callback: impl IntoObserverSystem<E, M>) -> Entity {
        ObserverBuilder::new(self).run(callback)
    }

    /// Constructs an [`EventBuilder`] for an [`EcsEvent`].
    pub fn ecs_event<E: EcsEvent>(&mut self, event: E) -> EventBuilder<E> {
        self.init_component::<E>();
        // TODO: Safe into deferred for world
        EventBuilder::new(event, self.commands())
    }

    pub(crate) fn spawn_observer(&mut self, mut observer: ObserverComponent) -> Entity {
        let components = &mut observer.descriptor.components;
        let sources = &observer.descriptor.sources;
        // If the observer has no explicit targets use the accesses of the query
        if components.is_empty() && sources.is_empty() {
            components.push(ANY);
        }
        let entity = self.entities.reserve_entity();

        self.command_queue.push(move |world: &mut World| {
            if let Some(mut entity) = world.get_entity_mut(entity) {
                entity.insert(observer);
            }
        });

        entity
    }

    pub(crate) fn register_observer(&mut self, entity: Entity) {
        let observer_component: *const ObserverComponent =
            self.get::<ObserverComponent>(entity).unwrap();
        // TODO: Make less nasty
        let observer_component = unsafe { &*observer_component };

        for &event in &observer_component.descriptor.events {
            let cache = self.observers.get_observers(event);
            for &component in &observer_component.descriptor.components {
                let observers = cache.component_observers.entry(component).or_default();
                observers.insert(entity, observer_component.runner);
                if observers.len() == 1 {
                    if let Some(flag) = Observers::is_archetype_cached(event) {
                        self.archetypes.update_flags(component, flag, true);
                    }
                }
            }
            for &source in &observer_component.descriptor.sources {
                let observers = cache.entity_observers.entry(source).or_default();
                observers.insert(entity, observer_component.runner);
            }
        }
    }

    pub(crate) fn unregister_observer(&mut self, entity: Entity) {
        let observer_component: *const ObserverComponent =
            self.get::<ObserverComponent>(entity).unwrap();

        // TODO: Make less nasty
        let observer_component = unsafe { &*observer_component };

        for &event in &observer_component.descriptor.events {
            let Some(cache) = self.observers.try_get_observers_mut(event) else {
                continue;
            };
            for component in &observer_component.descriptor.components {
                let Some(observers) = cache.component_observers.get_mut(component) else {
                    continue;
                };
                observers.remove(&entity);
                if observers.is_empty() {
                    cache.component_observers.remove(component);
                    if let Some(flag) = Observers::is_archetype_cached(event) {
                        self.archetypes.update_flags(*component, flag, false);
                    }
                }
            }
            for source in &observer_component.descriptor.sources {
                let Some(observers) = cache.entity_observers.get_mut(source) else {
                    continue;
                };
                observers.remove(&entity);
                if observers.is_empty() {
                    cache.entity_observers.remove(source);
                }
            }
        }
    }
}
