//! Mesh generation for [primitive shapes](bevy_math::primitives).
//!
//! Primitives that support meshing implement the [`Meshable`] trait.
//! Calling [`mesh`](Meshable::mesh) will return a builder that can be used
//! to specify shape-specific configuration.
//!
//! ```
//! # use bevy_asset::Assets;
//! # use bevy_ecs::prelude::ResMut;
//! # use bevy_math::primitives;
//! # use bevy_render::prelude::*;
//! #
//! # fn setup(mut meshes: ResMut<Assets<Mesh>>) {
//! // Create circle mesh with default configuration
//! let circle = meshes.add(primitives::Circle { radius: 25.0 });
//!
//! // Specify number of vertices
//! let circle = meshes.add(primitives::Circle { radius: 25.0 }.mesh().resolution(64));
//! # }
//! ```
//!
//! Some shapes also support different facing directions through the [`Facing`] enum or builder methods.
//!
//! ```
//! # use bevy_asset::Assets;
//! # use bevy_ecs::prelude::ResMut;
//! # use bevy_math::primitives;
//! # use bevy_render::prelude::*;
//! #
//! # fn setup(mut meshes: ResMut<Assets<Mesh>>) {
//! // Create rectangle mesh facing up
//! let rectangle = meshes.add(primitives::Rectangle::new(50.0, 25.0).mesh().facing_y());
//! # }
//! ```

#![warn(missing_docs)]

mod circle;
mod ellipse;
mod rectangle;
mod regular_polygon;
mod triangle;

pub use circle::CircleMeshBuilder;
pub use ellipse::EllipseMeshBuilder;
pub use rectangle::RectangleMeshBuilder;
pub use regular_polygon::RegularPolygonMeshBuilder;
pub use triangle::Triangle2dMeshBuilder;

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

    /// Returns the direction as an array in the format `[x, y, z]`.
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
