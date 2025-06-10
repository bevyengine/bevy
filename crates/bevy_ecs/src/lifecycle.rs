//! This module contains various tools to allow you to react to component insertion or removal,
//! as well as entity spawning and despawning.
//!
//! There are four main ways to react to these lifecycle events:
//!
//! 1. Using component hooks, which act as inherent constructors and destructors for components.
//! 2. Using [observers], which are a user-extensible way to respond to events, including component lifecycle events.
//! 3. Using the [`RemovedComponents`] system parameter, which offers an event-style interface.
//! 4. Using the [`Added`] query filter, which checks each component to see if it has been added since the last time a system ran.
//!
//! [observers]: crate::observer
//! [`Added`]: crate::query::Added
//!
//! # Types of lifecycle events
//!
//! There are five types of lifecycle events, split into two categories. First, we have lifecycle events that are triggered
//! when a component is added to an entity:
//!
//! - [`OnAdd`]: Triggered when a component is added to an entity that did not already have it.
//! - [`OnInsert`]: Triggered when a component is added to an entity, regardless of whether it already had it.
//!
//! When both events occur, [`OnAdd`] hooks are evaluated before [`OnInsert`].
//!
//! Next, we have lifecycle events that are triggered when a component is removed from an entity:
//!
//! - [`OnReplace`]: Triggered when a component is removed from an entity, regardless if it is then replaced with a new value.
//! - [`OnRemove`]: Triggered when a component is removed from an entity and not replaced, before the component is removed.
//! - [`OnDespawn`]: Triggered for each component on an entity when it is despawned.
//!
//! [`OnReplace`] hooks are evaluated before [`OnRemove`], then finally [`OnDespawn`] hooks are evaluated.
//!
//! [`OnAdd`] and [`OnRemove`] are counterparts: they are only triggered when a component is added or removed
//! from an entity in such a way as to cause a change in the component's presence on that entity.
//! Similarly, [`OnInsert`] and [`OnReplace`] are counterparts: they are triggered when a component is added or replaced
//! on an entity, regardless of whether this results in a change in the component's presence on that entity.
//!
//! To reliably synchronize data structures using with component lifecycle events,
//! you can combine [`OnInsert`] and [`OnReplace`] to fully capture any changes to the data.
//! This is particularly useful in combination with immutable components,
//! to avoid any lifecycle-bypassing mutations.
//!
//! ## Lifecycle events and component types
//!
//! Despite the absence of generics, each lifecycle event is associated with a specific component.
//! When defining a component hook for a [`Component`] type, that component is used.
//! When listening to lifecycle events for observers, the `B: Bundle` generic is used.
//!
//! Each of these lifecycle events also corresponds to a fixed [`ComponentId`],
//! which are assigned during [`World`] initialization.
//! For example, [`OnAdd`] corresponds to [`ON_ADD`].
//! This is used to skip [`TypeId`](core::any::TypeId) lookups in hot paths.
use crate::{
    change_detection::MaybeLocation,
    component::{Component, ComponentId, ComponentIdFor, Tick},
    entity::Entity,
    event::{Event, EventCursor, EventId, EventIterator, EventIteratorWithId, Events},
    relationship::RelationshipHookMode,
    storage::SparseSet,
    system::{Local, ReadOnlySystemParam, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use derive_more::derive::Into;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    fmt::Debug,
    iter,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    option,
};

/// The type used for [`Component`] lifecycle hooks such as `on_add`, `on_insert` or `on_remove`.
pub type ComponentHook = for<'w> fn(DeferredWorld<'w>, HookContext);

/// Context provided to a [`ComponentHook`].
#[derive(Clone, Copy, Debug)]
pub struct HookContext {
    /// The [`Entity`] this hook was invoked for.
    pub entity: Entity,
    /// The [`ComponentId`] this hook was invoked for.
    pub component_id: ComponentId,
    /// The caller location is `Some` if the `track_caller` feature is enabled.
    pub caller: MaybeLocation,
    /// Configures how relationship hooks will run
    pub relationship_hook_mode: RelationshipHookMode,
}

/// [`World`]-mutating functions that run as part of lifecycle events of a [`Component`].
///
/// Hooks are functions that run when a component is added, overwritten, or removed from an entity.
/// These are intended to be used for structural side effects that need to happen when a component is added or removed,
/// and are not intended for general-purpose logic.
///
/// For example, you might use a hook to update a cached index when a component is added,
/// to clean up resources when a component is removed,
/// or to keep hierarchical data structures across entities in sync.
///
/// This information is stored in the [`ComponentInfo`](crate::component::ComponentInfo) of the associated component.
///
/// There are two ways of configuring hooks for a component:
/// 1. Defining the relevant hooks on the [`Component`] implementation
/// 2. Using the [`World::register_component_hooks`] method
///
/// # Example
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_platform::collections::HashSet;
///
/// #[derive(Component)]
/// struct MyTrackedComponent;
///
/// #[derive(Resource, Default)]
/// struct TrackedEntities(HashSet<Entity>);
///
/// let mut world = World::new();
/// world.init_resource::<TrackedEntities>();
///
/// // No entities with `MyTrackedComponent` have been added yet, so we can safely add component hooks
/// let mut tracked_component_query = world.query::<&MyTrackedComponent>();
/// assert!(tracked_component_query.iter(&world).next().is_none());
///
/// world.register_component_hooks::<MyTrackedComponent>().on_add(|mut world, context| {
///    let mut tracked_entities = world.resource_mut::<TrackedEntities>();
///   tracked_entities.0.insert(context.entity);
/// });
///
/// world.register_component_hooks::<MyTrackedComponent>().on_remove(|mut world, context| {
///   let mut tracked_entities = world.resource_mut::<TrackedEntities>();
///   tracked_entities.0.remove(&context.entity);
/// });
///
/// let entity = world.spawn(MyTrackedComponent).id();
/// let tracked_entities = world.resource::<TrackedEntities>();
/// assert!(tracked_entities.0.contains(&entity));
///
/// world.despawn(entity);
/// let tracked_entities = world.resource::<TrackedEntities>();
/// assert!(!tracked_entities.0.contains(&entity));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ComponentHooks {
    pub(crate) on_add: Option<ComponentHook>,
    pub(crate) on_insert: Option<ComponentHook>,
    pub(crate) on_replace: Option<ComponentHook>,
    pub(crate) on_remove: Option<ComponentHook>,
    pub(crate) on_despawn: Option<ComponentHook>,
}

impl ComponentHooks {
    pub(crate) fn update_from_component<C: Component + ?Sized>(&mut self) -> &mut Self {
        if let Some(hook) = C::on_add() {
            self.on_add(hook);
        }
        if let Some(hook) = C::on_insert() {
            self.on_insert(hook);
        }
        if let Some(hook) = C::on_replace() {
            self.on_replace(hook);
        }
        if let Some(hook) = C::on_remove() {
            self.on_remove(hook);
        }
        if let Some(hook) = C::on_despawn() {
            self.on_despawn(hook);
        }

        self
    }

    /// Register a [`ComponentHook`] that will be run when this component is added to an entity.
    /// An `on_add` hook will always run before `on_insert` hooks. Spawning an entity counts as
    /// adding all of its components.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_add` hook
    pub fn on_add(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_add(hook)
            .expect("Component already has an on_add hook")
    }

    /// Register a [`ComponentHook`] that will be run when this component is added (with `.insert`)
    /// or replaced.
    ///
    /// An `on_insert` hook always runs after any `on_add` hooks (if the entity didn't already have the component).
    ///
    /// # Warning
    ///
    /// The hook won't run if the component is already present and is only mutated, such as in a system via a query.
    /// As a result, this needs to be combined with immutable components to serve as a mechanism for reliably updating indexes and other caches.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_insert` hook
    pub fn on_insert(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_insert(hook)
            .expect("Component already has an on_insert hook")
    }

    /// Register a [`ComponentHook`] that will be run when this component is about to be dropped,
    /// such as being replaced (with `.insert`) or removed.
    ///
    /// If this component is inserted onto an entity that already has it, this hook will run before the value is replaced,
    /// allowing access to the previous data just before it is dropped.
    /// This hook does *not* run if the entity did not already have this component.
    ///
    /// An `on_replace` hook always runs before any `on_remove` hooks (if the component is being removed from the entity).
    ///
    /// # Warning
    ///
    /// The hook won't run if the component is already present and is only mutated, such as in a system via a query.
    /// As a result, this needs to be combined with immutable components to serve as a mechanism for reliably updating indexes and other caches.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_replace` hook
    pub fn on_replace(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_replace(hook)
            .expect("Component already has an on_replace hook")
    }

    /// Register a [`ComponentHook`] that will be run when this component is removed from an entity.
    /// Despawning an entity counts as removing all of its components.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_remove` hook
    pub fn on_remove(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_remove(hook)
            .expect("Component already has an on_remove hook")
    }

    /// Register a [`ComponentHook`] that will be run for each component on an entity when it is despawned.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_despawn` hook
    pub fn on_despawn(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_despawn(hook)
            .expect("Component already has an on_despawn hook")
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is added to an entity.
    ///
    /// This is a fallible version of [`Self::on_add`].
    ///
    /// Returns `None` if the component already has an `on_add` hook.
    pub fn try_on_add(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_add.is_some() {
            return None;
        }
        self.on_add = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is added (with `.insert`)
    ///
    /// This is a fallible version of [`Self::on_insert`].
    ///
    /// Returns `None` if the component already has an `on_insert` hook.
    pub fn try_on_insert(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_insert.is_some() {
            return None;
        }
        self.on_insert = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is replaced (with `.insert`) or removed
    ///
    /// This is a fallible version of [`Self::on_replace`].
    ///
    /// Returns `None` if the component already has an `on_replace` hook.
    pub fn try_on_replace(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_replace.is_some() {
            return None;
        }
        self.on_replace = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is removed from an entity.
    ///
    /// This is a fallible version of [`Self::on_remove`].
    ///
    /// Returns `None` if the component already has an `on_remove` hook.
    pub fn try_on_remove(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_remove.is_some() {
            return None;
        }
        self.on_remove = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run for each component on an entity when it is despawned.
    ///
    /// This is a fallible version of [`Self::on_despawn`].
    ///
    /// Returns `None` if the component already has an `on_despawn` hook.
    pub fn try_on_despawn(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_despawn.is_some() {
            return None;
        }
        self.on_despawn = Some(hook);
        Some(self)
    }
}

/// [`ComponentId`] for [`OnAdd`]
pub const ON_ADD: ComponentId = ComponentId::new(0);
/// [`ComponentId`] for [`OnInsert`]
pub const ON_INSERT: ComponentId = ComponentId::new(1);
/// [`ComponentId`] for [`OnReplace`]
pub const ON_REPLACE: ComponentId = ComponentId::new(2);
/// [`ComponentId`] for [`OnRemove`]
pub const ON_REMOVE: ComponentId = ComponentId::new(3);
/// [`ComponentId`] for [`OnDespawn`]
pub const ON_DESPAWN: ComponentId = ComponentId::new(4);

/// Trigger emitted when a component is inserted onto an entity that does not already have that
/// component. Runs before `OnInsert`.
/// See [`crate::lifecycle::ComponentHooks::on_add`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnAdd;

/// Trigger emitted when a component is inserted, regardless of whether or not the entity already
/// had that component. Runs after `OnAdd`, if it ran.
/// See [`crate::lifecycle::ComponentHooks::on_insert`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnInsert;

/// Trigger emitted when a component is removed from an entity, regardless
/// of whether or not it is later replaced.
///
/// Runs before the value is replaced, so you can still access the original component data.
/// See [`crate::lifecycle::ComponentHooks::on_replace`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnReplace;

/// Trigger emitted when a component is removed from an entity, and runs before the component is
/// removed, so you can still access the component data.
/// See [`crate::lifecycle::ComponentHooks::on_remove`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnRemove;

/// Trigger emitted for each component on an entity when it is despawned.
/// See [`crate::lifecycle::ComponentHooks::on_despawn`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnDespawn;

/// Wrapper around [`Entity`] for [`RemovedComponents`].
/// Internally, `RemovedComponents` uses these as an `Events<RemovedComponentEntity>`.
#[derive(Event, Debug, Clone, Into)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug, Clone))]
pub struct RemovedComponentEntity(Entity);

/// Wrapper around a [`EventCursor<RemovedComponentEntity>`] so that we
/// can differentiate events between components.
#[derive(Debug)]
pub struct RemovedComponentReader<T>
where
    T: Component,
{
    reader: EventCursor<RemovedComponentEntity>,
    marker: PhantomData<T>,
}

impl<T: Component> Default for RemovedComponentReader<T> {
    fn default() -> Self {
        Self {
            reader: Default::default(),
            marker: PhantomData,
        }
    }
}

impl<T: Component> Deref for RemovedComponentReader<T> {
    type Target = EventCursor<RemovedComponentEntity>;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<T: Component> DerefMut for RemovedComponentReader<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

/// Stores the [`RemovedComponents`] event buffers for all types of component in a given [`World`].
#[derive(Default, Debug)]
pub struct RemovedComponentEvents {
    event_sets: SparseSet<ComponentId, Events<RemovedComponentEntity>>,
}

impl RemovedComponentEvents {
    /// Creates an empty storage buffer for component removal events.
    pub fn new() -> Self {
        Self::default()
    }

    /// For each type of component, swaps the event buffers and clears the oldest event buffer.
    /// In general, this should be called once per frame/update.
    pub fn update(&mut self) {
        for (_component_id, events) in self.event_sets.iter_mut() {
            events.update();
        }
    }

    /// Returns an iterator over components and their entity events.
    pub fn iter(&self) -> impl Iterator<Item = (&ComponentId, &Events<RemovedComponentEntity>)> {
        self.event_sets.iter()
    }

    /// Gets the event storage for a given component.
    pub fn get(
        &self,
        component_id: impl Into<ComponentId>,
    ) -> Option<&Events<RemovedComponentEntity>> {
        self.event_sets.get(component_id.into())
    }

    /// Sends a removal event for the specified component.
    pub fn send(&mut self, component_id: impl Into<ComponentId>, entity: Entity) {
        self.event_sets
            .get_or_insert_with(component_id.into(), Default::default)
            .send(RemovedComponentEntity(entity));
    }
}

/// A [`SystemParam`] that yields entities that had their `T` [`Component`]
/// removed or have been despawned with it.
///
/// This acts effectively the same as an [`EventReader`](crate::event::EventReader).
///
/// Unlike hooks or observers (see the [lifecycle](crate) module docs),
/// this does not allow you to see which data existed before removal.
///
/// If you are using `bevy_ecs` as a standalone crate,
/// note that the [`RemovedComponents`] list will not be automatically cleared for you,
/// and will need to be manually flushed using [`World::clear_trackers`](World::clear_trackers).
///
/// For users of `bevy` and `bevy_app`, [`World::clear_trackers`](World::clear_trackers) is
/// automatically called by `bevy_app::App::update` and `bevy_app::SubApp::update`.
/// For the main world, this is delayed until after all `SubApp`s have run.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::lifecycle::RemovedComponents;
/// #
/// # #[derive(Component)]
/// # struct MyComponent;
/// fn react_on_removal(mut removed: RemovedComponents<MyComponent>) {
///     removed.read().for_each(|removed_entity| println!("{}", removed_entity));
/// }
/// # bevy_ecs::system::assert_is_system(react_on_removal);
/// ```
#[derive(SystemParam)]
pub struct RemovedComponents<'w, 's, T: Component> {
    component_id: ComponentIdFor<'s, T>,
    reader: Local<'s, RemovedComponentReader<T>>,
    event_sets: &'w RemovedComponentEvents,
}

/// Iterator over entities that had a specific component removed.
///
/// See [`RemovedComponents`].
pub type RemovedIter<'a> = iter::Map<
    iter::Flatten<option::IntoIter<iter::Cloned<EventIterator<'a, RemovedComponentEntity>>>>,
    fn(RemovedComponentEntity) -> Entity,
>;

/// Iterator over entities that had a specific component removed.
///
/// See [`RemovedComponents`].
pub type RemovedIterWithId<'a> = iter::Map<
    iter::Flatten<option::IntoIter<EventIteratorWithId<'a, RemovedComponentEntity>>>,
    fn(
        (&RemovedComponentEntity, EventId<RemovedComponentEntity>),
    ) -> (Entity, EventId<RemovedComponentEntity>),
>;

fn map_id_events(
    (entity, id): (&RemovedComponentEntity, EventId<RemovedComponentEntity>),
) -> (Entity, EventId<RemovedComponentEntity>) {
    (entity.clone().into(), id)
}

// For all practical purposes, the api surface of `RemovedComponents<T>`
// should be similar to `EventReader<T>` to reduce confusion.
impl<'w, 's, T: Component> RemovedComponents<'w, 's, T> {
    /// Fetch underlying [`EventCursor`].
    pub fn reader(&self) -> &EventCursor<RemovedComponentEntity> {
        &self.reader
    }

    /// Fetch underlying [`EventCursor`] mutably.
    pub fn reader_mut(&mut self) -> &mut EventCursor<RemovedComponentEntity> {
        &mut self.reader
    }

    /// Fetch underlying [`Events`].
    pub fn events(&self) -> Option<&Events<RemovedComponentEntity>> {
        self.event_sets.get(self.component_id.get())
    }

    /// Destructures to get a mutable reference to the `EventCursor`
    /// and a reference to `Events`.
    ///
    /// This is necessary since Rust can't detect destructuring through methods and most
    /// usecases of the reader uses the `Events` as well.
    pub fn reader_mut_with_events(
        &mut self,
    ) -> Option<(
        &mut RemovedComponentReader<T>,
        &Events<RemovedComponentEntity>,
    )> {
        self.event_sets
            .get(self.component_id.get())
            .map(|events| (&mut *self.reader, events))
    }

    /// Iterates over the events this [`RemovedComponents`] has not seen yet. This updates the
    /// [`RemovedComponents`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn read(&mut self) -> RemovedIter<'_> {
        self.reader_mut_with_events()
            .map(|(reader, events)| reader.read(events).cloned())
            .into_iter()
            .flatten()
            .map(RemovedComponentEntity::into)
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`] of the events.
    pub fn read_with_id(&mut self) -> RemovedIterWithId<'_> {
        self.reader_mut_with_events()
            .map(|(reader, events)| reader.read_with_id(events))
            .into_iter()
            .flatten()
            .map(map_id_events)
    }

    /// Determines the number of removal events available to be read from this [`RemovedComponents`] without consuming any.
    pub fn len(&self) -> usize {
        self.events()
            .map(|events| self.reader.len(events))
            .unwrap_or(0)
    }

    /// Returns `true` if there are no events available to read.
    pub fn is_empty(&self) -> bool {
        self.events()
            .is_none_or(|events| self.reader.is_empty(events))
    }

    /// Consumes all available events.
    ///
    /// This means these events will not appear in calls to [`RemovedComponents::read()`] or
    /// [`RemovedComponents::read_with_id()`] and [`RemovedComponents::is_empty()`] will return `true`.
    pub fn clear(&mut self) {
        if let Some((reader, events)) = self.reader_mut_with_events() {
            reader.clear(events);
        }
    }
}

// SAFETY: Only reads World removed component events
unsafe impl<'a> ReadOnlySystemParam for &'a RemovedComponentEvents {}

// SAFETY: no component value access.
unsafe impl<'a> SystemParam for &'a RemovedComponentEvents {
    type State = ();
    type Item<'w, 's> = &'w RemovedComponentEvents;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.removed_components()
    }
}
