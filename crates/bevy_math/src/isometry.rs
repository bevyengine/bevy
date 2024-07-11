//! Isometry types for expressing rigid motions in two and three dimensions.
//!
//! These are often used to express the relative positions of two entities (e.g. primitive shapes).
//! For example, in determining whether a sphere intersects a cube, one needs to know how the two are
//! positioned relative to one another in addition to their sizes.
//! If the two had absolute positions and orientations described by isometries `cube_iso` and `sphere_iso`,
//! then `cube_iso.inverse() * sphere_iso` would describe the relative orientation, which is sufficient for
//! answering this query.

use crate::{Affine2, Affine3, Affine3A, Mat3, Mat3A, Quat, Rot2, Vec2, Vec3, Vec3A};
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize};
use std::ops::Mul;

/// An isometry in two dimensions.
#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Isometry2d {
    /// The rotational part of a two-dimensional isometry.
    pub rotation: Rot2,
    /// The translational part of a two-dimensional isometry.
    pub translation: Vec2,
}

impl Isometry2d {
    /// The identity isometry which represents the rigid motion of not doing anything.
    pub const IDENTITY: Self = Isometry2d {
        rotation: Rot2::IDENTITY,
        translation: Vec2::ZERO,
    };

    /// Create a two-dimensional isometry from a rotation and a translation.
    #[inline]
    pub fn new(translation: Vec2, rotation: Rot2) -> Self {
        Isometry2d {
            rotation,
            translation,
        }
    }

    /// Create a two-dimensional isometry from a rotation.
    #[inline]
    pub fn from_rotation(rotation: Rot2) -> Self {
        Isometry2d {
            rotation,
            translation: Vec2::ZERO,
        }
    }

    /// Create a two-dimensional isometry from a translation.
    #[inline]
    pub fn from_translation(translation: Vec2) -> Self {
        Isometry2d {
            rotation: Rot2::IDENTITY,
            translation,
        }
    }

    /// The inverse isometry that undoes this one.
    #[inline]
    pub fn inverse(&self) -> Self {
        let inv_rot = self.rotation.inverse();
        Isometry2d {
            rotation: inv_rot,
            translation: inv_rot * -self.translation,
        }
    }

    /// Transform a point by rotating and translating it using this isometry.
    #[inline]
    pub fn transform_point(&self, point: Vec2) -> Vec2 {
        self.rotation * point + self.translation
    }
}

impl From<Isometry2d> for Affine2 {
    #[inline]
    fn from(iso: Isometry2d) -> Self {
        Affine2 {
            matrix2: iso.rotation.into(),
            translation: iso.translation,
        }
    }
}

impl Mul for Isometry2d {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Isometry2d {
            rotation: self.rotation * rhs.rotation,
            translation: self.rotation * rhs.translation + self.translation,
        }
    }
}

impl Mul<Vec2> for Isometry2d {
    type Output = Vec2;

    #[inline]
    fn mul(self, rhs: Vec2) -> Self::Output {
        self.transform_point(rhs)
    }
}

/// An isometry in three dimensions.
#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Isometry3d {
    /// The rotational part of a three-dimensional isometry.
    pub rotation: Quat,
    /// The translational part of a three-dimensional isometry.
    pub translation: Vec3A,
}

impl Isometry3d {
    /// The identity isometry which represents the rigid motion of not doing anything.
    pub const IDENTITY: Self = Isometry3d {
        rotation: Quat::IDENTITY,
        translation: Vec3A::ZERO,
    };

    /// Create a three-dimensional isometry from a rotation and a translation.
    #[inline]
    pub fn new(translation: impl Into<Vec3A>, rotation: Quat) -> Self {
        Isometry3d {
            rotation,
            translation: translation.into(),
        }
    }

    /// Create a three-dimensional isometry from a rotation.
    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        Isometry3d {
            rotation,
            translation: Vec3A::ZERO,
        }
    }

    /// Create a three-dimensional isometry from a translation.
    #[inline]
    pub fn from_translation(translation: impl Into<Vec3A>) -> Self {
        Isometry3d {
            rotation: Quat::IDENTITY,
            translation: translation.into(),
        }
    }

    /// The inverse isometry that undoes this one.
    #[inline]
    pub fn inverse(&self) -> Self {
        let inv_rot = self.rotation.inverse();
        Isometry3d {
            rotation: inv_rot,
            translation: inv_rot * -self.translation,
        }
    }

    /// Transform a point by rotating and translating it using this isometry.
    #[inline]
    pub fn transform_point(&self, point: impl Into<Vec3A>) -> Vec3A {
        self.rotation * point.into() + self.translation
    }
}

impl From<Isometry3d> for Affine3 {
    #[inline]
    fn from(iso: Isometry3d) -> Self {
        Affine3 {
            matrix3: Mat3::from_quat(iso.rotation),
            translation: iso.translation.into(),
        }
    }
}

impl From<Isometry3d> for Affine3A {
    #[inline]
    fn from(iso: Isometry3d) -> Self {
        Affine3A {
            matrix3: Mat3A::from_quat(iso.rotation),
            translation: iso.translation,
        }
    }
}

impl Mul for Isometry3d {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Isometry3d {
            rotation: self.rotation * rhs.rotation,
            translation: self.rotation * rhs.translation + self.translation,
        }
    }
}

impl Mul<Vec3A> for Isometry3d {
    type Output = Vec3A;

    #[inline]
    fn mul(self, rhs: Vec3A) -> Self::Output {
        self.transform_point(rhs)
    }
}

impl Mul<Vec3> for Isometry3d {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        self.transform_point(rhs).into()
    }
}
