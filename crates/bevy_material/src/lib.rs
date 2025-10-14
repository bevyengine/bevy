//! Provides a material abstraction for bevy
#![allow(missing_docs)]

extern crate alloc;

pub mod alpha;
pub mod material;
pub mod opaque;
pub mod render;
pub mod render_phase;
pub mod render_resource;

/// The material prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::alpha::AlphaMode;
}
