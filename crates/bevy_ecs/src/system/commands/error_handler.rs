//! This module contains convenience functions that return simple error handlers
//! for use with [`Commands::queue_handled`](super::Commands::queue_handled) and [`EntityCommands::queue_handled`](super::EntityCommands::queue_handled).

use crate::{result::Error, world::World};
use log::{error, warn};

/// An error handler that does nothing.
pub fn silent() -> fn(&mut World, Error) {
    |_, _| {}
}

/// An error handler that accepts an error and logs it with [`warn!`].
pub fn warn() -> fn(&mut World, Error) {
    |_, error| warn!("{error}")
}

/// An error handler that accepts an error and logs it with [`error!`].
pub fn error() -> fn(&mut World, Error) {
    |_, error| error!("{error}")
}

/// An error handler that accepts an error and panics with the error in
/// the panic message.
pub fn panic() -> fn(&mut World, Error) {
    |_, error| panic!("{error}")
}

/// The default error handler. This defaults to [`panic()`]. If the
/// `configurable_error_handler` cargo feature is enabled, then
/// `GLOBAL_ERROR_HANDLER` will be used instead, enabling error handler customization.
#[cfg(not(feature = "configurable_error_handler"))]
#[inline]
pub fn default() -> fn(&mut World, Error) {
    panic()
}

/// A global error handler. This can be set at startup, as long as it is set before
/// any uses. This should generally be configured _before_ initializing the app.
///
/// If the `configurable_error_handler` cargo feature is enabled, this will be used
/// by default.
///
/// This should be set in the following way:
///
/// ```
/// # use bevy_ecs::system::error_handler::{GLOBAL_ERROR_HANDLER, warn};
/// GLOBAL_ERROR_HANDLER.set(warn());
/// // initialize Bevy App here
/// ```
#[cfg(feature = "configurable_error_handler")]
pub static GLOBAL_ERROR_HANDLER: std::sync::OnceLock<fn(&mut World, Error)> =
    std::sync::OnceLock::new();

/// The default error handler. This defaults to [`panic()`]. If the
/// `configurable_error_handler` cargo feature is enabled, then
/// [`GLOBAL_ERROR_HANDLER`] will be used instead, enabling error handler customization.
#[cfg(feature = "configurable_error_handler")]
#[inline]
pub fn default() -> fn(&mut World, Error) {
    *GLOBAL_ERROR_HANDLER.get_or_init(|| panic())
}
