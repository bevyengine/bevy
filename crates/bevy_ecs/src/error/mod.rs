//! Error handling for "fallible" systems.
//!
//! When a system is added to a [`Schedule`], and its return type is that of [`Result`], then Bevy
//! considers those systems to be "fallible", and the ECS scheduler will special-case the [`Err`]
//! variant of the returned `Result`.
//!
//! All [`BevyError`]s returned by a system are handled by an "error handler". By default, the
//! [`panic`] error handler function is used, resulting in a panic with the error message attached.
//!
//! You can change the default behavior by registering a custom error handler, either globally or
//! per `Schedule`:
//!
//! - [`App::set_system_error_handler`] sets the global error handler for all systems of the
//!   current [`World`].
//! - [`Schedule::set_error_handler`] sets the error handler for all systems of that schedule.
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
//! fn(BevyError, SystemErrorContext)
//! ```
//!
//! The [`SystemErrorContext`] allows you to access additional details relevant to providing
//! context surrounding the system error – such as the system's [`name`] – in your error messages.
//!
//! For example:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_ecs::schedule::ScheduleLabel;
//! # use log::trace;
//! # fn update() -> Result { Ok(()) }
//! # #[derive(ScheduleLabel, Hash, Debug, PartialEq, Eq, Clone, Copy)]
//! # struct MySchedule;
//! # fn main() {
//! let mut schedule = Schedule::new(MySchedule);
//! schedule.add_systems(update);
//! schedule.set_error_handler(|error, ctx| {
//!     if ctx.name.ends_with("update") {
//!         trace!("Nothing to see here, move along.");
//!         return;
//!     }
//!
//!     bevy_ecs::error::error(error, ctx);
//! });
//! # }
//! ```
//!
//! If you need special handling of individual fallible systems, you can use Bevy's [`system piping
//! feature`] to capture the `Result` output of the system and handle it accordingly.
//!
//! [`Schedule`]: crate::schedule::Schedule
//! [`panic`]: panic()
//! [`World`]: crate::world::World
//! [`Schedule::set_error_handler`]: crate::schedule::Schedule::set_error_handler
//! [`System`]: crate::system::System
//! [`name`]: crate::system::System::name
//! [`App::set_system_error_handler`]: ../../bevy_app/struct.App.html#method.set_system_error_handler
//! [`system piping feature`]: crate::system::In

mod bevy_error;
mod handler;

pub use bevy_error::*;
pub use handler::*;

/// A result type for use in fallible systems.
pub type Result<T = (), E = BevyError> = core::result::Result<T, E>;
