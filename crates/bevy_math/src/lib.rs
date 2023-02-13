//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#![warn(missing_docs)]

mod bezier;
mod ray;
mod rect;

pub use bezier::{easing::CubicBezierEasing, CubicBezier2d, CubicBezier3d};
pub use ray::Ray;
pub use rect::Rect;

/// The `bevy_math` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        BVec2, BVec3, BVec4, CubicBezier2d, CubicBezier3d, CubicBezierEasing, EulerRot, IVec2,
        IVec3, IVec4, Mat2, Mat3, Mat4, Quat, Ray, Rect, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4,
    };
}

pub use glam::*;
