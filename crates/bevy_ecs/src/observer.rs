//! Types for creating and storing [`Observer`]s

use std::{any::TypeId, marker::PhantomData};

use crate::{
    self as bevy_ecs,
    archetype::{ArchetypeFlags, Archetypes},
    query::{DebugCheckedUnwrap, FilteredAccess},
    system::Insert,
    world::*,
};

use bevy_ptr::PtrMut;
use bevy_utils::{EntityHashMap, HashMap};

use crate::{
    component::ComponentId,
    prelude::*,
    query::{ReadOnlyWorldQuery, WorldQuery},
    world::DeferredWorld,
};

pub struct Observer<'w, E, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    world: DeferredWorld<'w>,
    state: &'w mut ObserverState<Q, F>,
    data: &'w mut E,
    trigger: ObserverTrigger,
}

impl<'w, E, Q: WorldQuery, F: ReadOnlyWorldQuery> Observer<'w, E, Q, F> {
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

    pub fn event(&self) -> ComponentId {
        self.trigger.event
    }

    pub fn fetch(&mut self) -> Q::Item<'_> {
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

    pub fn data(&self) -> &E {
        &self.data
    }

    pub fn source(&self) -> Entity {
        self.trigger.source
    }

    pub fn world(&self) -> &DeferredWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut DeferredWorld<'w> {
        &mut self.world
    }
}

#[derive(Component)]
struct ObserverState<Q: WorldQuery, F: ReadOnlyWorldQuery> {
    fetch_state: Q::State,
    filter_state: F::State,
    component_access: FilteredAccess<ComponentId>,
    last_event_id: u32,
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> ObserverState<Q, F> {
    pub fn new(world: &mut World) -> Self {
        let fetch_state = Q::init_state(world);
        let filter_state = F::init_state(world);

        let mut component_access = FilteredAccess::default();
        Q::update_component_access(&fetch_state, &mut component_access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_component_access = FilteredAccess::default();
        F::update_component_access(&filter_state, &mut filter_component_access);

        // Merge the temporary filter access with the main access. This ensures that filter access is
        // properly considered in a global "cross-query" context (both within systems and across systems).
        component_access.extend(&filter_component_access);

        Self {
            fetch_state,
            filter_state,
            component_access,
            last_event_id: 0,
        }
    }
}

pub trait EcsEvent: Component {}

impl<C: Component> EcsEvent for C {}

#[derive(Default, Clone, Component)]
pub(crate) struct ObserverDescriptor {
    events: Vec<ComponentId>,
    components: Vec<ComponentId>,
    sources: Vec<Entity>,
}

pub struct ObserverBuilder<'w, E: EcsEvent = NoEvent> {
    world: &'w mut World,
    descriptor: ObserverDescriptor,
    _marker: PhantomData<E>,
}

impl<'w, E: EcsEvent> ObserverBuilder<'w, E> {
    pub fn new(world: &'w mut World) -> Self {
        let mut descriptor = ObserverDescriptor::default();
        let event = world.init_component::<E>();
        if event != NO_EVENT {
            descriptor.events.push(event);
        }
        Self {
            world,
            descriptor,
            _marker: PhantomData::default(),
        }
    }

    // Allows listening for multiple types of events but without passing typed data
    pub fn on_event<NewE: EcsEvent>(&mut self) -> &mut ObserverBuilder<'w, NoEvent> {
        let type_id = TypeId::of::<NewE>();
        let event = self.world.init_component::<NewE>();
        self.descriptor.events.push(event);
        // SAFETY: () type will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    pub fn on_event_ids(
        &mut self,
        events: impl IntoIterator<Item = ComponentId>,
    ) -> &mut ObserverBuilder<'w, NoEvent> {
        self.descriptor.events.extend(events);
        // SAFETY: () type will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    pub fn components<T: Bundle>(&mut self) -> &mut Self {
        T::component_ids(
            &mut self.world.components,
            &mut self.world.storages,
            &mut |id| self.descriptor.components.push(id),
        );
        self
    }

    pub fn component_ids<T: Bundle>(
        &mut self,
        ids: impl IntoIterator<Item = ComponentId>,
    ) -> &mut Self {
        self.descriptor.components.extend(ids);
        self
    }

    pub fn source(&mut self, source: Entity) -> &mut Self {
        self.descriptor.sources.push(source);
        self
    }

    pub fn run<Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
        callback: fn(Observer<E, Q, F>),
    ) -> Entity {
        let entity = self.world.spawn_observer(&self.descriptor, callback);
        self.world.flush_commands();
        entity
    }

    pub fn enqueue<Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
        callback: fn(Observer<E, Q, F>),
    ) -> Entity {
        self.world.spawn_observer(&self.descriptor, callback)
    }
}

pub struct ObserverTrigger {
    observer: Entity,
    event: ComponentId,
    source: Entity,
}

#[derive(Copy, Clone, Debug)]
struct ObserverCallback {
    run: fn(DeferredWorld, ObserverTrigger, PtrMut, Option<fn(Observer<(), ()>)>),
    callback: Option<fn(Observer<(), ()>)>,
}

#[derive(Component)]
pub(crate) struct ObserverComponent {
    descriptor: ObserverDescriptor,
    runner: ObserverCallback,
}

impl ObserverComponent {
    fn from<E: 'static, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static>(
        descriptor: ObserverDescriptor,
        value: fn(Observer<E, Q, F>),
    ) -> Self {
        Self {
            descriptor,
            runner: ObserverCallback {
                run: |mut world, trigger, ptr, callback| {
                    let callback: fn(Observer<E, Q, F>) =
                        unsafe { std::mem::transmute(callback.debug_checked_unwrap()) };
                    // let last_event = world.last_event_id;
                    let mut state = unsafe {
                        world
                            .get_mut::<ObserverState<Q, F>>(trigger.observer)
                            .debug_checked_unwrap()
                    };
                    // if state.last_event_id == last_event {
                    //     return;
                    // }

                    let state: *mut ObserverState<Q, F> = state.as_mut();
                    // SAFETY: Pointer is valid as we just created it, ObserverState is a private type and so will not be aliased
                    let observer = Observer::new(
                        world,
                        unsafe { &mut *state },
                        unsafe { ptr.deref_mut() },
                        trigger,
                    );
                    callback(observer);
                },
                callback: Some(unsafe { std::mem::transmute(value) }),
            },
        }
    }
}

#[derive(Default, Debug)]
struct CachedObservers {
    component_observers: HashMap<ComponentId, EntityHashMap<Entity, ObserverCallback>>,
    entity_observers: EntityHashMap<Entity, EntityHashMap<Entity, ObserverCallback>>,
}

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
                observers.insert(entity, observer.runner);
                if observers.len() == 1 {
                    if let Some(flag) = Self::is_archetype_cached(event) {
                        archetypes.update_flags(component, flag, true);
                    }
                }
            }
            for &source in &observer.descriptor.sources {
                let observers = cache.entity_observers.entry(source).or_default();
                observers.insert(entity, observer.runner);
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
        components: impl Iterator<Item = ComponentId>,
        mut world: DeferredWorld,
        data: &mut E,
    ) {
        let Some(observers) = self.try_get_observers(event) else {
            return;
        };
        if let Some(observers) = observers.entity_observers.get(&source) {
            observers.iter().for_each(|(&observer, runner)| {
                (runner.run)(
                    world.clone(),
                    ObserverTrigger {
                        observer,
                        event,
                        source,
                    },
                    data.into(),
                    runner.callback,
                );
            });
        }
        for component in components {
            if let Some(observers) = observers.component_observers.get(&component) {
                observers.iter().for_each(|(&observer, runner)| {
                    (runner.run)(
                        world.clone(),
                        ObserverTrigger {
                            observer,
                            event,
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

#[derive(Component)]
pub struct OnAdd;

#[derive(Component)]
pub struct OnInsert;

#[derive(Component)]
pub struct OnRemove;

#[derive(Component)]
pub struct NoEvent;

#[derive(Component)]
pub(crate) struct AttachObserver(pub(crate) Entity);

#[derive(Component, Default)]
pub(crate) struct ObservedBy(Vec<Entity>);

pub struct EventBuilder<'w, E> {
    event: ComponentId,
    world: &'w mut World,
    data: E,
    targets: Vec<Entity>,
    components: Vec<ComponentId>,
}

impl<'w, E: EcsEvent> EventBuilder<'w, E> {
    pub fn new(world: &'w mut World, data: E) -> Self {
        let event = world.init_component::<E>();
        Self {
            event,
            world,
            data,
            targets: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn target(&mut self, target: Entity) -> &mut Self {
        self.targets.push(target);
        self
    }

    pub fn emit(&mut self) {
        let mut world = unsafe { self.world.as_unsafe_world_cell().into_deferred() };
        for &target in &self.targets {
            unsafe {
                world.trigger_observers_with_data(
                    self.event,
                    target,
                    self.components.iter().cloned(),
                    &mut self.data,
                )
            }
        }
    }
}

impl World {
    pub(crate) fn bootstrap_observers(&mut self) {
        assert_eq!(NO_EVENT, self.init_component::<NoEvent>());
        assert_eq!(ON_ADD, self.init_component::<OnAdd>());
        assert_eq!(ON_INSERT, self.init_component::<OnInsert>());
        assert_eq!(ON_REMOVE, self.init_component::<OnRemove>());
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

        self.register_component::<AttachObserver>()
            .on_add(|mut world, entity, _| {
                let observer = world.get::<AttachObserver>(entity).unwrap().0;
                world.with_commands(|mut commands| {
                    commands.entity(entity).remove::<AttachObserver>();
                });
                match world.get_mut::<ObservedBy>(entity) {
                    Some(mut o) => o.0.push(observer),
                    None => world.with_commands(|mut commands| {
                        commands.entity(entity).insert(ObservedBy(vec![observer]));
                    }),
                }
            });

        self.register_component::<ObservedBy>()
            .on_remove(|mut world, entity, _| {
                let observed_by =
                    std::mem::take(world.get_mut::<ObservedBy>(entity).unwrap().as_mut());
                world.with_commands(|mut commands| {
                    observed_by.0.iter().for_each(|&e| {
                        commands.entity(e).despawn();
                    })
                })
            });
    }

    pub fn observer_builder<E: EcsEvent>(&mut self) -> ObserverBuilder<E> {
        ObserverBuilder::new(self)
    }

    pub fn observer<E: EcsEvent, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static>(
        &mut self,
        callback: fn(Observer<E, Q, F>),
    ) -> Entity {
        ObserverBuilder::new(self).run(callback)
    }

    pub fn ecs_event<E: EcsEvent>(&mut self, event: E) -> EventBuilder<E> {
        EventBuilder::new(self, event)
    }

    pub(crate) fn spawn_observer<
        E: EcsEvent,
        Q: WorldQuery + 'static,
        F: ReadOnlyWorldQuery + 'static,
    >(
        &mut self,
        descriptor: &ObserverDescriptor,
        callback: fn(Observer<E, Q, F>),
    ) -> Entity {
        let mut descriptor = descriptor.clone();
        let iterator_state = ObserverState::<Q, F>::new(self);
        if descriptor.components.is_empty() && descriptor.sources.is_empty() {
            descriptor
                .components
                .extend(iterator_state.component_access.access().reads_and_writes());
        }
        let entity = self.entities.reserve_entity();
        self.command_queue.push(Insert {
            entity,
            bundle: (
                iterator_state,
                ObserverComponent::from(descriptor, callback),
            ),
        });

        entity
    }
}
