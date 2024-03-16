/// Call some expression only once per call site.
#[macro_export]
macro_rules! once {
    ($expression:expr) => {{
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static SHOULD_FIRE: AtomicBool = AtomicBool::new(true);
        if SHOULD_FIRE.swap(false, Ordering::Relaxed) {
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
