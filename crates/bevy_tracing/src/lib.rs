#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![allow(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Tracing utilities for first-party [Bevy] engine crates.
//!
//! [Bevy]: https://bevyengine.org/
//!

#[allow(missing_docs)]
pub mod prelude {
    pub use crate::tracing;
}

pub use tracing;

use std::fmt::Debug;

/// Calls the [`tracing::info!`] macro on a value.
pub fn info<T: Debug>(data: T) {
    tracing::info!("{:?}", data);
}

/// Calls the [`tracing::debug!`] macro on a value.
pub fn dbg<T: Debug>(data: T) {
    tracing::debug!("{:?}", data);
}

/// Processes a [`Result`] by calling the [`tracing::warn!`] macro in case of an [`Err`] value.
pub fn warn<E: Debug>(result: Result<(), E>) {
    if let Err(warn) = result {
        tracing::warn!("{:?}", warn);
    }
}

/// Processes a [`Result`] by calling the [`tracing::error!`] macro in case of an [`Err`] value.
pub fn error<E: Debug>(result: Result<(), E>) {
    if let Err(error) = result {
        tracing::error!("{:?}", error);
    }
}

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

/// Like [`tracing::trace`], but conditional on cargo feature `detailed_trace`.
#[macro_export]
macro_rules! detailed_trace {
    ($($tts:tt)*) => {
        if cfg!(detailed_trace) {
            $crate::tracing::trace!($($tts)*);
        }
    }
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
