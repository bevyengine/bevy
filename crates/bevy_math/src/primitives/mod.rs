//! This module defines primitive shapes.
//! The origin of all primitives is at 0,0(,0) unless stated otherwise

mod dim2;
pub use dim2::*;
mod dim3;
pub use dim3::*;

/// A trait marker trait for 2D primitives
pub trait Primitive2d {}

/// A trait marker trait for 3D primitives
pub trait Primitive3d {}
