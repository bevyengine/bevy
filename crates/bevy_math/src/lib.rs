//! Provides math types and functionality for the Bevy game engine.
//!
//! The commonly used types are vectors like [`Vec2`] and [`Vec3`],
//! matrices like [`Mat2`], [`Mat3`] and [`Mat4`] and orientation representations
//! like [`Quat`].

#![warn(missing_docs)]

mod ray;
mod rect;

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

/// Fast approximated reciprocal square root.
#[inline]
pub fn approx_rsqrt(x: f32) -> f32 {
    // Quake 3 fast inverse sqrt, has a higher error but still good
    // enough and faster than `.sqrt().recip()`, implementation
    // borrowed from Piston under the MIT License:
    // [https://github.com/PistonDevelopers/skeletal_animation]
    //
    // Includes a refinement seen in [http://rrrola.wz.cz/inv_sqrt.html]
    // that improves overall accuracy by 2.7x while maintaining the same
    // performance characteristics.
    let x2: f32 = x * 0.5;
    let mut y: f32 = x;

    let mut i: i32 = y.to_bits() as i32;
    i = 0x5f1ffff9 - (i >> 1);
    y = f32::from_bits(i as u32);

    y = 0.70395225 * y * (2.3892446 - (x2 * y * y));
    y
}
