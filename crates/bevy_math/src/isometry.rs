//! Isometry types for expressing rigid motions in two and three dimensions.
//!
//! These are often used to express the relative positions of two entities (e.g. primitive shapes).
//! For example, in determining whether a sphere intersects a cube, one needs to know how the two are
//! positioned relative to one another in addition to their sizes.
//! If the two had absolute positions and orientations described by isometries `cube_iso` and `sphere_iso`,
//! then `cube_iso.inverse() * sphere_iso` would describe the relative orientation, which is sufficient for
//! answering this query.

use crate::{Affine2, Affine3, Affine3A, Dir2, Dir3, Mat3, Mat3A, Quat, Rot2, Vec2, Vec3, Vec3A};
use std::ops::Mul;

#[cfg(feature = "approx")]
use approx::{AbsDiffEq, RelativeEq, UlpsEq};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

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

    /// Compute `iso1.inverse() * iso2` in a more efficient way for one-shot cases.
    ///
    /// If the same isometry is used multiple times, it is more efficient to instead compute
    /// the inverse once and use that for each transformation.
    #[inline]
    pub fn inverse_mul(&self, rhs: Self) -> Self {
        let inv_rot = self.rotation.inverse();
        let delta_translation = rhs.translation - self.translation;
        Self::new(inv_rot * delta_translation, inv_rot * rhs.rotation)
    }

    /// Transform a point by rotating and translating it using this isometry.
    #[inline]
    pub fn transform_point(&self, point: Vec2) -> Vec2 {
        self.rotation * point + self.translation
    }

    /// Transform a point by rotating and translating it using the inverse of this isometry.
    ///
    /// This is more efficient than `iso.inverse().transform_point(point)` for one-shot cases.
    /// If the same isometry is used multiple times, it is more efficient to instead compute
    /// the inverse once and use that for each transformation.
    #[inline]
    pub fn inverse_transform_point(&self, point: Vec2) -> Vec2 {
        self.rotation.inverse() * (point - self.translation)
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

impl Mul<Dir2> for Isometry2d {
    type Output = Dir2;

    #[inline]
    fn mul(self, rhs: Dir2) -> Self::Output {
        self.rotation * rhs
    }
}

#[cfg(feature = "approx")]
impl AbsDiffEq for Isometry2d {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.rotation.abs_diff_eq(&other.rotation, epsilon)
            && self.translation.abs_diff_eq(other.translation, epsilon)
    }
}

#[cfg(feature = "approx")]
impl RelativeEq for Isometry2d {
    fn default_max_relative() -> Self::Epsilon {
        Self::default_epsilon()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.rotation
            .relative_eq(&other.rotation, epsilon, max_relative)
            && self
                .translation
                .relative_eq(&other.translation, epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl UlpsEq for Isometry2d {
    fn default_max_ulps() -> u32 {
        4
    }

    fn ulps_eq(&self, other: &Self, epsilon: Self::Epsilon, max_ulps: u32) -> bool {
        self.rotation.ulps_eq(&other.rotation, epsilon, max_ulps)
            && self
                .translation
                .ulps_eq(&other.translation, epsilon, max_ulps)
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

    /// Compute `iso1.inverse() * iso2` in a more efficient way for one-shot cases.
    ///
    /// If the same isometry is used multiple times, it is more efficient to instead compute
    /// the inverse once and use that for each transformation.
    #[inline]
    pub fn inverse_mul(&self, rhs: Self) -> Self {
        let inv_rot = self.rotation.inverse();
        let delta_translation = rhs.translation - self.translation;
        Self::new(inv_rot * delta_translation, inv_rot * rhs.rotation)
    }

    /// Transform a point by rotating and translating it using this isometry.
    #[inline]
    pub fn transform_point(&self, point: impl Into<Vec3A>) -> Vec3A {
        self.rotation * point.into() + self.translation
    }

    /// Transform a point by rotating and translating it using the inverse of this isometry.
    ///
    /// This is more efficient than `iso.inverse().transform_point(point)` for one-shot cases.
    /// If the same isometry is used multiple times, it is more efficient to instead compute
    /// the inverse once and use that for each transformation.
    #[inline]
    pub fn inverse_transform_point(&self, point: impl Into<Vec3A>) -> Vec3A {
        self.rotation.inverse() * (point.into() - self.translation)
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

impl Mul<Dir3> for Isometry3d {
    type Output = Dir3;

    #[inline]
    fn mul(self, rhs: Dir3) -> Self::Output {
        self.rotation * rhs
    }
}

#[cfg(feature = "approx")]
impl AbsDiffEq for Isometry3d {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.rotation.abs_diff_eq(other.rotation, epsilon)
            && self.translation.abs_diff_eq(other.translation, epsilon)
    }
}

#[cfg(feature = "approx")]
impl RelativeEq for Isometry3d {
    fn default_max_relative() -> Self::Epsilon {
        Self::default_epsilon()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.rotation
            .relative_eq(&other.rotation, epsilon, max_relative)
            && self
                .translation
                .relative_eq(&other.translation, epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl UlpsEq for Isometry3d {
    fn default_max_ulps() -> u32 {
        4
    }

    fn ulps_eq(&self, other: &Self, epsilon: Self::Epsilon, max_ulps: u32) -> bool {
        self.rotation.ulps_eq(&other.rotation, epsilon, max_ulps)
            && self
                .translation
                .ulps_eq(&other.translation, epsilon, max_ulps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vec2, vec3};
    use approx::assert_abs_diff_eq;
    use glam::vec3a;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};

    #[test]
    fn mul_2d() {
        let iso1 = Isometry2d::new(vec2(1.0, 0.0), Rot2::FRAC_PI_2);
        let iso2 = Isometry2d::new(vec2(0.0, 1.0), Rot2::FRAC_PI_2);
        let expected = Isometry2d::new(vec2(0.0, 0.0), Rot2::PI);
        assert_abs_diff_eq!(iso1 * iso2, expected);
    }

    #[test]
    fn inverse_mul_2d() {
        let iso1 = Isometry2d::new(vec2(1.0, 0.0), Rot2::FRAC_PI_2);
        let iso2 = Isometry2d::new(vec2(0.0, 0.0), Rot2::PI);
        let expected = Isometry2d::new(vec2(0.0, 1.0), Rot2::FRAC_PI_2);
        assert_abs_diff_eq!(iso1.inverse_mul(iso2), expected);
    }

    #[test]
    fn mul_3d() {
        let iso1 = Isometry3d::new(vec3(1.0, 0.0, 0.0), Quat::from_rotation_x(FRAC_PI_2));
        let iso2 = Isometry3d::new(vec3(0.0, 1.0, 0.0), Quat::IDENTITY);
        let expected = Isometry3d::new(vec3(1.0, 0.0, 1.0), Quat::from_rotation_x(FRAC_PI_2));
        assert_abs_diff_eq!(iso1 * iso2, expected);
    }

    #[test]
    fn inverse_mul_3d() {
        let iso1 = Isometry3d::new(vec3(1.0, 0.0, 0.0), Quat::from_rotation_x(FRAC_PI_2));
        let iso2 = Isometry3d::new(vec3(1.0, 0.0, 1.0), Quat::from_rotation_x(FRAC_PI_2));
        let expected = Isometry3d::new(vec3(0.0, 1.0, 0.0), Quat::IDENTITY);
        assert_abs_diff_eq!(iso1.inverse_mul(iso2), expected);
    }

    #[test]
    fn identity_2d() {
        let iso = Isometry2d::new(vec2(-1.0, -0.5), Rot2::degrees(75.0));
        assert_abs_diff_eq!(Isometry2d::IDENTITY * iso, iso);
        assert_abs_diff_eq!(iso * Isometry2d::IDENTITY, iso);
    }

    #[test]
    fn identity_3d() {
        let iso = Isometry3d::new(vec3(-1.0, 2.5, 3.3), Quat::from_rotation_z(FRAC_PI_3));
        assert_abs_diff_eq!(Isometry3d::IDENTITY * iso, iso);
        assert_abs_diff_eq!(iso * Isometry3d::IDENTITY, iso);
    }

    #[test]
    fn inverse_2d() {
        let iso = Isometry2d::new(vec2(-1.0, -0.5), Rot2::degrees(75.0));
        let inv = iso.inverse();
        assert_abs_diff_eq!(iso * inv, Isometry2d::IDENTITY);
        assert_abs_diff_eq!(inv * iso, Isometry2d::IDENTITY);
    }

    #[test]
    fn inverse_3d() {
        let iso = Isometry3d::new(vec3(-1.0, 2.5, 3.3), Quat::from_rotation_z(FRAC_PI_3));
        let inv = iso.inverse();
        assert_abs_diff_eq!(iso * inv, Isometry3d::IDENTITY);
        assert_abs_diff_eq!(inv * iso, Isometry3d::IDENTITY);
    }

    #[test]
    fn transform_2d() {
        let iso = Isometry2d::new(vec2(0.5, -0.5), Rot2::FRAC_PI_2);
        let point = vec2(1.0, 1.0);
        assert_abs_diff_eq!(vec2(-0.5, 0.5), iso * point);
    }

    #[test]
    fn inverse_transform_2d() {
        let iso = Isometry2d::new(vec2(0.5, -0.5), Rot2::FRAC_PI_2);
        let point = vec2(-0.5, 0.5);
        assert_abs_diff_eq!(vec2(1.0, 1.0), iso.inverse_transform_point(point));
    }

    #[test]
    fn transform_3d() {
        let iso = Isometry3d::new(vec3(1.0, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2));
        let point = vec3(1.0, 1.0, 1.0);
        assert_abs_diff_eq!(vec3(2.0, 1.0, -1.0), iso * point);
    }

    #[test]
    fn inverse_transform_3d() {
        let iso = Isometry3d::new(vec3(1.0, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2));
        let point = vec3(2.0, 1.0, -1.0);
        assert_abs_diff_eq!(vec3a(1.0, 1.0, 1.0), iso.inverse_transform_point(point));
    }
}
