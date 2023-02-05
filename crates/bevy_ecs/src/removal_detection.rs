//! Alerting events when a component is removed from an entity.

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentId, ComponentIdFor},
    entity::Entity,
    event::{Events, ManualEventIterator, ManualEventReader},
    prelude::Local,
    storage::SparseSet,
    system::{ReadOnlySystemParam, SystemMeta, SystemParam},
    world::World,
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
#[derive(Debug, Clone)]
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

/// Wrapper around a map of components to [`Events<RemovedComponentEntity>`].
/// So that we can find the events without naming the type directly.
#[derive(Default, Debug)]
pub struct RemovedComponentEvents {
    event_sets: SparseSet<ComponentId, Events<RemovedComponentEntity>>,
}

impl RemovedComponentEvents {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self) {
        for (_component_id, events) in self.event_sets.iter_mut() {
            events.update();
        }
    }

    pub fn get(
        &self,
        component_id: impl Into<ComponentId>,
    ) -> Option<&Events<RemovedComponentEntity>> {
        self.event_sets.get(component_id.into())
    }

    pub fn send(&mut self, component_id: impl Into<ComponentId>, entity: Entity) {
        self.event_sets
            .get_or_insert_with(component_id.into(), Default::default)
            .send(RemovedComponentEntity(entity));
    }
}

/// A [`SystemParam`] that grants access to the entities that had their `T` [`Component`] removed.
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
    component_id: Local<'s, ComponentIdFor<T>>,
    reader: Local<'s, RemovedComponentReader<T>>,
    event_sets: &'w RemovedComponentEvents,
}

/// Iterator over entities that had a specific component removed.
///
/// See [`RemovedComponents`].
pub type RemovedIter<'a> = iter::Map<
    iter::Flatten<option::IntoIter<iter::Cloned<ManualEventIterator<'a, RemovedComponentEntity>>>>,
    fn(RemovedComponentEntity) -> Entity,
>;

impl<'w, 's, T: Component> RemovedComponents<'w, 's, T> {
    pub fn iter(&mut self) -> RemovedIter<'_> {
        self.event_sets
            .get(**self.component_id)
            .map(|events| self.reader.iter(events).cloned())
            .into_iter()
            .flatten()
            .map(RemovedComponentEntity::into)
    }
}

impl<'a, 'w, 's: 'a, T> IntoIterator for &'a mut RemovedComponents<'w, 's, T>
where
    T: Component,
{
    type Item = Entity;
    type IntoIter = RemovedIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// SAFETY: Only reads World removed component events
unsafe impl<'a> ReadOnlySystemParam for &'a RemovedComponentEvents {}

// SAFETY: no component value access, removed component events can be read in parallel and are
// never mutably borrowed during system execution
unsafe impl<'a> SystemParam for &'a RemovedComponentEvents {
    type State = ();
    type Item<'w, 's> = &'w RemovedComponentEvents;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.removed_components()
    }
}
