//! This module contains the error type used by commands.

use alloc::boxed::Box;
use thiserror::Error;

use crate::entity::{Entity, EntityDoesNotExistDetails};

/// An error that occurs when executing a command.
#[derive(Error, Debug)]
pub enum CommandError {
    /// The entity with the given ID does not exist.
    #[error("Command failed because the entity with ID {0} {1}")]
    NoSuchEntity(Entity, EntityDoesNotExistDetails),
    /// The command returned an error.
    #[error("Command returned an error: {0}")]
    CommandFailed(Box<dyn core::error::Error + Send + Sync + 'static>),
}
