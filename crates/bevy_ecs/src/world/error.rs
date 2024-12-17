//! Contains error types returned by bevy's schedule.

use thiserror::Error;

use crate::{component::ComponentId, entity::Entity, schedule::InternedScheduleLabel};

use super::unsafe_world_cell::UnsafeWorldCell;

/// The error type returned by [`World::try_run_schedule`] if the provided schedule does not exist.
///
/// [`World::try_run_schedule`]: crate::world::World::try_run_schedule
#[derive(Error, Debug)]
#[error("The schedule with the label {0:?} was not found.")]
pub struct TryRunScheduleError(pub InternedScheduleLabel);

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
#[derive(Clone, Copy)]
pub enum EntityFetchError<'w> {
    /// The entity with the given ID does not exist.
    NoSuchEntity(Entity, UnsafeWorldCell<'w>),
    /// The entity with the given ID was requested mutably more than once.
    AliasedMutability(Entity),
}

impl<'w> core::error::Error for EntityFetchError<'w> {}

impl<'w> core::fmt::Display for EntityFetchError<'w> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            Self::NoSuchEntity(entity, world) => {
                write!(
                    f,
                    "Entity {entity} {}",
                    world
                        .entities()
                        .entity_does_not_exist_error_details_message(entity)
                )
            }
            Self::AliasedMutability(entity) => {
                write!(f, "Entity {entity} was requested mutably more than once")
            }
        }
    }
}

impl<'w> core::fmt::Debug for EntityFetchError<'w> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NoSuchEntity(entity, world) => {
                write!(
                    f,
                    "NoSuchEntity({entity} {})",
                    world
                        .entities()
                        .entity_does_not_exist_error_details_message(entity)
                )
            }
            Self::AliasedMutability(entity) => write!(f, "AliasedMutability({entity})"),
        }
    }
}

impl<'w> PartialEq for EntityFetchError<'w> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoSuchEntity(e1, _), Self::NoSuchEntity(e2, _)) if e1 == e2 => true,
            (Self::AliasedMutability(e1), Self::AliasedMutability(e2)) if e1 == e2 => true,
            _ => false,
        }
    }
}

impl<'w> Eq for EntityFetchError<'w> {}
