#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]
#![no_std]

//! Platform compatibility support for first-party [Bevy] engine crates.
//!
//! [Bevy]: https://bevyengine.org/

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod hash;
pub mod sync;
pub mod thread;
pub mod time;

#[cfg(feature = "alloc")]
pub mod collections;

/// Frequently used items which would typically be included in most contexts.
///
/// When adding `no_std` support to a crate for the first time, often there's a substantial refactor
/// required due to the change in implicit prelude from `std::prelude` to `core::prelude`.
/// This unfortunately leaves out many items from `alloc`, even if the crate unconditionally
/// includes that crate.
///
/// This prelude aims to ease the transition by re-exporting items from `alloc` which would
/// otherwise be included in the `std` implicit prelude.
pub mod prelude {
    #[cfg(feature = "alloc")]
    pub use alloc::{
        borrow::ToOwned, boxed::Box, format, string::String, string::ToString, vec, vec::Vec,
    };

    // Items from `std::prelude` that are missing in this module:
    // * dbg
    // * eprint
    // * eprintln
    // * is_x86_feature_detected
    // * print
    // * println
    // * thread_local
}
