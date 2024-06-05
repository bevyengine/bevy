//! Mesh generation for [primitive shapes](bevy_math::primitives).
//!
//! Primitives that support meshing implement the [`Meshable`] trait.
//! Calling [`mesh`](Meshable::mesh) will return either a [`Mesh`](super::Mesh) or a builder
//! that can be used to specify shape-specific configuration for creating the [`Mesh`](super::Mesh).
//!
//! ```
//! # use bevy_asset::Assets;
//! # use bevy_ecs::prelude::ResMut;
//! # use bevy_math::prelude::Circle;
//! # use bevy_render::prelude::*;
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

/// A trait for shapes that can be turned into a [`Mesh`](super::Mesh).
pub trait Meshable {
    /// The output of [`Self::mesh`]. This will be a [`MeshBuilder`] used for creating a [`Mesh`](super::Mesh).
    type Output: MeshBuilder;

    /// Creates a [`Mesh`](super::Mesh) for a shape.
    fn mesh(&self) -> Self::Output;
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
