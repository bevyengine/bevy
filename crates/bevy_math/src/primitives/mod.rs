//! This module defines primitive shapes.
//! The origin is (0, 0) for 2D primitives and (0, 0, 0) for 3D primitives,
//! unless stated otherwise.

mod dim2;
pub use dim2::*;
mod dim3;
pub use dim3::*;

/// A marker trait for 2D primitives
pub trait Primitive2d {}

/// A marker trait for 3D primitives
pub trait Primitive3d {}

/// An error indicating that a direction is invalid.
#[derive(Debug, PartialEq)]
pub enum InvalidDirectionError {
    /// The length of the direction vector is zero or very close to zero.
    Zero,
    /// The length of the direction vector is `std::f32::INFINITY`.
    Infinite,
    /// The length of the direction vector is `NaN`.
    NaN,
}

impl std::fmt::Display for InvalidDirectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Direction can not be zero (or very close to zero), or non-finite."
        )
    }
}

/// The winding order for a set of points
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindingOrder {
    /// A clockwise winding order
    Clockwise,
    /// A counterclockwise winding order
    CounterClockwise,
    /// An invalid winding order indicating that it could not be computed reliably.
    /// This often happens in *degenerate cases* where the points lie on the same line
    #[doc(alias = "Degenerate")]
    Invalid,
}
