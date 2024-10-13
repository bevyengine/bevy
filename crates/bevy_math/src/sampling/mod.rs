//! This module contains tools related to random sampling.
//!
//! To use this, the "rand" feature must be enabled.

pub mod mesh_sampling;
pub mod shape_sampling;
pub mod standard;

pub use mesh_sampling::*;
pub use shape_sampling::*;
pub use standard::*;
