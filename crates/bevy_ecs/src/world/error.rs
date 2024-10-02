//! Contains error types returned by bevy's schedule.

use thiserror::Error;

use crate::{component::ComponentId, schedule::InternedScheduleLabel};

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
    #[error("The component with the ID {0:?} does not exist on the entity.")]
    NoSuchComponent(ComponentId),
    /// The component with the given [`ComponentId`] was requested mutably more than once.
    #[error("The component with the ID {0:?} was requested mutably more than once.")]
    AliasedMutability(ComponentId),
}
