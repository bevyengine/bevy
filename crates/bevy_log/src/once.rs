/// Call `trace!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! trace_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            trace!("{}", format!($($arg)+));
        }
    });
}

/// Call `debug!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! debug_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            debug!("{}", format!($($arg)+));
        }
    });
}

/// Call `info!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! info_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            info!("{}", format!($($arg)+));
        }
    });
}

/// Call `warn!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! warn_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            warn!("{}", format!($($arg)+));
        }
    });
}

/// Call `error!(...)` once per call site.
///
/// Useful for logging within systems which are called every frame.
#[macro_export]
macro_rules! error_once {
    ($($arg:tt)+) => ({
        use ::std::sync::atomic::{AtomicBool, Ordering};

        static FIRST_TIME: AtomicBool = AtomicBool::new(true);
        if FIRST_TIME.swap(false, Ordering::Relaxed) {
            error!("{}", format!($($arg)+));
        }
    });
}
