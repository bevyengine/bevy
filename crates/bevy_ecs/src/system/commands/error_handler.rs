//! This module contains convenience functions that return simple error handlers
//! for use with the following methods:
//! - [`Commands::queue_fallible_with`](super::Commands::queue_fallible_with)
//! - [`Commands::override_error_handler`](super::Commands::override_error_handler)
//! - [`EntityCommands::queue_with`](super::EntityCommands::queue_with)
//! - [`EntityCommands::override_error_handler`](super::EntityCommands::override_error_handler)
//! - [`EntityEntryCommands::override_error_handler`](super::EntityEntryCommands::override_error_handler)

use log::{error, warn};

use crate::{system::CommandError, world::World};

/// An error handler that does nothing.
pub fn silent() -> fn(&mut World, CommandError) {
    |_, _| {}
}

/// An error handler that accepts an error and logs it with [`warn!`].
pub fn warn() -> fn(&mut World, CommandError) {
    |_, error| warn!("{error}")
}

/// An error handler that accepts an error and logs it with [`error!`].
pub fn error() -> fn(&mut World, CommandError) {
    |_, error| error!("{error}")
}

/// An error handler that accepts an error and panics with the error in
/// the panic message.
pub fn panic() -> fn(&mut World, CommandError) {
    |_, error| panic!("{error}")
}
