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
    system::{error_handler, Command, IntoObserverSystem},
    world::{error::EntityFetchError, EntityWorldMut, FromWorld, World},
};
use bevy_ptr::OwningPtr;
use thiserror::Error;

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
/// fn count_name(mut entity: EntityWorldMut) {
///     // Get the current value of the counter, and increment it for next time.
///     let i = {
///         let mut counter = entity.resource_mut::<Counter>();
///         let i = counter.0;
///         counter.0 += 1;
///         i
///     };
///     // Name the entity after the value of the counter.
///     entity.insert(Name::new(format!("Entity #{i}")));
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
pub trait EntityCommand<T = ()>: Send + 'static {
    /// Executes this command for the given [`Entity`] and
    /// returns a [`Result`] for error handling.
    fn apply(self, entity: EntityWorldMut) -> T;
}
/// Passes in a specific entity to an [`EntityCommand`], resulting in a [`Command`] that
/// internally runs the [`EntityCommand`] on that entity.
pub trait CommandWithEntity<T> {
    /// Passes in a specific entity to an [`EntityCommand`], resulting in a [`Command`] that
    /// internally runs the [`EntityCommand`] on that entity.
    fn with_entity(self, entity: Entity) -> impl Command<T>;
}

impl<C: EntityCommand<()>> CommandWithEntity<Result<(), EntityFetchError>> for C {
    fn with_entity(self, entity: Entity) -> impl Command<Result<(), EntityFetchError>> {
        move |world: &mut World| {
            let entity = world.get_entity_mut(entity)?;
            self.apply(entity);
            Ok(())
        }
    }
}

impl<C: EntityCommand<Result<T, Err>>, T, Err> CommandWithEntity<Result<T, EntityCommandError<Err>>>
    for C
{
    fn with_entity(self, entity: Entity) -> impl Command<Result<T, EntityCommandError<Err>>> {
        move |world: &mut World| {
            let entity = world.get_entity_mut(entity)?;
            match self.apply(entity) {
                Ok(result) => Ok(result),
                Err(err) => Err(EntityCommandError::Error(err)),
            }
        }
    }
}

/// Takes a [`EntityCommand`] that returns a Result and uses a given error handler function to convert it into
/// a [`EntityCommand`] that internally handles an error if it occurs and returns `()`.
pub trait HandleEntityError {
    /// Takes a [`EntityCommand`] that returns a Result and uses a given error handler function to convert it into
    /// a [`EntityCommand`] that internally handles an error if it occurs and returns `()`.
    fn handle_error_with(
        self,
        error_handler: fn(&mut World, crate::result::Error),
    ) -> impl EntityCommand;
    /// Takes a [`EntityCommand`] that returns a Result and uses the default error handler function to convert it into
    /// a [`EntityCommand`] that internally handles an error if it occurs and returns `()`.
    fn handle_error(self) -> impl EntityCommand
    where
        Self: Sized,
    {
        self.handle_error_with(error_handler::default())
    }
}

impl<C: EntityCommand<crate::result::Result>> HandleEntityError for C {
    fn handle_error_with(
        self,
        error_handler: fn(&mut World, crate::result::Error),
    ) -> impl EntityCommand {
        move |entity: EntityWorldMut| {
            let location = entity.location();
            let id = entity.id();
            // This is broken up into parts so we can pass in the world to the error handler
            // after EntityWorldMut is consumed
            let world = entity.into_world_mut();
            // SAFETY: location has not changed and entity is valid
            match self.apply(unsafe { EntityWorldMut::new(world, id, location) }) {
                Ok(_) => {}
                Err(err) => (error_handler)(world, err),
            }
        }
    }
}

/// Takes a [`EntityCommand`] that returns a [`Result`] with an error that can be converted into the [`Error`] type
/// and returns a [`EntityCommand`] that internally converts that error to [`Error`] (if it occurs).
pub fn map_entity_command_err<T, E: Into<crate::result::Error>>(
    command: impl EntityCommand<Result<T, E>>,
) -> impl EntityCommand<Result<T, crate::result::Error>> {
    move |entity: EntityWorldMut| match command.apply(entity) {
        Ok(result) => Ok(result),
        Err(err) => Err(err.into()),
    }
}

/// An error that occurs when running an [`EntityCommand`] on a specific entity.
#[derive(Error, Debug)]
pub enum EntityCommandError<E> {
    /// The entity this [`EntityCommand`] tried to run on could not be fetched.
    #[error(transparent)]
    EntityFetchError(#[from] EntityFetchError),
    /// An error that occurred while running the [`EntityCommand`].
    #[error("{0}")]
    Error(E),
}

impl<T, F> EntityCommand<T> for F
where
    F: FnOnce(EntityWorldMut) -> T + Send + 'static,
{
    fn apply(self, entity: EntityWorldMut) -> T {
        self(entity)
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity,
/// replacing any that were already present.
#[track_caller]
pub fn insert(bundle: impl Bundle) -> impl EntityCommand {
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
pub fn insert_if_new(bundle: impl Bundle) -> impl EntityCommand {
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
pub fn insert_by_id<T: Send + 'static>(component_id: ComponentId, value: T) -> impl EntityCommand {
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
    move |mut entity: EntityWorldMut| {
        let value = entity.world_scope(|world| T::from_world(world));
        entity.insert_with_caller(
            value,
            mode,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity.
pub fn remove<T: Bundle>() -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.remove::<T>();
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity,
/// as well as the required components for each component removed.
pub fn remove_with_requires<T: Bundle>() -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.remove_with_requires::<T>();
    }
}

/// An [`EntityCommand`] that removes a dynamic component from an entity.
pub fn remove_by_id(component_id: ComponentId) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.remove_by_id(component_id);
    }
}

/// An [`EntityCommand`] that removes all components from an entity.
pub fn clear() -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.clear();
    }
}

/// An [`EntityCommand`] that removes all components from an entity,
/// except for those in the given [`Bundle`].
pub fn retain<T: Bundle>() -> impl EntityCommand {
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
pub fn despawn() -> impl EntityCommand {
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
) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.observe(observer);
    }
}

/// An [`EntityCommand`] that clones parts of an entity onto another entity,
/// configured through [`EntityCloneBuilder`].
pub fn clone_with(
    target: Entity,
    config: impl FnOnce(&mut EntityCloneBuilder) + Send + Sync + 'static,
) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.clone_with(target, config);
    }
}

/// An [`EntityCommand`] that clones the specified components of an entity
/// and inserts them into another entity.
pub fn clone_components<B: Bundle>(target: Entity) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.clone_components::<B>(target);
    }
}

/// An [`EntityCommand`] that clones the specified components of an entity
/// and inserts them into another entity, then removes them from the original entity.
pub fn move_components<B: Bundle>(target: Entity) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.move_components::<B>(target);
    }
}

/// An [`EntityCommand`] that logs the components of an entity.
pub fn log_components() -> impl EntityCommand {
    move |entity: EntityWorldMut| {
        let debug_infos: Vec<_> = entity
            .world()
            .inspect_entity(entity.id())
            .map(ComponentInfo::name)
            .collect();
        info!("Entity {}: {debug_infos:?}", entity.id());
    }
}
