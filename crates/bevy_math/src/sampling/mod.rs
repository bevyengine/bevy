//! This module contains tools related to random sampling.
//!
//! To use this, the "rand" feature must be enabled.

pub mod shape_sampling;
pub mod standard;

pub use shape_sampling::*;
pub use standard::*;
