//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod affine3;
mod aspect_ratio;
pub mod bounding;
pub mod cubic_splines;
mod direction;
pub mod primitives;
mod ray;
mod rects;
mod rotation2d;
#[cfg(feature = "rand")]
mod shape_sampling;

pub use affine3::*;
pub use aspect_ratio::AspectRatio;
pub use direction::*;
pub use ray::{Ray2d, Ray3d};
pub use rects::*;
pub use rotation2d::Rotation2d;
#[cfg(feature = "rand")]
pub use shape_sampling::ShapeSample;

/// The `bevy_math` prelude.
pub mod prelude {
    #[doc(hidden)]
    #[cfg(feature = "rand")]
    pub use crate::shape_sampling::ShapeSample;
    #[doc(hidden)]
    pub use crate::{
        cubic_splines::{
            CubicBSpline, CubicBezier, CubicCardinalSpline, CubicCurve, CubicGenerator,
            CubicHermite, CubicNurbs, CubicNurbsError, CubicSegment, RationalCurve,
            RationalGenerator, RationalSegment,
        },
        direction::{Dir2, Dir3, Dir3A},
        primitives::*,
        BVec2, BVec3, BVec4, EulerRot, FloatExt, IRect, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4,
        Quat, Ray2d, Ray3d, Rect, Rotation2d, URect, UVec2, UVec3, UVec4, Vec2, Vec2Swizzles, Vec3,
        Vec3Swizzles, Vec4, Vec4Swizzles,
    };
}

pub use glam::*;
