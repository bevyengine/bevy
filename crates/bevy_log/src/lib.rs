#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate provides logging functions and configuration for [Bevy](https://bevyengine.org)
//! apps, and automatically configures platform specific log handlers (i.e. Wasm or Android).
//!
//! The macros provided for logging are reexported from [`tracing`](https://docs.rs/tracing),
//! and behave identically to it.
//!
//! By default, the [`LogPlugin`] from this crate is included in Bevy's `DefaultPlugins`
//! and the logging macros can be used out of the box, if used.
//!
//! For more fine-tuned control over logging behavior, set up the [`LogPlugin`] or
//! `DefaultPlugins` during app initialization.
#![cfg_attr(
    not(feature = "tracing"),
    doc = "\n\n[`LogPlugin`]: https://docs.rs/bevy_log"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

#[cfg(all(target_os = "android", feature = "std"))]
mod android_tracing;
mod once;
#[cfg(feature = "tracing")]
mod plugin;

#[cfg(feature = "trace_tracy_memory")]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

/// The log prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        debug, debug_once, error, error_once, info, info_once, trace, trace_once, warn, warn_once,
    };

    #[doc(hidden)]
    pub use bevy_utils::once;

    #[cfg(feature = "tracing")]
    #[doc(hidden)]
    pub use crate::{debug_span, error_span, info_span, trace_span, warn_span};
}

pub use bevy_utils::once;
pub use log::{debug, error, info, trace, warn};

#[cfg(feature = "tracing")]
pub use {
    crate::plugin::{BoxedLayer, LogPlugin, DEFAULT_FILTER},
    tracing::{self, debug_span, error_span, info_span, trace_span, warn_span, Level},
    tracing_subscriber,
};
