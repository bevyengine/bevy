//! Mesh generation for [primitive shapes](bevy_math::primitives).

#![warn(missing_docs)]

mod capsule;
mod circle;
mod cone;
mod conical_frustum;
mod cuboid;
mod cylinder;
mod ellipse;
mod plane;
mod rectangle;
mod regular_polygon;
mod sphere;
mod torus;
mod triangle;

pub use capsule::CapsuleMesh;
pub use circle::CircleMesh;
pub use cone::ConeMesh;
pub use conical_frustum::ConicalFrustumMesh;
pub use cuboid::CuboidMesh;
pub use cylinder::CylinderMesh;
pub use ellipse::EllipseMesh;
pub use plane::PlaneMesh;
pub use rectangle::RectangleMesh;
pub use regular_polygon::RegularPolygonMesh;
pub use sphere::{SphereKind, SphereMesh};
pub use torus::TorusMesh;
pub use triangle::Triangle2dMesh;

use super::Mesh;

/// A trait for shapes that can be turned into a [`Mesh`].
pub trait Meshable {
    /// The output of [`Self::mesh`]. This can either be a [`Mesh`]
    /// or a builder used for creating a [`Mesh`].
    type Output;

    /// Creates a [`Mesh`] for a shape.
    fn mesh(&self) -> Self::Output;
}

/// The cartesian axis that a [`Mesh`] should be facing upon creation.
/// This is either positive or negative `X`, `Y`, or `Z`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Facing {
    /// Facing the `+X` direction.
    X = 1,
    /// Facing the `+Y` direction.
    Y = 2,
    /// Facing the `+Z` direction.
    #[default]
    Z = 3,
    /// Facing the `-X` direction.
    NegX = -1,
    /// Facing the `-Y` direction.
    NegY = -2,
    /// Facing the `-Z` direction.
    NegZ = -3,
}

impl Facing {
    /// Returns `1` if the facing direction is positive `X`, `Y`, or `Z`, and `-1` otherwise.
    #[inline]
    pub const fn signum(&self) -> i8 {
        match self {
            Facing::X | Facing::Y | Facing::Z => 1,
            _ => -1,
        }
    }

    /// Returns the direction in as an array in the format `[x, y, z]`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_render::prelude::Facing;
    /// assert_eq!(Facing::X.to_array(), [1.0, 0.0, 0.0]);
    /// ```
    #[inline]
    pub const fn to_array(&self) -> [f32; 3] {
        match self {
            Facing::X => [1.0, 0.0, 0.0],
            Facing::Y => [0.0, 1.0, 0.0],
            Facing::Z => [0.0, 0.0, 1.0],
            Facing::NegX => [-1.0, 0.0, 0.0],
            Facing::NegY => [0.0, -1.0, 0.0],
            Facing::NegZ => [0.0, 0.0, -1.0],
        }
    }
}

/// An extension trait for methods related to setting a specific [`Facing`] direction.
pub trait MeshFacingExtension: Sized {
    /// Set the [`Facing`] direction.
    fn facing(self, facing: Facing) -> Self;

    /// Set the [`Facing`] direction to `+X`.
    #[inline]
    fn facing_x(self) -> Self {
        self.facing(Facing::X)
    }

    /// Set the [`Facing`] direction to `+Y`.
    #[inline]
    fn facing_y(self) -> Self {
        self.facing(Facing::Y)
    }

    /// Set the [`Facing`] direction to `+Z`.
    #[inline]
    fn facing_z(self) -> Self {
        self.facing(Facing::Z)
    }

    /// Set the [`Facing`] direction to `-X`.
    #[inline]
    fn facing_neg_x(self) -> Self {
        self.facing(Facing::NegX)
    }

    /// Set the [`Facing`] direction to `-Y`.
    #[inline]
    fn facing_neg_y(self) -> Self {
        self.facing(Facing::NegY)
    }

    /// Set the [`Facing`] direction to `-Z`.
    #[inline]
    fn facing_neg_z(self) -> Self {
        self.facing(Facing::NegZ)
    }
}
