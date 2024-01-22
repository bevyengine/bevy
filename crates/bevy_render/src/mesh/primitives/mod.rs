//! Mesh generation for [primitive shapes](bevy_math::primitives).
//!
//! Primitives that support meshing implement the [`Meshable`] trait.
//! Calling [`mesh`](Meshable::mesh) will return either a [`Mesh`] or a builder
//! that can be used to specify shape-specific configuration for creating the [`Mesh`].
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
//! let circle = meshes.add(primitivesCircle { radius: 25.0 }.mesh().resolution(64));
//! # }
//! ```
//!
//! [`Mesh`]: super::Mesh

#![warn(missing_docs)]

mod dim2;
pub use dim2::{CircleMeshBuilder, EllipseMeshBuilder};

/// A trait for shapes that can be turned into a [`Mesh`].
pub trait Meshable {
    /// The output of [`Self::mesh`]. This can either be a [`Mesh`]
    /// or a builder used for creating a [`Mesh`].
    type Output;

    /// Creates a [`Mesh`] for a shape.
    fn mesh(&self) -> Self::Output;
}
