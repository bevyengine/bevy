//! Ray casting types and functionality.

mod dim2;
mod dim3;

pub use dim2::{PrimitiveRayCast2d, RayHit2d};
pub use dim3::{PrimitiveRayCast3d, RayHit3d};
