//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#![warn(missing_docs)]

mod rect;

pub use rect::Rect;

/// The `bevy_math` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        ivec2, ivec3, ivec4, mat2, mat3, mat4, quat, uvec2, uvec3, uvec4, vec2, vec3, vec4, BVec2,
        BVec3, BVec4, EulerRot, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, Quat, Rect, UVec2, UVec3,
        UVec4, Vec2, Vec3, Vec4,
    };
}

pub use glam::*;
