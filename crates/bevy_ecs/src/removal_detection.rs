//! Alerting events when a component is removed from an entity.

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentId, ComponentIdFor, Tick},
    entity::Entity,
    event::{Event, EventId, EventIterator, EventIteratorWithId, Events, ManualEventReader},
    prelude::Local,
    storage::SparseSet,
    system::{ReadOnlySystemParam, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use std::{
    fmt::Debug,
    iter,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    option,
};

/// Wrapper around [`Entity`] for [`RemovedComponents`].
/// Internally, `RemovedComponents` uses these as an `Events<RemovedComponentEntity>`.
#[derive(Event, Debug, Clone)]
pub struct RemovedComponentEntity(Entity);

impl From<RemovedComponentEntity> for Entity {
    fn from(value: RemovedComponentEntity) -> Self {
        value.0
    }
}

/// Wrapper around a [`ManualEventReader<RemovedComponentEntity>`] so that we
/// can differentiate events between components.
#[derive(Debug)]
pub struct RemovedComponentReader<T>
where
    T: Component,
{
    reader: ManualEventReader<RemovedComponentEntity>,
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
    type Target = ManualEventReader<RemovedComponentEntity>;
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

/// A [`SystemParam`] that grants access to the entities that had their `T` [`Component`] removed.
///
/// This acts effectively the same as an [`EventReader`](crate::event::EventReader).
///
/// Note that this does not allow you to see which data existed before removal.
/// If you need this, you will need to track the component data value on your own,
/// using a regularly scheduled system that requests `Query<(Entity, &T), Changed<T>>`
/// and stores the data somewhere safe to later cross-reference.
///
/// If you are using `bevy_ecs` as a standalone crate,
/// note that the `RemovedComponents` list will not be automatically cleared for you,
/// and will need to be manually flushed using [`World::clear_trackers`](crate::world::World::clear_trackers)
///
/// For users of `bevy` and `bevy_app`, this is automatically done in `bevy_app::App::update`.
/// For the main world, [`World::clear_trackers`](crate::world::World::clear_trackers) is run after the main schedule is run and after
/// `SubApp`'s have run.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::removal_detection::RemovedComponents;
/// #
/// # #[derive(Component)]
/// # struct MyComponent;
/// fn react_on_removal(mut removed: RemovedComponents<MyComponent>) {
///     removed.iter().for_each(|removed_entity| println!("{:?}", removed_entity));
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
    /// Fetch underlying [`ManualEventReader`].
    pub fn reader(&self) -> &ManualEventReader<RemovedComponentEntity> {
        &self.reader
    }

    /// Fetch underlying [`ManualEventReader`] mutably.
    pub fn reader_mut(&mut self) -> &mut ManualEventReader<RemovedComponentEntity> {
        &mut self.reader
    }

    /// Fetch underlying [`Events`].
    pub fn events(&self) -> Option<&Events<RemovedComponentEntity>> {
        self.event_sets.get(self.component_id.get())
    }

    /// Destructures to get a mutable reference to the `ManualEventReader`
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

    /// Iterates over the events this [`RemovedComponents`] has not seen yet. This updates the
    /// [`RemovedComponents`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    #[deprecated = "use `.read()` instead."]
    pub fn iter(&mut self) -> RemovedIter<'_> {
        self.read()
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`] of the events.
    pub fn read_with_id(&mut self) -> RemovedIterWithId<'_> {
        self.reader_mut_with_events()
            .map(|(reader, events)| reader.read_with_id(events))
            .into_iter()
            .flatten()
            .map(map_id_events)
    }

    /// Like [`iter`](Self::iter), except also returning the [`EventId`] of the events.
    #[deprecated = "use `.read_with_id()` instead."]
    pub fn iter_with_id(&mut self) -> RemovedIterWithId<'_> {
        self.read_with_id()
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
            .map(|events| self.reader.is_empty(events))
            .unwrap_or(true)
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
