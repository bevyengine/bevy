//! Contains error types returned by bevy's schedule.

use alloc::vec::Vec;
use thiserror::Error;

use crate::{
    component::ComponentId,
    entity::{Entity, EntityDoesNotExistDetails},
    schedule::InternedScheduleLabel,
};

/// The error type returned by [`World::try_run_schedule`] if the provided schedule does not exist.
///
/// [`World::try_run_schedule`]: crate::world::World::try_run_schedule
#[derive(Error, Debug)]
#[error("The schedule with the label {0:?} was not found.")]
pub struct TryRunScheduleError(pub InternedScheduleLabel);

/// The error type returned by [`World::try_despawn`] if the provided entity does not exist.
///
/// [`World::try_despawn`]: crate::world::World::try_despawn
#[derive(Error, Debug, Clone, Copy)]
#[error("Could not despawn the entity with ID {0} because it {1}")]
pub struct TryDespawnError(pub Entity, pub EntityDoesNotExistDetails);

/// The error type returned by [`World::try_insert_batch`] and [`World::try_insert_batch_if_new`]
/// if any of the provided entities do not exist.
///
/// [`World::try_insert_batch`]: crate::world::World::try_insert_batch
/// [`World::try_insert_batch_if_new`]: crate::world::World::try_insert_batch_if_new
#[derive(Error, Debug, Clone)]
#[error("Could not insert bundles of type {0} into the entities with the following IDs because they did not exist: {1:?}")]
pub struct TryInsertBatchError(pub &'static str, pub Vec<Entity>);

/// An error that occurs when dynamically retrieving components from an entity.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityComponentError {
    /// The component with the given [`ComponentId`] does not exist on the entity.
    #[error("The component with ID {0:?} does not exist on the entity.")]
    MissingComponent(ComponentId),
    /// The component with the given [`ComponentId`] was requested mutably more than once.
    #[error("The component with ID {0:?} was requested mutably more than once.")]
    AliasedMutability(ComponentId),
}

/// An error that occurs when fetching entities mutably from a world.
#[derive(Error, Debug, Clone, Copy)]
pub enum EntityFetchError {
    /// The entity with the given ID does not exist.
    #[error("The entity with ID {0} {1}")]
    NoSuchEntity(Entity, EntityDoesNotExistDetails),
    /// The entity with the given ID was requested mutably more than once.
    #[error("The entity with ID {0} was requested mutably more than once")]
    AliasedMutability(Entity),
}

impl PartialEq for EntityFetchError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoSuchEntity(e1, _), Self::NoSuchEntity(e2, _)) if e1 == e2 => true,
            (Self::AliasedMutability(e1), Self::AliasedMutability(e2)) if e1 == e2 => true,
            _ => false,
        }
    }
}

impl Eq for EntityFetchError {}
