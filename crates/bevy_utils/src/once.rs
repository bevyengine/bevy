/// Call some expression only once per call site.
#[macro_export]
macro_rules! once {
    ($expression:expr) => {{
        use ::core::sync::atomic::{AtomicBool, Ordering};

        static SHOULD_FIRE: AtomicBool = AtomicBool::new(true);
        if SHOULD_FIRE.swap(false, Ordering::Relaxed) {
            $expression;
        }
    }};
}

/// Call [`trace!`](crate::log::trace) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! trace_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::log::trace!($($arg)+))
    });
}

/// Call [`debug!`](crate::log::debug) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! debug_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::log::debug!($($arg)+))
    });
}

/// Call [`info!`](crate::log::info) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! info_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::log::info!($($arg)+))
    });
}

/// Call [`warn!`](crate::log::warn) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! warn_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::log::warn!($($arg)+))
    });
}

/// Call [`error!`](crate::log::error) once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! error_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::log::error!($($arg)+))
    });
}
