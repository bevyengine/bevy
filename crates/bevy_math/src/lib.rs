//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#![allow(clippy::type_complexity)]
#![warn(missing_docs)]

mod affine3;
pub mod cubic_splines;
mod ray;
mod rects;

pub use affine3::*;
pub use ray::Ray;
pub use rects::*;

/// The `bevy_math` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        cubic_splines::{
            BSpline, CardinalSpline, CubicBezier, CubicGenerator, CubicSegment, Hermite,
        },
        BVec2, BVec3, BVec4, EulerRot, IRect, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, Quat, Ray,
        Rect, URect, UVec2, UVec3, UVec4, Vec2, Vec2Swizzles, Vec3, Vec3Swizzles, Vec4,
        Vec4Swizzles,
    };
}

pub use glam::*;
