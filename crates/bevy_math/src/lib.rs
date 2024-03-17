//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod affine3;
mod aspect_ratio;
pub mod bounding;
pub mod componentwise_arith;
pub mod cubic_splines;
mod direction;
pub mod point;
pub mod primitives;
mod ray;
mod rects;
mod rotation2d;

pub use affine3::*;
pub use aspect_ratio::AspectRatio;
pub use direction::*;
pub use ray::{Ray2d, Ray3d};
pub use rects::*;
pub use rotation2d::Rotation2d;

/// The `bevy_math` prelude.
pub mod prelude {
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
