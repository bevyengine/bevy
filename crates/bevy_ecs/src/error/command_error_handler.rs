//! This module contains convenience functions that return simple error handlers
//! for use with [`Commands::queue_handled`](super::Commands::queue_handled) and [`EntityCommands::queue_handled`](super::EntityCommands::queue_handled).

use crate::{error::BevyError, world::World};
use log::{error, warn};

/// An error handler that does nothing.
pub fn silent() -> fn(&mut World, BevyError) {
    |_, _| {}
}

/// An error handler that accepts an error and logs it with [`warn!`].
pub fn warn() -> fn(&mut World, BevyError) {
    |_, error| warn!("{error}")
}

/// An error handler that accepts an error and logs it with [`error!`].
pub fn error() -> fn(&mut World, BevyError) {
    |_, error| error!("{error}")
}

/// An error handler that accepts an error and panics with the error in
/// the panic message.
pub fn panic() -> fn(&mut World, BevyError) {
    |_, error| panic!("{error}")
}

/// A global error handler. This can be set at startup, as long as it is set before
/// any uses. This should generally be configured _before_ initializing the app.
///
/// This should be set in the following way:
///
/// ```
/// # use bevy_ecs::system::error_handler::{GLOBAL_ERROR_HANDLER, warn};
/// GLOBAL_ERROR_HANDLER.set(warn());
/// // initialize Bevy App here
/// ```
pub static GLOBAL_ERROR_HANDLER: std::sync::OnceLock<fn(&mut World, BevyError)> =
    std::sync::OnceLock::new();

/// The default error handler. This defaults to [`panic()`],
/// but if set, the [`GLOBAL_ERROR_HANDLER`] will be used instead, enabling error handler customization.
#[inline]
pub fn default_error_handler() -> fn(&mut World, BevyError) {
    *GLOBAL_ERROR_HANDLER.get_or_init(|| panic())
}
