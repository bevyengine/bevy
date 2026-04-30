//! Isometry types for expressing rigid motions in two and three dimensions.

use crate::{Affine2, Affine3, Affine3A, Dir2, Dir3, Mat3, Mat3A, Quat, Rot2, Vec2, Vec3, Vec3A};
use core::ops::Mul;

#[cfg(feature = "approx")]
use approx::{AbsDiffEq, RelativeEq, UlpsEq};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// An isometry in two dimensions, representing a rotation followed by a translation.
/// This can often be useful for expressing relative positions and transformations from one position to another.
///
/// In particular, this type represents a distance-preserving transformation known as a *rigid motion* or a *direct motion*,
/// and belongs to the special [Euclidean group] SE(2). This includes translation and rotation, but excludes reflection.
///
/// For the three-dimensional version, see [`Isometry3d`].
///
/// [Euclidean group]: https://en.wikipedia.org/wiki/Euclidean_group
///
/// # Example
///
/// Isometries can be created from a given translation and rotation:
///
/// ```
/// # use bevy_math::{Isometry2d, Rot2, Vec2};
/// #
/// let iso = Isometry2d::new(Vec2::new(2.0, 1.0), Rot2::degrees(90.0));
/// ```
///
/// Or from separate parts:
///
/// ```
/// # use bevy_math::{Isometry2d, Rot2, Vec2};
/// #
/// let iso1 = Isometry2d::from_translation(Vec2::new(2.0, 1.0));
/// let iso2 = Isometry2d::from_rotation(Rot2::degrees(90.0));
/// ```
///
/// The isometries can be used to transform points:
///
/// ```
/// # use approx::assert_abs_diff_eq;
/// # use bevy_math::{Isometry2d, Rot2, Vec2};
/// #
/// let iso = Isometry2d::new(Vec2::new(2.0, 1.0), Rot2::degrees(90.0));
/// let point = Vec2::new(4.0, 4.0);
///
/// // These are equivalent
/// let result = iso.transform_point(point);
/// let result = iso * point;
///
/// assert_eq!(result, Vec2::new(-2.0, 5.0));
/// ```
///
/// Isometries can also be composed together:
///
/// ```
/// # use bevy_math::{Isometry2d, Rot2, Vec2};
/// #
/// # let iso = Isometry2d::new(Vec2::new(2.0, 1.0), Rot2::degrees(90.0));
/// # let iso1 = Isometry2d::from_translation(Vec2::new(2.0, 1.0));
/// # let iso2 = Isometry2d::from_rotation(Rot2::degrees(90.0));
/// #
/// assert_eq!(iso1 * iso2, iso);
/// ```
///
/// One common operation is to compute an isometry representing the relative positions of two objects
/// for things like intersection tests. This can be done with an inverse transformation:
///
/// ```
/// # use bevy_math::{Isometry2d, Rot2, Vec2};
/// #
/// let circle_iso = Isometry2d::from_translation(Vec2::new(2.0, 1.0));
/// let rectangle_iso = Isometry2d::from_rotation(Rot2::degrees(90.0));
///
/// // Compute the relative position and orientation between the two shapes
/// let relative_iso = circle_iso.inverse() * rectangle_iso;
///
/// // Or alternatively, to skip an extra rotation operation:
/// let relative_iso = circle_iso.inverse_mul(rectangle_iso);
/// ```
#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default, Clone)
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
    pub const fn new(translation: Vec2, rotation: Rot2) -> Self {
        Isometry2d {
            rotation,
            translation,
        }
    }

    /// Create a two-dimensional isometry from a rotation.
    #[inline]
    pub const fn from_rotation(rotation: Rot2) -> Self {
        Isometry2d {
            rotation,
            translation: Vec2::ZERO,
        }
    }

    /// Create a two-dimensional isometry from a translation.
    #[inline]
    pub const fn from_translation(translation: Vec2) -> Self {
        Isometry2d {
            rotation: Rot2::IDENTITY,
            translation,
        }
    }

    /// Create a two-dimensional isometry from a translation with the given `x` and `y` components.
    #[inline]
    pub const fn from_xy(x: f32, y: f32) -> Self {
        Isometry2d {
            rotation: Rot2::IDENTITY,
            translation: Vec2::new(x, y),
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

impl From<Vec2> for Isometry2d {
    #[inline]
    fn from(translation: Vec2) -> Self {
        Isometry2d::from_translation(translation)
    }
}

impl From<Rot2> for Isometry2d {
    #[inline]
    fn from(rotation: Rot2) -> Self {
        Isometry2d::from_rotation(rotation)
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

/// An isometry in three dimensions, representing a rotation followed by a translation.
/// This can often be useful for expressing relative positions and transformations from one position to another.
///
/// In particular, this type represents a distance-preserving transformation known as a *rigid motion* or a *direct motion*,
/// and belongs to the special [Euclidean group] SE(3). This includes translation and rotation, but excludes reflection.
///
/// For the two-dimensional version, see [`Isometry2d`].
///
/// [Euclidean group]: https://en.wikipedia.org/wiki/Euclidean_group
///
/// # Example
///
/// Isometries can be created from a given translation and rotation:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let iso = Isometry3d::new(Vec3::new(2.0, 1.0, 3.0), Quat::from_rotation_z(FRAC_PI_2));
/// ```
///
/// Or from separate parts:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let iso1 = Isometry3d::from_translation(Vec3::new(2.0, 1.0, 3.0));
/// let iso2 = Isometry3d::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
/// ```
///
/// The isometries can be used to transform points:
///
/// ```
/// # use approx::assert_relative_eq;
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let iso = Isometry3d::new(Vec3::new(2.0, 1.0, 3.0), Quat::from_rotation_z(FRAC_PI_2));
/// let point = Vec3::new(4.0, 4.0, 4.0);
///
/// // These are equivalent
/// let result = iso.transform_point(point);
/// let result = iso * point;
///
/// assert_relative_eq!(result, Vec3::new(-2.0, 5.0, 7.0));
/// ```
///
/// Isometries can also be composed together:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// # let iso = Isometry3d::new(Vec3::new(2.0, 1.0, 3.0), Quat::from_rotation_z(FRAC_PI_2));
/// # let iso1 = Isometry3d::from_translation(Vec3::new(2.0, 1.0, 3.0));
/// # let iso2 = Isometry3d::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
/// #
/// assert_eq!(iso1 * iso2, iso);
/// ```
///
/// One common operation is to compute an isometry representing the relative positions of two objects
/// for things like intersection tests. This can be done with an inverse transformation:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let sphere_iso = Isometry3d::from_translation(Vec3::new(2.0, 1.0, 3.0));
/// let cuboid_iso = Isometry3d::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
///
/// // Compute the relative position and orientation between the two shapes
/// let relative_iso = sphere_iso.inverse() * cuboid_iso;
///
/// // Or alternatively, to skip an extra rotation operation:
/// let relative_iso = sphere_iso.inverse_mul(cuboid_iso);
/// ```
#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default, Clone)
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
    pub const fn from_rotation(rotation: Quat) -> Self {
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

    /// Create a three-dimensional isometry from a translation with the given `x`, `y`, and `z` components.
    #[inline]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Isometry3d {
            rotation: Quat::IDENTITY,
            translation: Vec3A::new(x, y, z),
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

impl From<Vec3> for Isometry3d {
    #[inline]
    fn from(translation: Vec3) -> Self {
        Isometry3d::from_translation(translation)
    }
}

impl From<Vec3A> for Isometry3d {
    #[inline]
    fn from(translation: Vec3A) -> Self {
        Isometry3d::from_translation(translation)
    }
}

impl From<Quat> for Isometry3d {
    #[inline]
    fn from(rotation: Quat) -> Self {
        Isometry3d::from_rotation(rotation)
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
#[cfg(feature = "approx")]
mod tests {
    use super::*;
    use crate::{vec2, vec3, vec3a};
    use approx::assert_abs_diff_eq;
    use core::f32::consts::{FRAC_PI_2, FRAC_PI_3};

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
