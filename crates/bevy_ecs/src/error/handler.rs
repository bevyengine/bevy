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

type BevyErrorHandler = fn(BevyError, ErrorContext);

#[cfg(feature = "configurable_error_handler")]
mod inner {
    use super::*;
    use core::sync::atomic::{AtomicPtr, Ordering};

    // TODO: If we're willing to stomach the perf cost we could do a `RwLock<Box<dyn Fn(..)>>`.
    static GLOBAL_ERROR_HANDLER: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

    /// Gets the global error handler.
    ///
    /// If not set by [`set_error_handler`] defaults to [`panic()`].
    pub fn get_error_handler() -> BevyErrorHandler {
        // We need the acquire ordering since a sufficiently malicious user might call `set_error_handler`
        // and immediately call `get_error_handler` so we need to make sure these loads and stores are synchronized
        // with each other.
        let handler = GLOBAL_ERROR_HANDLER.load(Ordering::Acquire);

        if handler.is_null() {
            panic
        } else {
            // SAFETY: We just checked if this is null and the only way to set this value is using `set_error_handler` which
            //         makes sure this is actually a `BevyErrorHandler`.
            unsafe { core::mem::transmute::<*mut (), BevyErrorHandler>(handler) }
        }
    }

    /// Sets the error handler.
    ///
    /// This function is only available with the `configurable_error_handler` method enabled.
    pub fn set_error_handler(hook: BevyErrorHandler) {
        // Casting function pointers to normal pointers and back is called out as non-portable
        // by the `mem::transmute` documentation. The problem is that on some architectures
        // the size of a function pointers might be different from the size of a normal pointer.
        //
        // As of 2025-04-20 we're aware of 2 such architectures that also have a official rust target:
        // - WebAssembly: This architecture explicitly allows casting functions to ints and back.
        // - AVR:         The only official target with this architecture (avr-none) has a pointer width of 16.
        //                Which we don't support.
        //
        // Additionally the rust `alloc` library uses the same trick for its allocation error hook and we require
        // `alloc` support in this crate.
        //
        // AtomicFnPtr when?

        // See `get_error_handler` for why we need `Ordering::Release`.
        GLOBAL_ERROR_HANDLER.store(hook as *mut (), Ordering::Release);
    }
}

#[cfg(not(feature = "configurable_error_handler"))]
mod inner {
    /// Gets the global error handler. This is currently [`panic()`].
    pub fn get_error_handler() -> super::BevyErrorHandler {
        super::panic
    }
}

pub use inner::*;

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

#[cfg(test)]
mod tests {
    #![allow(
        clippy::allow_attributes,
        reason = "We can't use except because the allow attribute becomes redundant in some cases."
    )]

    #[allow(
        unused,
        reason = "With the correct combination of features we might end up not compiling any tests."
    )]
    use super::*;

    #[test]
    // This test only makes sense under miri
    #[cfg(miri)]
    fn default_handler() {
        // Check under miri that we aren't casting a null into a function pointer in the default case

        // Don't trigger dead code elimination
        core::hint::black_box(get_error_handler());
    }

    #[test]
    #[cfg(feature = "configurable_error_handler")]
    fn set_handler() {
        // We need to cast the function into a pointer ahead of time. The function pointers were randomly different otherwise.
        let new_handler = dont_handler_error as fn(_, _);

        set_error_handler(new_handler);
        let handler = get_error_handler();

        assert_eq!(handler as *const (), new_handler as *const ());
    }

    #[cfg(feature = "configurable_error_handler")]
    fn dont_handler_error(_: BevyError, _: ErrorContext) {}
}
