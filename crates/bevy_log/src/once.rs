/// Call `trace!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! trace_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            trace!($($arg)+);
            true
        } else {
            false
        }
    });
}

/// Call `debug!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! debug_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            debug!($($arg)+);
            true
        } else {
            false
        }
    });
}

/// Call `info!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! info_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            info!($($arg)+);
            true
        } else {
            false
        }
    });
}

/// Call `warn!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! warn_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            warn!($($arg)+);
            true
        } else {
            false
        }
    });
}

/// Call `error!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
///
/// Returns true the first time this is called
#[macro_export]
macro_rules! error_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            info!($($arg)+);
            true
        } else {
            false
        }
    });
}
