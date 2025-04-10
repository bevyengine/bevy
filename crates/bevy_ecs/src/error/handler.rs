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

mod global_error_handler {
    use super::{BevyError, ErrorContext};
    use bevy_platform_support::sync::atomic::{
        AtomicPtr,
        Ordering::{AcqRel, Acquire, Relaxed},
    };

    // SAFETY: Only accessible from within this module.
    static HANDLER_ADDRESS: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

    /// Set the global error handler.
    ///
    /// If used, this should be called [before] any uses of [`default_error_handler`],
    /// generally inside your `main` function before initializing the app.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::error::{set_global_default_error_handler, warn};
    /// set_global_default_error_handler(warn);
    /// // initialize Bevy App here
    /// ```
    ///
    /// To use this error handler in your app for custom error handling logic:
    ///
    /// ```rust
    /// use bevy_ecs::error::{default_error_handler, BevyError, ErrorContext, panic};
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
    ///
    /// [before]: https://doc.rust-lang.org/nightly/core/sync/atomic/index.html#memory-model-for-atomic-accesses
    /// [`default_error_handler`]: super::default_error_handler
    pub fn set_global_default_error_handler(handler: fn(BevyError, ErrorContext)) {
        if HANDLER_ADDRESS
            .compare_exchange(core::ptr::null_mut(), handler as *mut (), AcqRel, Acquire)
            .is_err()
        {
            panic!("Global error handler set multiple times");
        }
    }

    #[inline]
    pub(super) fn get() -> Option<fn(BevyError, ErrorContext)> {
        // The error handler must have been already set from the perspective of this thread,
        // otherwise we will panic. It will never be updated after this point.
        // We therefore only need a relaxed load.
        let ptr = HANDLER_ADDRESS.load(Relaxed);
        // SAFETY: We only ever store null or a valid handler function.
        // `null` is guaranteed to be represent `None`:
        // https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html#representation
        unsafe { core::mem::transmute(ptr) }
    }
}

pub use global_error_handler::set_global_default_error_handler;

/// The default error handler. This defaults to [`panic()`],
/// but you can override this behavior via [`set_global_error_handler`].
#[inline]
pub fn default_error_handler() -> fn(BevyError, ErrorContext) {
    global_error_handler::get().unwrap_or(panic)
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
