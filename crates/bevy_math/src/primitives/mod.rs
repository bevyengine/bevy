//! This module defines primitive shapes.
//! The origin is (0, 0) for 2D primitives and (0, 0, 0) for 3D primitives,
//! unless stated otherwise.

mod dim2;
pub use dim2::*;
mod dim3;
pub use dim3::*;
use glam::{Vec2, Vec3};
mod polygon;
#[cfg(feature = "serialize")]
mod serde;

/// A marker trait for 2D primitives
pub trait Primitive2d {}

/// A marker trait for 3D primitives
pub trait Primitive3d {}

/// The winding order for a set of points
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(alias = "Orientation")]
pub enum WindingOrder {
    /// A clockwise winding order
    Clockwise,
    /// A counterclockwise winding order
    #[doc(alias = "AntiClockwise")]
    CounterClockwise,
    /// An invalid winding order indicating that it could not be computed reliably.
    /// This often happens in *degenerate cases* where the points lie on the same line
    #[doc(alias("Degenerate", "Collinear"))]
    Invalid,
}

/// A trait for getting measurements of 2D shapes
pub trait Measured2d {
    /// Get the perimeter of the shape
    fn perimeter(&self) -> f32;

    /// Get the area of the shape
    fn area(&self) -> f32;
}

/// A trait for getting measurements of 3D shapes
pub trait Measured3d {
    /// Get the surface area of the shape
    fn area(&self) -> f32;

    /// Get the volume of the shape
    fn volume(&self) -> f32;
}

/// A trait for uniformly scaling shapes
pub trait ScaleUniform {
    /// Scale this primitive by the provided factor.
    fn scale_uniform(&self, scale: f32) -> Self;
}

/// A trait for scaling 2D shapes non-uniformly
pub trait ScaleNonUniform2d: ScaleUniform {
    /// The type of the scaled primitive.
    /// Most shapes will be the same when scaled but i.e. a [`Circle`] may become an [`Ellipse`] when scaled.
    type Output;

    /// Scale the primitive along the X- and Y-axis
    fn scale(&self, scale: Vec2) -> Self::Output;
}

/// A trait for scaling 3D shapes non-uniformly
pub trait ScaleNonUniform3d: ScaleUniform {
    /// The type of the scaled primitive.
    /// Most shapes will be the same when scaled but i.e. a [`Sphere`] may become an `Ellipsoid` when scaled.
    type Output;

    /// Scale the primitive along the X-, Y- and Z-axis
    fn scale(&self, scale: Vec3) -> Self::Output;
}
