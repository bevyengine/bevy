#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(doc_cfg, rustdoc_internals))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]

//! This crate is about everything concerning extract.

extern crate alloc;

pub mod extract_base_component;
pub mod extract_instances;
mod extract_param;
pub mod extract_plugin;
pub mod extract_base_resource;
pub mod sync_component;
pub mod sync_world;

pub use extract_param::Extract;
pub use extract_plugin::*;
pub use extract_plugin::{ExtractSchedule, MainWorld};
pub use sync_world::*;

// Required to make proc macros work in bevy itself.
extern crate self as bevy_extract;

/// The extract prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{ExtractPlugin, ExtractSchedule};
}

