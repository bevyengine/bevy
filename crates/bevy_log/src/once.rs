/// Call some expression only once
#[macro_export]
macro_rules! once {
    ($expression:expr) => {{
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static SHOULD_FIRE: AtomicBool = AtomicBool::new(true);
        if SHOULD_FIRE.swap(false, Ordering::Relaxed) {
            $expression;
            true
        } else {
            false
        }
    }};
}

/// Call [`trace!`] once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! trace_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::trace!($($arg)+))
    });
}

/// Call [`debug!`] once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! debug_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::debug!($($arg)+))
    });
}

/// Call [`info!`] once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! info_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::info!($($arg)+))
    });
}

/// Call [`warn!`] once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! warn_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::warn!($($arg)+))
    });
}

/// Call [`error!`] once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! error_once {
    ($($arg:tt)+) => ({
        $crate::once!($crate::error!($($arg)+))
    });
}
