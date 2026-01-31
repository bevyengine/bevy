//! Shared physics types.
//!
//! This module contains semantic type aliases for physics properties.
//! Using these aliases makes the code more self-documenting by expressing
//! the physical meaning of values rather than just their storage type.

#![allow(non_camel_case_types)]

/// A scalar floating-point value.
pub type float = f32;

/// A 3D vector (direction or displacement).
pub type vector3f = bevy_math::Vec3;

/// A quaternion representing orientation.
pub type quatf = bevy_math::Quat;

/// A 3D point (position in space).
pub type point3f = bevy_math::Vec3;

/// An angle measured in radians.
pub type angle = f32;
