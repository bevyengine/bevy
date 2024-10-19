//! Contains error types returned by bevy's schedule.

use derive_more::derive::{Display, Error};

use crate::{component::ComponentId, entity::Entity, schedule::InternedScheduleLabel};

/// The error type returned by [`World::try_run_schedule`] if the provided schedule does not exist.
///
/// [`World::try_run_schedule`]: crate::world::World::try_run_schedule
#[derive(Error, Display, Debug)]
#[display("The schedule with the label {_0:?} was not found.")]
#[error(ignore)]
pub struct TryRunScheduleError(pub InternedScheduleLabel);

/// An error that occurs when dynamically retrieving components from an entity.
#[derive(Error, Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityComponentError {
    /// The component with the given [`ComponentId`] does not exist on the entity.
    #[display("The component with ID {_0:?} does not exist on the entity.")]
    #[error(ignore)]
    MissingComponent(ComponentId),
    /// The component with the given [`ComponentId`] was requested mutably more than once.
    #[display("The component with ID {_0:?} was requested mutably more than once.")]
    #[error(ignore)]
    AliasedMutability(ComponentId),
}

/// An error that occurs when fetching entities mutably from a world.
#[derive(Error, Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityFetchError {
    /// The entity with the given ID does not exist.
    #[display("The entity with ID {_0:?} does not exist.")]
    #[error(ignore)]
    NoSuchEntity(Entity),
    /// The entity with the given ID was requested mutably more than once.
    #[display("The entity with ID {_0:?} was requested mutably more than once.")]
    #[error(ignore)]
    AliasedMutability(Entity),
}
