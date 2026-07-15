//! Mesh generation traits.
//!
//! Anything that support meshing implement the [`Meshable`] trait.
//! Calling [`mesh`](Meshable::mesh) will return a [`Mesh`] while calling
//! [`mesh_builder`](Meshable::mesh_builder) returns a builder that can be used to specify
//! shape-specific configuration for creating the [`Mesh`].
//!
//! ```
//! # use bevy_asset::Assets;
//! # use bevy_ecs::prelude::ResMut;
//! # use bevy_shapes::prelude::Circle;
//! # use bevy_mesh::*;
//! #
//! # fn setup(mut meshes: ResMut<Assets<Mesh>>) {
//! // Create circle mesh with default configuration
//! let circle = meshes.add(Circle { radius: 25.0 });
//!
//! // Specify number of vertices
//! let circle = meshes.add(Circle { radius: 25.0 }.mesh_builder().resolution(64));
//! # }
//! ```

use super::Mesh;

/// A trait for shapes that can be turned into a [`Mesh`].
pub trait Meshable {
    /// The output of [`Self::mesh`]. This will be a [`MeshBuilder`] used for creating a [`Mesh`].
    type Output: MeshBuilder;

    /// Creates a [`MeshBuilder`] for a shape.
    fn mesh_builder(&self) -> Self::Output;

    /// Creates a [`Mesh`] for a shape.
    fn mesh(&self) -> Mesh {
        self.mesh_builder().build()
    }
}

/// A trait used to build [`Mesh`]es from a configuration
pub trait MeshBuilder {
    /// Builds a [`Mesh`] based on the configuration in `self`.
    fn build(&self) -> Mesh;
}

impl<T: MeshBuilder> From<T> for Mesh {
    fn from(builder: T) -> Self {
        builder.build()
    }
}
