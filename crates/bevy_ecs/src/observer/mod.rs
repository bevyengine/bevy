//! Types for creating and storing [`Observer`]s

mod builder;
mod runner;
mod state;

pub use builder::*;
pub use runner::*;
use state::*;

use crate::{
    self as bevy_ecs,
    archetype::{ArchetypeFlags, Archetypes},
    entity::EntityLocation,
    query::{DebugCheckedUnwrap, FilteredAccess, WorldQuery, WorldQueryData},
    system::{EmitEcsEvent, Insert},
    world::*,
};

use bevy_ptr::PtrMut;
use bevy_utils::{EntityHashMap, HashMap};

use crate::{component::ComponentId, prelude::*, query::WorldQueryFilter, world::DeferredWorld};

/// Trait used to mark components used as ECS events.
pub trait EcsEvent: Component {}

impl<E: Component> EcsEvent for E {}

/// Type used in callbacks registered for observers.
/// TODO: Proper docs and examples
pub struct Observer<'w, E, Q: WorldQueryData, F: WorldQueryFilter = ()> {
    world: DeferredWorld<'w>,
    state: &'w ObserverState<Q, F>,
    data: &'w mut E,
    trigger: ObserverTrigger,
}

impl<'w, E, Q: WorldQueryData, F: WorldQueryFilter> Observer<'w, E, Q, F> {
    pub(crate) fn new(
        world: DeferredWorld<'w>,
        state: &'w mut ObserverState<Q, F>,
        data: &'w mut E,
        trigger: ObserverTrigger,
    ) -> Self {
        Self {
            world,
            state,
            data,
            trigger,
        }
    }

    /// Returns the event id for the triggering event
    pub fn event(&self) -> ComponentId {
        self.trigger.event
    }

    /// Gets read access to the data for [`Q`] from the triggering entity
    pub fn fetch(&self) -> <Q::ReadOnly as WorldQuery>::Item<'_> {
        let location = self.world.entities.get(self.trigger.source).unwrap();
        let world = self.world.as_unsafe_world_cell_readonly();
        unsafe {
            let mut fetch = Q::ReadOnly::init_fetch(
                world,
                &self.state.fetch_state,
                world.last_change_tick(),
                world.change_tick(),
            );
            let archetype = world.archetypes().get(location.archetype_id).unwrap();
            let table = world.storages().tables.get(location.table_id).unwrap();
            Q::ReadOnly::set_archetype(&mut fetch, &self.state.fetch_state, archetype, table);
            Q::ReadOnly::fetch(&mut fetch, self.trigger.source, location.table_row)
        }
    }

    /// Gets write access to the data for [`Q`] from the triggering entity
    pub fn fetch_mut(&mut self) -> Q::Item<'_> {
        let location = self.world.entities.get(self.trigger.source).unwrap();
        let world = self.world.as_unsafe_world_cell();
        unsafe {
            let mut fetch = Q::init_fetch(
                world,
                &self.state.fetch_state,
                world.last_change_tick(),
                world.change_tick(),
            );
            let archetype = world.archetypes().get(location.archetype_id).unwrap();
            let table = world.storages().tables.get(location.table_id).unwrap();
            Q::set_archetype(&mut fetch, &self.state.fetch_state, archetype, table);
            Q::fetch(&mut fetch, self.trigger.source, location.table_row)
        }
    }

    /// Returns a reference to the data associated with the event that triggered the observer.
    pub fn data(&self) -> &E {
        &self.data
    }

    /// Returns a mutable reference to the data associated with the event that triggered the observer.
    pub fn data_mut(&mut self) -> &mut E {
        &mut self.data
    }

    /// Returns the entity that triggered the observer.
    pub fn source(&self) -> Entity {
        self.trigger.source
    }

    /// Returns a reference to the underlying [`DeferredWorld`]
    pub fn world(&self) -> &DeferredWorld {
        &self.world
    }

    /// Returns a mutable reference to the underlying [`DeferredWorld`]
    pub fn world_mut(&mut self) -> &mut DeferredWorld<'w> {
        &mut self.world
    }
}

#[derive(Default, Clone, Component)]
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
    component_observers: HashMap<ComponentId, EntityHashMap<Entity, ObserverCallback>>,
    entity_observers: EntityHashMap<Entity, EntityHashMap<Entity, ObserverCallback>>,
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

    pub(crate) fn register(
        &mut self,
        archetypes: &mut Archetypes,
        entity: Entity,
        observer: &ObserverComponent,
    ) {
        for &event in &observer.descriptor.events {
            let cache = self.get_observers(event);
            for &component in &observer.descriptor.components {
                let observers = cache.component_observers.entry(component).or_default();
                observers.insert(entity, observer.callback);
                if observers.len() == 1 {
                    if let Some(flag) = Self::is_archetype_cached(event) {
                        archetypes.update_flags(component, flag, true);
                    }
                }
            }
            for &source in &observer.descriptor.sources {
                let observers = cache.entity_observers.entry(source).or_default();
                observers.insert(entity, observer.callback);
            }
        }
    }

    pub(crate) fn unregister(
        &mut self,
        archetypes: &mut Archetypes,
        entity: Entity,
        observer: &ObserverComponent,
    ) {
        for &event in &observer.descriptor.events {
            let Some(cache) = self.try_get_observers_mut(event) else {
                continue;
            };
            for component in &observer.descriptor.components {
                let Some(observers) = cache.component_observers.get_mut(component) else {
                    continue;
                };
                observers.remove(&entity);
                if observers.is_empty() {
                    cache.component_observers.remove(component);
                    if let Some(flag) = Self::is_archetype_cached(event) {
                        archetypes.update_flags(*component, flag, false);
                    }
                }
            }
            for source in &observer.descriptor.sources {
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

    pub(crate) fn invoke<E>(
        &self,
        event: ComponentId,
        source: Entity,
        location: EntityLocation,
        components: impl Iterator<Item = ComponentId>,
        mut world: DeferredWorld,
        data: &mut E,
    ) {
        let Some(observers) = self.try_get_observers(event) else {
            return;
        };
        // Run entity observers for source
        if let Some(observers) = observers.entity_observers.get(&source) {
            observers.iter().for_each(|(&observer, runner)| {
                (runner.run)(
                    world.reborrow(),
                    ObserverTrigger {
                        observer,
                        event,
                        location,
                        source,
                    },
                    data.into(),
                    runner.callback,
                );
            });
        }
        // Run component observers for ANY
        if let Some(observers) = observers.component_observers.get(&ANY) {
            observers.iter().for_each(|(&observer, runner)| {
                (runner.run)(
                    world.reborrow(),
                    ObserverTrigger {
                        observer,
                        event,
                        location,
                        source,
                    },
                    data.into(),
                    runner.callback,
                )
            })
        }
        // Run component observers for each component
        for component in components {
            if let Some(observers) = observers.component_observers.get(&component) {
                observers.iter().for_each(|(&observer, runner)| {
                    (runner.run)(
                        world.reborrow(),
                        ObserverTrigger {
                            observer,
                            event,
                            location,
                            source,
                        },
                        data.into(),
                        runner.callback,
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

/// EcsEvent to signify an entity observer being attached to an entity
/// Can be modelled by parent-child relationship if/when that is enforced
#[derive(Component)]
pub(crate) struct AttachObserver(pub(crate) Entity);

/// Tracks a list of entity observers for the attached entity
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
pub(crate) struct ObservedBy(Vec<Entity>);

/// Type used to construct and emit a [`EcsEvent`]
pub struct EventBuilder<'w, E> {
    world: DeferredWorld<'w>,
    targets: Vec<Entity>,
    components: Vec<ComponentId>,
    data: Option<E>,
}

impl<'w, E: EcsEvent> EventBuilder<'w, E> {
    /// Constructs a new builder that will write it's event to `world`'s command queue
    pub fn new(data: E, world: DeferredWorld<'w>) -> Self {
        Self {
            world,
            targets: Vec::new(),
            components: Vec::new(),
            data: Some(data),
        }
    }

    /// Adds `target` to the list of entities targeted by `self`
    pub fn entity(&mut self, target: Entity) -> &mut Self {
        self.targets.push(target);
        self
    }

    /// Add the [`ComponentId`] of `T` to the list of components targeted by `self`
    pub fn component<T: Component>(&mut self) -> &mut Self {
        let component_id = self.world.components().component_id::<T>().expect(
            "Cannot emit event for component that does not exist, initialize components before emitting events targeting them."
        );
        self.components.push(component_id);
        self
    }

    /// Adds `component_id` to the list of components targeted by `self`
    pub fn component_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.components.push(component_id);
        self
    }

    /// Add the event to the command queue of world
    pub fn emit(&mut self) {
        self.world.commands().add(EmitEcsEvent::<E> {
            data: std::mem::take(&mut self.data).unwrap(),
            entities: std::mem::take(&mut self.targets),
            components: std::mem::take(&mut self.components),
        });
    }
}

impl World {
    /// Initialize components and register hooks for types used by [`Observer`].
    pub(crate) fn bootstrap_observers(&mut self) {
        // Update event cache when observers are spawned and despawned
        self.register_component::<ObserverComponent>()
            .on_add(|mut world, entity, _| {
                let (world, archetypes, observers) = unsafe {
                    let world = world.as_unsafe_world_cell();
                    (
                        world.into_deferred(),
                        world.archetypes_mut(),
                        world.observers_mut(),
                    )
                };

                let observer = world.get::<ObserverComponent>(entity).unwrap();
                observers.register(archetypes, entity, observer);
            })
            .on_remove(|mut world, entity, _| {
                let (world, archetypes, observers) = unsafe {
                    let world = world.as_unsafe_world_cell();
                    (
                        world.into_deferred(),
                        world.archetypes_mut(),
                        world.observers_mut(),
                    )
                };

                let observer = world.get::<ObserverComponent>(entity).unwrap();
                observers.unregister(archetypes, entity, observer);
            });

        // When any entity is targeted for an `AttachObserver` event add it to `ObservedBy`
        // or insert `ObservedBy` if it doesn't exist
        // Can also use a hooks here instead
        self.observer_builder().components::<Any>().run(
            |mut observer: Observer<AttachObserver, Option<&mut ObservedBy>>| {
                let attached_observer = observer.data().0;
                if let Some(mut observed_by) = observer.fetch_mut() {
                    observed_by.0.push(attached_observer);
                } else {
                    let source = observer.source();
                    observer
                        .world_mut()
                        .commands()
                        .entity(source)
                        .insert(ObservedBy(vec![attached_observer]));
                }
            },
        );

        // When an entity is despawned while being observed by entity observers despawn them
        self.register_component::<ObservedBy>()
            .on_remove(|mut world, entity, _| {
                let observed_by =
                    std::mem::take(world.get_mut::<ObservedBy>(entity).unwrap().as_mut());
                observed_by.0.iter().for_each(|&e| {
                    world.commands().entity(e).despawn();
                });
            });
    }

    /// Construct an [`ObserverBuilder`]
    pub fn observer_builder<E: EcsEvent>(&mut self) -> ObserverBuilder<E> {
        ObserverBuilder::new(self)
    }

    /// Create an [`Observer`] for the components accessed in `Q`.
    /// For more control over targetting components see [`Self::observer_builder`].
    /// For observing events targetting a specific entity see [`EntityWorldMut::observe`].
    pub fn observer<E: EcsEvent, Q: WorldQueryData + 'static, F: WorldQueryFilter + 'static>(
        &mut self,
        callback: fn(Observer<E, Q, F>),
    ) -> Entity {
        ObserverBuilder::new(self).run(callback)
    }

    /// Constructs an [`EventBuilder`] for an [`EcsEvent`].
    pub fn ecs_event<E: EcsEvent>(&mut self, event: E) -> EventBuilder<E> {
        self.init_component::<E>();
        // TODO: Safe into deferred for world
        EventBuilder::new(event, unsafe {
            self.as_unsafe_world_cell().into_deferred()
        })
    }

    pub(crate) fn spawn_observer<
        E: EcsEvent,
        Q: WorldQueryData + 'static,
        F: WorldQueryFilter + 'static,
    >(
        &mut self,
        mut observer: ObserverComponent,
    ) -> Entity {
        let iterator_state = ObserverState::<Q, F>::new(self);
        let components = &mut observer.descriptor.components;
        let sources = &observer.descriptor.sources;
        // If the observer has no explicit targets use the accesses of the query
        if components.is_empty() && sources.is_empty() {
            components.extend(iterator_state.component_access.access().reads_and_writes());
            // If there are still no targets add the ANY target
            if components.is_empty() {
                components.push(ANY);
            }
        }
        let entity = self.entities.reserve_entity();
        self.command_queue.push(Insert {
            entity,
            bundle: (iterator_state, observer),
        });

        entity
    }
}
