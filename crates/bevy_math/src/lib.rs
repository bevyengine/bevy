//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#![warn(missing_docs)]

mod affine3;
mod aspect_ratio;
pub mod cubic_splines;
pub mod primitives;
mod ray;
mod rects;

pub use affine3::*;
pub use aspect_ratio::AspectRatio;
pub use ray::{Ray2d, Ray3d};
pub use rects::*;

/// The `bevy_math` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        cubic_splines::{
            CubicBSpline, CubicBezier, CubicCardinalSpline, CubicGenerator, CubicHermite,
            CubicSegment,
        },
        primitives, BVec2, BVec3, BVec4, EulerRot, IRect, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4,
        Quat, Ray2d, Ray3d, Rect, URect, UVec2, UVec3, UVec4, Vec2, Vec2Swizzles, Vec3,
        Vec3Swizzles, Vec4, Vec4Swizzles,
    };
}

pub use glam::*;
