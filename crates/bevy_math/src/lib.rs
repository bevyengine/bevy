#![warn(missing_docs)]

//! Provides math types and functionalities for the bevy game engine.
//!
//! The commonly used types are vectors, rectangles and sizes.
//!
//! # Example
//!
//! ```rust
//! use bevy_math::prelude::*;
//! ```

mod face_toward;
mod geometry;

pub use face_toward::*;
pub use geometry::*;
pub use glam::*;

/// The bevy_math prelude.
///
/// # Example
///
/// ```rust
/// use bevy_math::prelude::*;
/// ```
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        BVec2, BVec3, BVec4, EulerRot, FaceToward, IVec2, IVec3, IVec4, Mat3, Mat4, Quat, Rect,
        Size, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4,
    };
}
