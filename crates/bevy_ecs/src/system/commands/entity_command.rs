//! This module contains the definition of the [`EntityCommand`] trait, as well as
//! blanket implementations of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`EntityCommands`](crate::system::EntityCommands).

use alloc::vec::Vec;
use log::info;

use crate::{
    bundle::{Bundle, InsertMode},
    change_detection::MaybeLocation,
    component::{Component, ComponentId, ComponentInfo},
    entity::{Entity, EntityClonerBuilder},
    event::Event,
    result::Result,
    system::{command::HandleError, Command, IntoObserverSystem},
    world::{error::EntityFetchError, EntityWorldMut, FromWorld, World},
};
use bevy_ptr::OwningPtr;

/// A command which gets executed for a given [`Entity`].
///
/// Should be used with [`EntityCommands::queue`](crate::system::EntityCommands::queue).
///
/// The `Out` generic parameter is the returned "output" of the command.
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
pub trait EntityCommand<Out = ()>: Send + 'static {
    /// Executes this command for the given [`Entity`] and
    /// returns a [`Result`] for error handling.
    fn apply(self, entity: EntityWorldMut) -> Out;
}
/// Passes in a specific entity to an [`EntityCommand`], resulting in a [`Command`] that
/// internally runs the [`EntityCommand`] on that entity.
///
// NOTE: This is a separate trait from `EntityCommand` because "result-returning entity commands" and
// "non-result returning entity commands" require different implementations, so they cannot be automatically
// implemented. And this isn't the type of implementation that we want to thrust on people implementing
// EntityCommand.
pub trait CommandWithEntity<Out> {
    /// Passes in a specific entity to an [`EntityCommand`], resulting in a [`Command`] that
    /// internally runs the [`EntityCommand`] on that entity.
    fn with_entity(self, entity: Entity) -> impl Command<Out> + HandleError<Out>;
}

impl<C: EntityCommand> CommandWithEntity<Result<(), EntityFetchError>> for C {
    fn with_entity(
        self,
        entity: Entity,
    ) -> impl Command<Result<(), EntityFetchError>> + HandleError<Result<(), EntityFetchError>>
    {
        move |world: &mut World| -> Result<(), EntityFetchError> {
            let entity = world.get_entity_mut(entity)?;
            self.apply(entity);
            Ok(())
        }
    }
}

impl<
        C: EntityCommand<Result<T, Err>>,
        T,
        Err: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
    > CommandWithEntity<Result<T, EntityCommandError<Err>>> for C
{
    fn with_entity(
        self,
        entity: Entity,
    ) -> impl Command<Result<T, EntityCommandError<Err>>> + HandleError<Result<T, EntityCommandError<Err>>>
    {
        move |world: &mut World| {
            let entity = world.get_entity_mut(entity)?;
            self.apply(entity)
                .map_err(EntityCommandError::CommandFailed)
        }
    }
}

/// An error that occurs when running an [`EntityCommand`] on a specific entity.
#[derive(thiserror::Error, Debug)]
pub enum EntityCommandError<E> {
    /// The entity this [`EntityCommand`] tried to run on could not be fetched.
    #[error(transparent)]
    EntityFetchError(#[from] EntityFetchError),
    /// An error that occurred while running the [`EntityCommand`].
    #[error("{0}")]
    CommandFailed(E),
}

impl<Out, F> EntityCommand<Out> for F
where
    F: FnOnce(EntityWorldMut) -> Out + Send + 'static,
{
    fn apply(self, entity: EntityWorldMut) -> Out {
        self(entity)
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity,
/// replacing any that were already present.
#[track_caller]
pub fn insert(bundle: impl Bundle) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.insert_with_caller(bundle, InsertMode::Replace, caller);
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity,
/// except for any that were already present.
#[track_caller]
pub fn insert_if_new(bundle: impl Bundle) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.insert_with_caller(bundle, InsertMode::Keep, caller);
    }
}

/// An [`EntityCommand`] that adds a dynamic component to an entity.
#[track_caller]
pub fn insert_by_id<T: Send + 'static>(component_id: ComponentId, value: T) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        // SAFETY:
        // - `component_id` safety is ensured by the caller
        // - `ptr` is valid within the `make` block
        OwningPtr::make(value, |ptr| unsafe {
            entity.insert_by_id_with_caller(component_id, ptr, caller);
        });
    }
}

/// An [`EntityCommand`] that adds a component to an entity using
/// the component's [`FromWorld`] implementation.
#[track_caller]
pub fn insert_from_world<T: Component + FromWorld>(mode: InsertMode) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        let value = entity.world_scope(|world| T::from_world(world));
        entity.insert_with_caller(value, mode, caller);
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity.
#[track_caller]
pub fn remove<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity,
/// as well as the required components for each component removed.
#[track_caller]
pub fn remove_with_requires<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_with_requires_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that removes a dynamic component from an entity.
#[track_caller]
pub fn remove_by_id(component_id: ComponentId) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_by_id_with_caller(component_id, caller);
    }
}

/// An [`EntityCommand`] that removes all components from an entity.
#[track_caller]
pub fn clear() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.clear_with_caller(caller);
    }
}

/// An [`EntityCommand`] that removes all components from an entity,
/// except for those in the given [`Bundle`].
#[track_caller]
pub fn retain<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.retain_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that despawns an entity.
///
/// # Note
///
/// This will also despawn any [`Children`](crate::hierarchy::Children) entities, and any other [`RelationshipTarget`](crate::relationship::RelationshipTarget) that is configured
/// to despawn descendants. This results in "recursive despawn" behavior.
#[track_caller]
pub fn despawn() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |entity: EntityWorldMut| {
        entity.despawn_with_caller(caller);
    }
}

/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
/// listening for events of type `E` targeting an entity
#[track_caller]
pub fn observe<E: Event, B: Bundle, M>(
    observer: impl IntoObserverSystem<E, B, M>,
) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.observe_with_caller(observer, caller);
    }
}

/// An [`EntityCommand`] that clones parts of an entity onto another entity,
/// configured through [`EntityClonerBuilder`].
pub fn clone_with(
    target: Entity,
    config: impl FnOnce(&mut EntityClonerBuilder) + Send + Sync + 'static,
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
