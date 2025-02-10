//! Error handling for "fallible" systems.
//!
//! When a system is added to a [`Schedule`], and its return type is that of [`Result`], then Bevy
//! considers those systems to be "fallible", and the ECS scheduler will special-case the [`Err`]
//! variant of the returned `Result`.
//!
//! All [`Error`]s returned by a system are handled by an "error handler". By default, the
//! [`panic`] error handler function is used, resulting in a panic with the error message attached.
//!
//! You can change the default behavior by registering a custom error handler, either globally or
//! per `Schedule`:
//!
//! - [`App::set_systems_error_handler`] sets the global error handler for all systems of the
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
//! fn(Error, &ScheduleSystem)
//! ```
//!
//! The reference to [`ScheduleSystem`] allows you to access any non-mutating methods from
//! [`System`] – such as the system's [`name`] – in your error messages.
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
//! schedule.set_error_handler(|error, system| {
//!     if system.name().ends_with("update") {
//!         trace!("Nothing to see here, move along.");
//!         return;
//!     }
//!
//!     bevy_ecs::result::error(error, system);
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
//! [`App::set_systems_error_handler`]: ../../bevy_app/struct.App.html#method.set_systems_error_handler
//! [`system piping feature`]: crate::system::In

use crate::system::ScheduleSystem;
use alloc::boxed::Box;

/// A dynamic error type for use in fallible systems.
pub type Error = Box<dyn core::error::Error + Send + Sync + 'static>;

/// A result type for use in fallible systems.
pub type Result<T = (), E = Error> = core::result::Result<T, E>;

macro_rules! inner {
    ($call:path, $e:ident, $s:ident) => {
        $call!("Encountered an error in system `{}`: {:?}", $s.name(), $e);
    };
}

/// Error handler that panics with the system error.
#[track_caller]
#[inline]
pub fn panic(error: Error, system: &ScheduleSystem) {
    inner!(panic, error, system);
}

/// Error handler that logs the system error at the `error` level.
#[track_caller]
#[inline]
pub fn error(error: Error, system: &ScheduleSystem) {
    inner!(log::error, error, system);
}

/// Error handler that logs the system error at the `warn` level.
#[track_caller]
#[inline]
pub fn warn(error: Error, system: &ScheduleSystem) {
    inner!(log::warn, error, system);
}

/// Error handler that logs the system error at the `info` level.
#[track_caller]
#[inline]
pub fn info(error: Error, system: &ScheduleSystem) {
    inner!(log::info, error, system);
}

/// Error handler that logs the system error at the `debug` level.
#[track_caller]
#[inline]
pub fn debug(error: Error, system: &ScheduleSystem) {
    inner!(log::debug, error, system);
}

/// Error handler that logs the system error at the `trace` level.
#[track_caller]
#[inline]
pub fn trace(error: Error, system: &ScheduleSystem) {
    inner!(log::trace, error, system);
}

/// Error handler that ignores the system error.
#[track_caller]
#[inline]
pub fn ignore(_: Error, _: &ScheduleSystem) {}

#[track_caller]
#[inline]
pub(crate) fn default_error_handler(error: Error, system: &ScheduleSystem) {
    panic(error, system);
}
