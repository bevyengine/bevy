#[cfg(feature = "configurable_error_handler")]
use bevy_platform::sync::OnceLock;
use core::fmt::Display;

use crate::{component::Tick, error::BevyError};
use alloc::borrow::Cow;

/// Context for a [`BevyError`] to aid in debugging.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ErrorContext {
    /// The error occurred in a system.
    System {
        /// The name of the system that failed.
        name: Cow<'static, str>,
        /// The last tick that the system was run.
        last_run: Tick,
    },
    /// The error occurred in a run condition.
    RunCondition {
        /// The name of the run condition that failed.
        name: Cow<'static, str>,
        /// The last tick that the run condition was evaluated.
        last_run: Tick,
    },
    /// The error occurred in a command.
    Command {
        /// The name of the command that failed.
        name: Cow<'static, str>,
    },
    /// The error occurred in an observer.
    Observer {
        /// The name of the observer that failed.
        name: Cow<'static, str>,
        /// The last tick that the observer was run.
        last_run: Tick,
    },
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::System { name, .. } => {
                write!(f, "System `{}` failed", name)
            }
            Self::Command { name } => write!(f, "Command `{}` failed", name),
            Self::Observer { name, .. } => {
                write!(f, "Observer `{}` failed", name)
            }
            Self::RunCondition { name, .. } => {
                write!(f, "Run condition `{}` failed", name)
            }
        }
    }
}

impl ErrorContext {
    /// The name of the ECS construct that failed.
    pub fn name(&self) -> &str {
        match self {
            Self::System { name, .. }
            | Self::Command { name, .. }
            | Self::Observer { name, .. }
            | Self::RunCondition { name, .. } => name,
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

/// A global error handler. This can be set at startup, as long as it is set before
/// any uses. This should generally be configured _before_ initializing the app.
///
/// This should be set inside of your `main` function, before initializing the Bevy app.
/// The value of this error handler can be accessed using the [`default_error_handler`] function,
/// which calls [`OnceLock::get_or_init`] to get the value.
///
/// **Note:** this is only available when the `configurable_error_handler` feature of `bevy_ecs` (or `bevy`) is enabled!
///
/// # Example
///
/// ```
/// # use bevy_ecs::error::{GLOBAL_ERROR_HANDLER, warn};
/// GLOBAL_ERROR_HANDLER.set(warn).expect("The error handler can only be set once, globally.");
/// // initialize Bevy App here
/// ```
///
/// To use this error handler in your app for custom error handling logic:
///
/// ```rust
/// use bevy_ecs::error::{default_error_handler, GLOBAL_ERROR_HANDLER, BevyError, ErrorContext, panic};
///
/// fn handle_errors(error: BevyError, ctx: ErrorContext) {
///    let error_handler = default_error_handler();
///    error_handler(error, ctx);        
/// }
/// ```
///
/// # Warning
///
/// As this can *never* be overwritten, library code should never set this value.
#[cfg(feature = "configurable_error_handler")]
pub static GLOBAL_ERROR_HANDLER: OnceLock<fn(BevyError, ErrorContext)> = OnceLock::new();

/// The default error handler. This defaults to [`panic()`],
/// but if set, the [`GLOBAL_ERROR_HANDLER`] will be used instead, enabling error handler customization.
/// The `configurable_error_handler` feature must be enabled to change this from the panicking default behavior,
/// as there may be runtime overhead.
#[inline]
pub fn default_error_handler() -> fn(BevyError, ErrorContext) {
    #[cfg(not(feature = "configurable_error_handler"))]
    return panic;

    #[cfg(feature = "configurable_error_handler")]
    return *GLOBAL_ERROR_HANDLER.get_or_init(|| panic);
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
