#![forbid(unsafe_code)]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(rustdoc_internals))]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod affine3;
mod aspect_ratio;
pub mod bounding;
pub mod common_traits;
mod compass;
pub mod cubic_splines;
mod direction;
mod float_ord;
mod isometry;
pub mod ops;
pub mod primitives;
mod ray;
mod rects;
mod rotation2d;

#[cfg(feature = "curve")]
pub mod curve;

#[cfg(feature = "rand")]
pub mod sampling;

pub use affine3::*;
pub use aspect_ratio::AspectRatio;
pub use common_traits::*;
pub use compass::{CompassOctant, CompassQuadrant};
pub use direction::*;
pub use float_ord::*;
pub use isometry::{Isometry2d, Isometry3d};
pub use ops::FloatPow;
pub use ray::{Ray2d, Ray3d};
pub use rects::*;
pub use rotation2d::Rot2;

#[cfg(feature = "curve")]
pub use curve::Curve;

#[cfg(feature = "rand")]
pub use sampling::{FromRng, ShapeSample};

/// The math prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        bvec2, bvec3, bvec3a, bvec4, bvec4a,
        cubic_splines::{CubicNurbsError, CubicSegment, RationalSegment},
        direction::{Dir2, Dir3, Dir3A},
        ivec2, ivec3, ivec4, mat2, mat3, mat3a, mat4, ops,
        primitives::*,
        quat, uvec2, uvec3, uvec4, vec2, vec3, vec3a, vec4, BVec2, BVec3, BVec3A, BVec4, BVec4A,
        EulerRot, FloatExt, IRect, IVec2, IVec3, IVec4, Isometry2d, Isometry3d, Mat2, Mat3, Mat3A,
        Mat4, Quat, Ray2d, Ray3d, Rect, Rot2, StableInterpolate, URect, UVec2, UVec3, UVec4, Vec2,
        Vec2Swizzles, Vec3, Vec3A, Vec3Swizzles, Vec4, Vec4Swizzles,
    };

    #[doc(hidden)]
    #[cfg(feature = "curve")]
    pub use crate::curve::*;

    #[doc(hidden)]
    #[cfg(feature = "rand")]
    pub use crate::sampling::{FromRng, ShapeSample};

    #[cfg(feature = "alloc")]
    #[doc(hidden)]
    pub use crate::cubic_splines::{
        CubicBSpline, CubicBezier, CubicCardinalSpline, CubicCurve, CubicGenerator, CubicHermite,
        CubicNurbs, CyclicCubicGenerator, RationalCurve, RationalGenerator,
    };
}

pub use glam::*;
