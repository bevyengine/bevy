//! Isometry types for expressing rigid motions in two and three dimensions.

use std::ops::Mul;

use crate::{Affine2, Affine3, Affine3A, Mat3, Mat3A, Quat, Rot2, Vec2, Vec3, Vec3A};

/// An isometry in two dimensions.
#[derive(Copy, Clone, Debug, PartialEq)]
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

    /// Create a two-dimensional isometry from a translation and rotation.
    pub fn new(translation: Vec2, rotation: Rot2) -> Self {
        Isometry2d {
            rotation,
            translation,
        }
    }

    /// Create a two-dimensional isometry from a rotation.
    pub fn from_rotation(rotation: Rot2) -> Self {
        Isometry2d {
            rotation,
            translation: Vec2::ZERO,
        }
    }

    /// Create a two-dimensional isometry from a translation.
    pub fn from_translation(translation: Vec2) -> Self {
        Isometry2d {
            rotation: Rot2::IDENTITY,
            translation,
        }
    }

    /// The inverse isometry that undoes this one.
    pub fn inverse(self) -> Self {
        let inv_rot = self.rotation.inverse();
        Isometry2d {
            rotation: inv_rot,
            translation: inv_rot * -self.translation,
        }
    }
}

impl From<Isometry2d> for Affine2 {
    fn from(iso: Isometry2d) -> Self {
        Affine2 {
            matrix2: iso.rotation.into(),
            translation: iso.translation,
        }
    }
}

impl Mul for Isometry2d {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Isometry2d {
            rotation: self.rotation * rhs.rotation,
            translation: self.rotation * rhs.translation + self.translation,
        }
    }
}

/// An isometry in three dimensions.
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

    /// Create a three-dimensional isometry from a translation and rotation.
    pub fn new(translation: Vec3, rotation: Quat) -> Self {
        Isometry3d {
            rotation,
            translation: translation.into(),
        }
    }

    /// Create a three-dimensional isometry from a rotation.
    pub fn from_rotation(rotation: Quat) -> Self {
        Isometry3d {
            rotation,
            translation: Vec3A::ZERO,
        }
    }

    /// Create a three-dimensional isometry from a translation.
    pub fn from_translation(translation: impl Into<Vec3A>) -> Self {
        Isometry3d {
            rotation: Quat::IDENTITY,
            translation: translation.into(),
        }
    }

    /// The inverse isometry that undoes this one.
    pub fn inverse(self) -> Self {
        let inv_rot = self.rotation.inverse();
        Isometry3d {
            rotation: inv_rot,
            translation: inv_rot * -self.translation,
        }
    }
}

impl From<Isometry3d> for Affine3 {
    fn from(iso: Isometry3d) -> Self {
        Affine3 {
            matrix3: Mat3::from_quat(iso.rotation),
            translation: iso.translation.into(),
        }
    }
}

impl From<Isometry3d> for Affine3A {
    fn from(iso: Isometry3d) -> Self {
        Affine3A {
            matrix3: Mat3A::from_quat(iso.rotation),
            translation: iso.translation,
        }
    }
}

impl Mul for Isometry3d {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Isometry3d {
            rotation: self.rotation * rhs.rotation,
            translation: self.rotation * rhs.translation + self.translation,
        }
    }
}
