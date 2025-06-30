//! Error handling for Bevy systems, commands, and observers.
//!
//! When a system is added to a [`Schedule`], and its return type is that of [`Result`], then Bevy
//! considers those systems to be "fallible", and the ECS scheduler will special-case the [`Err`]
//! variant of the returned `Result`.
//!
//! All [`BevyError`]s returned by a system, observer or command are handled by an "error handler". By default, the
//! [`panic`] error handler function is used, resulting in a panic with the error message attached.
//!
//! You can change the default behavior by registering a custom error handler:
//! Use [`DefaultErrorHandler`] to set a custom error handler function for a world,
//! or `App::set_error_handler` for a whole app.
//! In practice, this is generally feature-flagged: panicking or loudly logging errors in development,
//! and quietly logging or ignoring them in production to avoid crashing the app.
//!
//! Bevy provides a number of pre-built error-handlers for you to use:
//!
//! - [`panic`] – panics with the system error
//! - [`error`] – logs the system error at the `error` level
//! - [`warn`] – logs the system error at the `warn` level
//! - [`info`] – logs the system error at the `info` level
//! - [`debug`] – logs the system error at the `debug` level
//! - [`trace`] – logs the system error at the `trace` level
//! - [`ignore`] – ignores the system error
//!
//! However, you can use any custom error handler logic by providing your own function (or
//! non-capturing closure that coerces to the function signature) as long as it matches the
//! signature:
//!
//! ```rust,ignore
//! fn(BevyError, ErrorContext)
//! ```
//!
//! The [`ErrorContext`] allows you to access additional details relevant to providing
//! context surrounding the error – such as the system's [`name`] – in your error messages.
//!
//! ```rust, ignore
//! use bevy_ecs::error::{BevyError, ErrorContext, DefaultErrorHandler};
//! use log::trace;
//!
//! fn my_error_handler(error: BevyError, ctx: ErrorContext) {
//!    if ctx.name().ends_with("plz_ignore") {
//!       trace!("Nothing to see here, move along.");
//!       return;
//!   }
//!   bevy_ecs::error::error(error, ctx);
//! }
//!
//! fn main() {
//!     let mut world = World::new();
//!     world.insert_resource(DefaultErrorHandler(my_error_handler));
//!     // Use your world here
//! }
//! ```
//!
//! If you need special handling of individual fallible systems, you can use Bevy's [`system piping
//! feature`] to capture the [`Result`] output of the system and handle it accordingly.
//!
//! When working with commands, you can handle the result of each command separately using the [`HandleError::handle_error_with`] method.
//!
//! [`Schedule`]: crate::schedule::Schedule
//! [`panic`]: panic()
//! [`World`]: crate::world::World
//! [`System`]: crate::system::System
//! [`name`]: crate::system::System::name
//! [`system piping feature`]: crate::system::In

mod bevy_error;
mod command_handling;
mod handler;

pub use bevy_error::*;
pub use command_handling::*;
pub use handler::*;

/// A result type for use in fallible systems, commands and observers.
///
/// The [`BevyError`] type is a type-erased error type with optional Bevy-specific diagnostics.
pub type Result<T = (), E = BevyError> = core::result::Result<T, E>;
