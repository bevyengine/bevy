//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#![warn(missing_docs)]

mod ray;
mod rect;

use std::ops::{Add, Div, Mul, Sub};

pub use ray::Ray;
pub use rect::Rect;

/// The `bevy_math` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        BVec2, BVec3, BVec4, EulerRot, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, Quat, Ray, Rect,
        UVec2, UVec3, UVec4, Vec2, Vec3, Vec4,
    };
}

pub use glam::*;

/// Maps a value from one range of values to a new range of values.
pub fn map_range<
    T: Add<Output = T> + Div<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy,
>(
    value: T,
    old_range: (T, T),
    new_range: (T, T),
) -> T {
    (value - old_range.0) / (old_range.1 - old_range.0) * (new_range.1 - new_range.0) + new_range.0
}
