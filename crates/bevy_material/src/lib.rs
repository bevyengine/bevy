//! Provides a material abstraction for bevy

pub mod alpha;
pub mod bind_group_layout_entries;
pub mod labels;

/// The material prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::alpha::AlphaMode;
}
