#[cfg(feature = "portable-atomic")]
use portable_atomic::{AtomicBool, Ordering};

#[cfg(not(feature = "portable-atomic"))]
use core::sync::atomic::{AtomicBool, Ordering};

/// Wrapper around an [`AtomicBool`], abstracting the backing implementation and
/// ordering considerations.
#[doc(hidden)]
pub struct OnceFlag(AtomicBool);

impl OnceFlag {
    /// Create a new flag in the unset state.
    pub const fn new() -> Self {
        Self(AtomicBool::new(true))
    }

    /// Sets this flag. Will return `true` if this flag hasn't been set before.
    pub fn set(&self) -> bool {
        self.0.swap(false, Ordering::Relaxed)
    }
}

impl Default for OnceFlag {
    fn default() -> Self {
        Self::new()
    }
}

/// Call some expression only once per call site.
#[macro_export]
macro_rules! once {
    ($expression:expr) => {{
        static SHOULD_FIRE: $crate::OnceFlag = $crate::OnceFlag::new();
        if SHOULD_FIRE.set() {
            $expression;
        }
    }};
}

/// Call [`trace!`](crate::tracing::trace) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! trace_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::tracing::trace!($($arg)+))
    });
}

/// Call [`debug!`](crate::tracing::debug) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! debug_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::tracing::debug!($($arg)+))
    });
}

/// Call [`info!`](crate::tracing::info) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! info_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::tracing::info!($($arg)+))
    });
}

/// Call [`warn!`](crate::tracing::warn) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! warn_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::tracing::warn!($($arg)+))
    });
}

/// Call [`error!`](crate::tracing::error) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! error_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::tracing::error!($($arg)+))
    });
}
