//! This module contains tools related to random sampling.
//!
//! To use this, the "rand" feature must be enabled.

#[cfg(feature = "meshing")]
pub mod mesh_sampling;
pub mod shape_sampling;

#[cfg(feature = "meshing")]
pub use mesh_sampling::*;
pub use shape_sampling::*;
