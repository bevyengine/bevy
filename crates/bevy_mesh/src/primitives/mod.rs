//! Mesh generation for [primitive shapes](bevy_math::primitives).
//!
//! Primitives that support meshing implement the [`Meshable`] trait.
//! Calling [`mesh`](Meshable::mesh) will return either a [`Mesh`] or a builder
//! that can be used to specify shape-specific configuration for creating the [`Mesh`].
//!
//! ```
//! # use bevy_asset::Assets;
//! # use bevy_ecs::prelude::ResMut;
//! # use bevy_math::prelude::Circle;
//! # use bevy_mesh::*;
//! #
//! # fn setup(mut meshes: ResMut<Assets<Mesh>>) {
//! // Create circle mesh with default configuration
//! let circle = meshes.add(Circle { radius: 25.0 });
//!
//! // Specify number of vertices
//! let circle = meshes.add(Circle { radius: 25.0 }.mesh().resolution(64));
//! # }
//! ```

mod dim2;
pub use dim2::*;

mod dim3;
pub use dim3::*;

mod extrusion;
pub use extrusion::*;

use super::Mesh;

/// A trait for shapes that can be turned into a [`Mesh`] via a [`MeshBuilder`].
pub trait Meshable {
    /// The output of [`Self::mesh_builder`].
    type Builder: MeshBuilder;

    /// Creates a [`MeshBuilder`] for a shape.
    fn mesh_builder(&self) -> Self::Builder;
}

/// A trait used to build [`Mesh`]es.
pub trait MeshBuilder {
    /// Builds a [`Mesh`] based on `&self`.
    fn mesh(&self) -> Mesh;
}

impl<M: Meshable<Builder = B>, B: MeshBuilder> MeshBuilder for M {
    fn mesh(&self) -> Mesh {
        self.mesh_builder().mesh()
    }
}

impl<B: MeshBuilder> From<B> for Mesh {
    fn from(builder: B) -> Self {
        builder.mesh()
    }
}
