//! Contains error types returned by bevy's schedule.

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

/// An error that occurs when cloning world failed.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldCloneError {
    /// World id allocation failed.
    #[error("More `bevy` `World`s have been created than is supported.")]
    WorldIdExhausted,
    /// Component clone handler failed to clone component.
    #[error("Component clone handler for component with ID {0:?} failed to clone the component.")]
    FailedToCloneComponent(ComponentId),
    /// Component clone handler failed to clone resource.
    #[error("Component clone handler for resource with ID {0:?} failed to clone the resource.")]
    FailedToCloneResource(ComponentId),
    /// Resource cloned from different thread than the one used to create it.
    #[error("Tried to clone non-send resource with ID {0:?} from a different thread than the one it was created from.")]
    NonSendResourceCloned(ComponentId),
    /// Component clone handler is set to `Ignore`.
    #[error("Component clone handler for component or resource with ID {0:?} is set to Ignore and can't be cloned.")]
    ComponentCantBeCloned(ComponentId),
}
