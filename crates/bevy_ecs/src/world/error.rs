//! Contains error types returned by bevy's schedule.

use alloc::vec::Vec;

use crate::{
    component::ComponentId,
    entity::{Entity, EntityDoesNotExistError},
    schedule::InternedScheduleLabel,
};

/// The error type returned by [`World::try_run_schedule`] if the provided schedule does not exist.
///
/// [`World::try_run_schedule`]: crate::world::World::try_run_schedule
#[derive(thiserror::Error, Debug)]
#[error("The schedule with the label {0:?} was not found.")]
pub struct TryRunScheduleError(pub InternedScheduleLabel);

/// The error type returned by [`World::try_insert_batch`] and [`World::try_insert_batch_if_new`]
/// if any of the provided entities do not exist.
///
/// [`World::try_insert_batch`]: crate::world::World::try_insert_batch
/// [`World::try_insert_batch_if_new`]: crate::world::World::try_insert_batch_if_new
#[derive(thiserror::Error, Debug, Clone)]
#[error("Could not insert bundles of type {bundle_type} into the entities with the following IDs because they do not exist: {entities:?}")]
pub struct TryInsertBatchError {
    /// The bundles' type name.
    pub bundle_type: &'static str,
    /// The IDs of the provided entities that do not exist.
    pub entities: Vec<Entity>,
}

/// An error that occurs when a specified [`Entity`] could not be despawned.
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("Could not despawn entity: {0}")]
pub struct EntityDespawnError(#[from] pub EntityMutableFetchError);

/// An error that occurs when dynamically retrieving components from an entity.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityComponentError {
    /// The component with the given [`ComponentId`] does not exist on the entity.
    #[error("The component with ID {0:?} does not exist on the entity.")]
    MissingComponent(ComponentId),
    /// The component with the given [`ComponentId`] was requested mutably more than once.
    #[error("The component with ID {0:?} was requested mutably more than once.")]
    AliasedMutability(ComponentId),
}

/// An error that occurs when fetching entities mutably from a world.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityMutableFetchError {
    /// The entity with the given ID does not exist.
    #[error(transparent)]
    EntityDoesNotExist(#[from] EntityDoesNotExistError),
    /// The entity with the given ID was requested mutably more than once.
    #[error("The entity with ID {0} was requested mutably more than once")]
    AliasedMutability(Entity),
}

/// An error that occurs when getting a resource of a given type in a world.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceFetchError {
    /// The resource has never been initialized or registered with the world.
    #[error("The resource has never been initialized or registered with the world. Did you forget to add it using `app.insert_resource` / `app.init_resource`?")]
    NotRegistered,
    /// The resource with the given [`ComponentId`] does not currently exist in the world.
    #[error("The resource with ID {0:?} does not currently exist in the world.")]
    DoesNotExist(ComponentId),
    /// Cannot get access to the resource with the given [`ComponentId`] in the world as it conflicts with an on going operation.
    #[error("Cannot get access to the resource with ID {0:?} in the world as it conflicts with an on going operation.")]
    NoResourceAccess(ComponentId),
}
