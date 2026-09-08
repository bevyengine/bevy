//! This module contains tools related to random sampling `bevy_shape`s.
//!
//! To use this, the "rand" feature must be enabled.

#[cfg(feature = "alloc")]
pub mod mesh_sampling;
#[cfg(feature = "alloc")]
pub use mesh_sampling::*;
pub mod shape_sampling;
pub use shape_sampling::*;
