use core::fmt::Display;

use crate::{component::Tick, error::BevyError, prelude::Resource};
use bevy_utils::prelude::DebugName;
use derive_more::derive::{Deref, DerefMut};

/// Context for a [`BevyError`] to aid in debugging.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ErrorContext {
    /// The error occurred in a system.
    System {
        /// The name of the system that failed.
        name: DebugName,
        /// The last tick that the system was run.
        last_run: Tick,
    },
    /// The error occurred in a run condition.
    RunCondition {
        /// The name of the run condition that failed.
        name: DebugName,
        /// The last tick that the run condition was evaluated.
        last_run: Tick,
        /// The system this run condition is attached to.
        system: DebugName,
        /// `true` if this run condition was on a set.
        on_set: bool,
    },
    /// The error occurred in a command.
    Command {
        /// The name of the command that failed.
        name: DebugName,
    },
    /// The error occurred in an observer.
    Observer {
        /// The name of the observer that failed.
        name: DebugName,
        /// The last tick that the observer was run.
        last_run: Tick,
    },
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::System { name, .. } => {
                write!(f, "System `{name}` failed")
            }
            Self::Command { name } => write!(f, "Command `{name}` failed"),
            Self::Observer { name, .. } => {
                write!(f, "Observer `{name}` failed")
            }
            Self::RunCondition {
                name,
                system,
                on_set,
                ..
            } => {
                write!(
                    f,
                    "Run condition `{name}` failed for{} system `{system}`",
                    if *on_set { " set containing" } else { "" }
                )
            }
        }
    }
}

impl ErrorContext {
    /// The name of the ECS construct that failed.
    pub fn name(&self) -> DebugName {
        match self {
            Self::System { name, .. }
            | Self::Command { name, .. }
            | Self::Observer { name, .. }
            | Self::RunCondition { name, .. } => name.clone(),
        }
    }

    /// A string representation of the kind of ECS construct that failed.
    ///
    /// This is a simpler helper used for logging.
    pub fn kind(&self) -> &str {
        match self {
            Self::System { .. } => "system",
            Self::Command { .. } => "command",
            Self::Observer { .. } => "observer",
            Self::RunCondition { .. } => "run condition",
        }
    }
}

macro_rules! inner {
    ($call:path, $e:ident, $c:ident) => {
        $call!(
            "Encountered an error in {} `{}`: {}",
            $c.kind(),
            $c.name(),
            $e
        );
    };
}

/// Defines how Bevy reacts to errors.
pub type ErrorHandler = fn(BevyError, ErrorContext);

/// Error handler to call when an error is not handled otherwise.
/// Defaults to [`panic()`].
///
/// When updated while a [`Schedule`] is running, it doesn't take effect for
/// that schedule until it's completed.
///
/// [`Schedule`]: crate::schedule::Schedule
#[derive(Resource, Deref, DerefMut, Copy, Clone)]
pub struct DefaultErrorHandler(pub ErrorHandler);

impl Default for DefaultErrorHandler {
    fn default() -> Self {
        Self(panic)
    }
}

/// Error handler that panics with the system error.
#[track_caller]
#[inline]
pub fn panic(error: BevyError, ctx: ErrorContext) {
    inner!(panic, error, ctx);
}

/// Error handler that logs the system error at the `error` level.
#[track_caller]
#[inline]
pub fn error(error: BevyError, ctx: ErrorContext) {
    inner!(log::error, error, ctx);
}

/// Error handler that logs the system error at the `warn` level.
#[track_caller]
#[inline]
pub fn warn(error: BevyError, ctx: ErrorContext) {
    inner!(log::warn, error, ctx);
}

/// Error handler that logs the system error at the `info` level.
#[track_caller]
#[inline]
pub fn info(error: BevyError, ctx: ErrorContext) {
    inner!(log::info, error, ctx);
}

/// Error handler that logs the system error at the `debug` level.
#[track_caller]
#[inline]
pub fn debug(error: BevyError, ctx: ErrorContext) {
    inner!(log::debug, error, ctx);
}

/// Error handler that logs the system error at the `trace` level.
#[track_caller]
#[inline]
pub fn trace(error: BevyError, ctx: ErrorContext) {
    inner!(log::trace, error, ctx);
}

/// Error handler that ignores the system error.
#[track_caller]
#[inline]
pub fn ignore(_: BevyError, _: ErrorContext) {}
