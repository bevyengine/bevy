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
