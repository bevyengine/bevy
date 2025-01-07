//! This module contains the definition of the [`EntityCommand`] trait, as well as
//! blanket implementations of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`EntityCommands`](crate::system::EntityCommands).

use alloc::vec::Vec;
use log::info;

#[cfg(feature = "track_location")]
use core::panic::Location;

use crate::{
    bundle::{Bundle, InsertMode},
    component::{Component, ComponentId, ComponentInfo},
    entity::{Entity, EntityCloneBuilder},
    event::Event,
    result::Result,
    system::{Command, CommandError, IntoObserverSystem},
    world::{EntityWorldMut, FromWorld, World},
};
use bevy_ptr::OwningPtr;

/// A [`Command`] which gets executed for a given [`Entity`].
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::EntityCommand;
/// #
/// # #[derive(Component, PartialEq)]
/// # struct Name(String);
/// # impl Name {
/// #   fn new(s: String) -> Self { Name(s) }
/// #   fn as_str(&self) -> &str { &self.0 }
/// # }
///
/// #[derive(Resource, Default)]
/// struct Counter(i64);
///
/// /// A `Command` which names an entity based on a global counter.
/// fn count_name(entity: Entity, world: &mut World) {
///     // Get the current value of the counter, and increment it for next time.
///     let mut counter = world.resource_mut::<Counter>();
///     let i = counter.0;
///     counter.0 += 1;
///
///     // Name the entity after the value of the counter.
///     world.entity_mut(entity).insert(Name::new(format!("Entity #{i}")));
/// }
///
/// // App creation boilerplate omitted...
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #
/// # let mut setup_schedule = Schedule::default();
/// # setup_schedule.add_systems(setup);
/// # let mut assert_schedule = Schedule::default();
/// # assert_schedule.add_systems(assert_names);
/// #
/// # setup_schedule.run(&mut world);
/// # assert_schedule.run(&mut world);
///
/// fn setup(mut commands: Commands) {
///     commands.spawn_empty().queue(count_name);
///     commands.spawn_empty().queue(count_name);
/// }
///
/// fn assert_names(named: Query<&Name>) {
///     // We use a HashSet because we do not care about the order.
///     let names: HashSet<_> = named.iter().map(Name::as_str).collect();
///     assert_eq!(names, HashSet::from_iter(["Entity #0", "Entity #1"]));
/// }
/// ```
///
/// # Note on Generic
///
/// The `Marker` generic is necessary to allow multiple blanket implementations
/// of `EntityCommand` for closures, like so:
/// ```ignore (This would conflict with the real implementations)
/// impl EntityCommand for FnOnce(Entity, &mut World)
/// impl EntityCommand<World> for FnOnce(EntityWorldMut)
/// impl EntityCommand<Result> for FnOnce(Entity, &mut World) -> Result
/// impl EntityCommand<(World, Result)> for FnOnce(EntityWorldMut) -> Result
/// ```
/// Without the generic, Rust would consider the implementations to be conflicting.
///
/// The type used for `Marker` has no connection to anything else in the implementation.
pub trait EntityCommand<Marker = ()>: Send + 'static {
    /// Executes this command for the given [`Entity`] and
    /// returns a [`Result`] for error handling.
    fn apply(self, entity: Entity, world: &mut World) -> Result;

    /// Returns a [`Command`] which executes this [`EntityCommand`] for the given [`Entity`].
    ///
    /// This method is called when adding an [`EntityCommand`] to a command queue via [`Commands`](crate::system::Commands).
    /// You can override the provided implementation if you can return a `Command` with a smaller memory
    /// footprint than `(Entity, Self)`.
    /// In most cases the provided implementation is sufficient.
    #[must_use = "commands do nothing unless applied to a `World`"]
    fn with_entity(self, entity: Entity) -> impl Command<(Result, CommandError)>
    where
        Self: Sized,
    {
        move |world: &mut World| -> Result<(), CommandError> {
            if world.entities().contains(entity) {
                match self.apply(entity, world) {
                    Ok(_) => Ok(()),
                    Err(error) => Err(CommandError::CommandFailed(error)),
                }
            } else {
                Err(CommandError::NoSuchEntity(
                    entity,
                    world
                        .entities()
                        .entity_does_not_exist_error_details_message(entity),
                ))
            }
        }
    }
}

impl<F> EntityCommand for F
where
    F: FnOnce(Entity, &mut World) + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) -> Result {
        self(id, world);
        Ok(())
    }
}

impl<F> EntityCommand<Result> for F
where
    F: FnOnce(Entity, &mut World) -> Result + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) -> Result {
        self(id, world)
    }
}

impl<F> EntityCommand<World> for F
where
    F: FnOnce(EntityWorldMut) + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) -> Result {
        self(world.entity_mut(id));
        Ok(())
    }
}

impl<F> EntityCommand<(World, Result)> for F
where
    F: FnOnce(EntityWorldMut) -> Result + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) -> Result {
        self(world.entity_mut(id))
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity,
/// replacing any that were already present.
#[track_caller]
pub fn insert(bundle: impl Bundle) -> impl EntityCommand<World> {
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |mut entity: EntityWorldMut| {
        entity.insert_with_caller(
            bundle,
            InsertMode::Replace,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity,
/// except for any that were already present.
#[track_caller]
pub fn insert_if_new(bundle: impl Bundle) -> impl EntityCommand<World> {
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |mut entity: EntityWorldMut| {
        entity.insert_with_caller(
            bundle,
            InsertMode::Keep,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// An [`EntityCommand`] that adds a dynamic component to an entity.
#[track_caller]
pub fn insert_by_id<T: Send + 'static>(
    component_id: ComponentId,
    value: T,
) -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        // SAFETY:
        // - `component_id` safety is ensured by the caller
        // - `ptr` is valid within the `make` block
        OwningPtr::make(value, |ptr| unsafe {
            entity.insert_by_id(component_id, ptr);
        });
    }
}

/// An [`EntityCommand`] that adds a component to an entity using
/// the component's [`FromWorld`] implementation.
#[track_caller]
pub fn insert_from_world<T: Component + FromWorld>(mode: InsertMode) -> impl EntityCommand {
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |entity: Entity, world: &mut World| {
        let value = T::from_world(world);
        let mut entity = world.entity_mut(entity);
        entity.insert_with_caller(
            value,
            mode,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity.
pub fn remove<T: Bundle>() -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.remove::<T>();
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity,
/// as well as the required components for each component removed.
pub fn remove_with_requires<T: Bundle>() -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.remove_with_requires::<T>();
    }
}

/// An [`EntityCommand`] that removes a dynamic component from an entity.
pub fn remove_by_id(component_id: ComponentId) -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.remove_by_id(component_id);
    }
}

/// An [`EntityCommand`] that removes all components from an entity.
pub fn clear() -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.clear();
    }
}

/// An [`EntityCommand`] that removes all components from an entity,
/// except for those in the given [`Bundle`].
pub fn retain<T: Bundle>() -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.retain::<T>();
    }
}

/// An [`EntityCommand`] that despawns an entity.
///
/// # Note
///
/// This won't clean up external references to the entity (such as parent-child relationships
/// if you're using `bevy_hierarchy`), which may leave the world in an invalid state.
pub fn despawn() -> impl EntityCommand<World> {
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |entity: EntityWorldMut| {
        entity.despawn_with_caller(
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
/// listening for events of type `E` targeting an entity
pub fn observe<E: Event, B: Bundle, M>(
    observer: impl IntoObserverSystem<E, B, M>,
) -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.observe(observer);
    }
}

/// An [`EntityCommand`] that clones parts of an entity onto another entity,
/// configured through [`EntityCloneBuilder`].
pub fn clone_with(
    target: Entity,
    config: impl FnOnce(&mut EntityCloneBuilder) + Send + Sync + 'static,
) -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.clone_with(target, config);
    }
}

/// An [`EntityCommand`] that clones the specified components of an entity
/// and inserts them into another entity.
pub fn clone_components<B: Bundle>(target: Entity) -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.clone_components::<B>(target);
    }
}

/// An [`EntityCommand`] that clones the specified components of an entity
/// and inserts them into another entity, then removes them from the original entity.
pub fn move_components<B: Bundle>(target: Entity) -> impl EntityCommand<World> {
    move |mut entity: EntityWorldMut| {
        entity.move_components::<B>(target);
    }
}

/// An [`EntityCommand`] that logs the components of an entity.
pub fn log_components() -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        let debug_infos: Vec<_> = world
            .inspect_entity(entity)
            .map(ComponentInfo::name)
            .collect();
        info!("Entity {entity}: {debug_infos:?}");
    }
}
