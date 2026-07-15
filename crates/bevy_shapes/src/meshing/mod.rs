//! [`Meshable`](bevy_mesh::Meshable) implementation for [primitive shapes](bevy_shapes::primitives).

mod dim2;
pub use dim2::*;

mod dim3;
pub use dim3::*;

#[cfg(feature = "extrusion")]
mod extrusion;
#[cfg(feature = "extrusion")]
pub use extrusion::*;
