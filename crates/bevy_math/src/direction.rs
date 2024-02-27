use crate::{
    primitives::{Primitive2d, Primitive3d},
    Quat, Vec2, Vec3, Vec3A,
};

/// An error indicating that a direction is invalid.
#[derive(Debug, PartialEq)]
pub enum InvalidDirectionError {
    /// The length of the direction vector is zero or very close to zero.
    Zero,
    /// The length of the direction vector is `std::f32::INFINITY`.
    Infinite,
    /// The length of the direction vector is `NaN`.
    NaN,
}

impl InvalidDirectionError {
    /// Creates an [`InvalidDirectionError`] from the length of an invalid direction vector.
    pub fn from_length(length: f32) -> Self {
        if length.is_nan() {
            InvalidDirectionError::NaN
        } else if !length.is_finite() {
            // If the direction is non-finite but also not NaN, it must be infinite
            InvalidDirectionError::Infinite
        } else {
            // If the direction is invalid but neither NaN nor infinite, it must be zero
            InvalidDirectionError::Zero
        }
    }
}

impl std::fmt::Display for InvalidDirectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Direction can not be zero (or very close to zero), or non-finite."
        )
    }
}

/// A normalized vector pointing in a direction in 2D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Direction2d(Vec2);
impl Primitive2d for Direction2d {}

impl Direction2d {
    /// A unit vector pointing along the positive X axis.
    pub const X: Self = Self(Vec2::X);
    /// A unit vector pointing along the positive Y axis.
    pub const Y: Self = Self(Vec2::Y);
    /// A unit vector pointing along the negative X axis.
    pub const NEG_X: Self = Self(Vec2::NEG_X);
    /// A unit vector pointing along the negative Y axis.
    pub const NEG_Y: Self = Self(Vec2::NEG_Y);

    /// Create a direction from a finite, nonzero [`Vec2`].
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new(value: Vec2) -> Result<Self, InvalidDirectionError> {
        Self::new_and_length(value).map(|(dir, _)| dir)
    }

    /// Create a [`Direction2d`] from a [`Vec2`] that is already normalized.
    ///
    /// # Warning
    ///
    /// `value` must be normalized, i.e it's length must be `1.0`.
    pub fn new_unchecked(value: Vec2) -> Self {
        debug_assert!(value.is_normalized());

        Self(value)
    }

    /// Create a direction from a finite, nonzero [`Vec2`], also returning its original length.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new_and_length(value: Vec2) -> Result<(Self, f32), InvalidDirectionError> {
        let length = value.length();
        let direction = (length.is_finite() && length > 0.0).then_some(value / length);

        direction
            .map(|dir| (Self(dir), length))
            .ok_or(InvalidDirectionError::from_length(length))
    }

    /// Create a direction from its `x` and `y` components.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the vector formed by the components is zero (or very close to zero), infinite, or `NaN`.
    pub fn from_xy(x: f32, y: f32) -> Result<Self, InvalidDirectionError> {
        Self::new(Vec2::new(x, y))
    }
}

impl TryFrom<Vec2> for Direction2d {
    type Error = InvalidDirectionError;

    fn try_from(value: Vec2) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::ops::Deref for Direction2d {
    type Target = Vec2;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Neg for Direction2d {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

#[cfg(feature = "approx")]
impl approx::AbsDiffEq for Direction2d {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.as_ref().abs_diff_eq(other.as_ref(), epsilon)
    }
}

#[cfg(feature = "approx")]
impl approx::RelativeEq for Direction2d {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.as_ref()
            .relative_eq(other.as_ref(), epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl approx::UlpsEq for Direction2d {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.as_ref().ulps_eq(other.as_ref(), epsilon, max_ulps)
    }
}

/// A normalized vector pointing in a direction in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Direction3d(Vec3);
impl Primitive3d for Direction3d {}

impl Direction3d {
    /// A unit vector pointing along the positive X axis.
    pub const X: Self = Self(Vec3::X);
    /// A unit vector pointing along the positive Y axis.
    pub const Y: Self = Self(Vec3::Y);
    /// A unit vector pointing along the positive Z axis.
    pub const Z: Self = Self(Vec3::Z);
    /// A unit vector pointing along the negative X axis.
    pub const NEG_X: Self = Self(Vec3::NEG_X);
    /// A unit vector pointing along the negative Y axis.
    pub const NEG_Y: Self = Self(Vec3::NEG_Y);
    /// A unit vector pointing along the negative Z axis.
    pub const NEG_Z: Self = Self(Vec3::NEG_Z);

    /// Create a direction from a finite, nonzero [`Vec3`].
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new(value: Vec3) -> Result<Self, InvalidDirectionError> {
        Self::new_and_length(value).map(|(dir, _)| dir)
    }

    /// Create a [`Direction3d`] from a [`Vec3`] that is already normalized.
    ///
    /// # Warning
    ///
    /// `value` must be normalized, i.e it's length must be `1.0`.
    pub fn new_unchecked(value: Vec3) -> Self {
        debug_assert!(value.is_normalized());

        Self(value)
    }

    /// Create a direction from a finite, nonzero [`Vec3`], also returning its original length.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new_and_length(value: Vec3) -> Result<(Self, f32), InvalidDirectionError> {
        let length = value.length();
        let direction = (length.is_finite() && length > 0.0).then_some(value / length);

        direction
            .map(|dir| (Self(dir), length))
            .ok_or(InvalidDirectionError::from_length(length))
    }

    /// Create a direction from its `x`, `y`, and `z` components.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the vector formed by the components is zero (or very close to zero), infinite, or `NaN`.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Result<Self, InvalidDirectionError> {
        Self::new(Vec3::new(x, y, z))
    }
}

impl TryFrom<Vec3> for Direction3d {
    type Error = InvalidDirectionError;

    fn try_from(value: Vec3) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Direction3d> for Vec3 {
    fn from(value: Direction3d) -> Self {
        value.0
    }
}

impl std::ops::Deref for Direction3d {
    type Target = Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Neg for Direction3d {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl std::ops::Mul<f32> for Direction3d {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Self::Output {
        self.0 * rhs
    }
}

impl std::ops::Mul<Direction3d> for Quat {
    type Output = Direction3d;

    /// Rotates the [`Direction3d`] using a [`Quat`].
    fn mul(self, direction: Direction3d) -> Self::Output {
        let rotated = self * *direction;

        // Make sure the result is normalized.
        // This can fail for non-unit quaternions.
        debug_assert!(rotated.is_normalized());

        Direction3d::new_unchecked(rotated)
    }
}

#[cfg(feature = "approx")]
impl approx::AbsDiffEq for Direction3d {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.as_ref().abs_diff_eq(other.as_ref(), epsilon)
    }
}

#[cfg(feature = "approx")]
impl approx::RelativeEq for Direction3d {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.as_ref()
            .relative_eq(other.as_ref(), epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl approx::UlpsEq for Direction3d {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.as_ref().ulps_eq(other.as_ref(), epsilon, max_ulps)
    }
}

/// A normalized SIMD vector pointing in a direction in 3D space.
///
/// This type stores a 16 byte aligned [`Vec3A`].
/// This may or may not be faster than [`Direction3d`]: make sure to benchmark!
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Direction3dA(Vec3A);
impl Primitive3d for Direction3dA {}

impl Direction3dA {
    /// A unit vector pointing along the positive X axis.
    pub const X: Self = Self(Vec3A::X);
    /// A unit vector pointing along the positive Y axis.
    pub const Y: Self = Self(Vec3A::Y);
    /// A unit vector pointing along the positive Z axis.
    pub const Z: Self = Self(Vec3A::Z);
    /// A unit vector pointing along the negative X axis.
    pub const NEG_X: Self = Self(Vec3A::NEG_X);
    /// A unit vector pointing along the negative Y axis.
    pub const NEG_Y: Self = Self(Vec3A::NEG_Y);
    /// A unit vector pointing along the negative Z axis.
    pub const NEG_Z: Self = Self(Vec3A::NEG_Z);

    /// Create a direction from a finite, nonzero [`Vec3A`].
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new(value: Vec3A) -> Result<Self, InvalidDirectionError> {
        Self::new_and_length(value).map(|(dir, _)| dir)
    }

    /// Create a [`Direction3dA`] from a [`Vec3A`] that is already normalized.
    ///
    /// # Warning
    ///
    /// `value` must be normalized, i.e it's length must be `1.0`.
    pub fn new_unchecked(value: Vec3A) -> Self {
        debug_assert!(value.is_normalized());

        Self(value)
    }

    /// Create a direction from a finite, nonzero [`Vec3A`], also returning its original length.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new_and_length(value: Vec3A) -> Result<(Self, f32), InvalidDirectionError> {
        let length = value.length();
        let direction = (length.is_finite() && length > 0.0).then_some(value / length);

        direction
            .map(|dir| (Self(dir), length))
            .ok_or(InvalidDirectionError::from_length(length))
    }

    /// Create a direction from its `x`, `y`, and `z` components.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the vector formed by the components is zero (or very close to zero), infinite, or `NaN`.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Result<Self, InvalidDirectionError> {
        Self::new(Vec3A::new(x, y, z))
    }
}

impl TryFrom<Vec3A> for Direction3dA {
    type Error = InvalidDirectionError;

    fn try_from(value: Vec3A) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Direction3dA> for Vec3A {
    fn from(value: Direction3dA) -> Self {
        value.0
    }
}

impl std::ops::Deref for Direction3dA {
    type Target = Vec3A;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Neg for Direction3dA {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl std::ops::Mul<f32> for Direction3dA {
    type Output = Vec3A;
    fn mul(self, rhs: f32) -> Self::Output {
        self.0 * rhs
    }
}

impl std::ops::Mul<Direction3dA> for Quat {
    type Output = Direction3dA;

    /// Rotates the [`Direction3dA`] using a [`Quat`].
    fn mul(self, direction: Direction3dA) -> Self::Output {
        let rotated = self * *direction;

        // Make sure the result is normalized.
        // This can fail for non-unit quaternions.
        debug_assert!(rotated.is_normalized());

        Direction3dA::new_unchecked(rotated)
    }
}

#[cfg(feature = "approx")]
impl approx::AbsDiffEq for Direction3dA {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.as_ref().abs_diff_eq(other.as_ref(), epsilon)
    }
}

#[cfg(feature = "approx")]
impl approx::RelativeEq for Direction3dA {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.as_ref()
            .relative_eq(other.as_ref(), epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl approx::UlpsEq for Direction3dA {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.as_ref().ulps_eq(other.as_ref(), epsilon, max_ulps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvalidDirectionError;

    #[test]
    fn dir2_creation() {
        assert_eq!(Direction2d::new(Vec2::X * 12.5), Ok(Direction2d::X));
        assert_eq!(
            Direction2d::new(Vec2::new(0.0, 0.0)),
            Err(InvalidDirectionError::Zero)
        );
        assert_eq!(
            Direction2d::new(Vec2::new(f32::INFINITY, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction2d::new(Vec2::new(f32::NEG_INFINITY, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction2d::new(Vec2::new(f32::NAN, 0.0)),
            Err(InvalidDirectionError::NaN)
        );
        assert_eq!(
            Direction2d::new_and_length(Vec2::X * 6.5),
            Ok((Direction2d::X, 6.5))
        );
    }

    #[test]
    fn dir3_creation() {
        assert_eq!(Direction3d::new(Vec3::X * 12.5), Ok(Direction3d::X));
        assert_eq!(
            Direction3d::new(Vec3::new(0.0, 0.0, 0.0)),
            Err(InvalidDirectionError::Zero)
        );
        assert_eq!(
            Direction3d::new(Vec3::new(f32::INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction3d::new(Vec3::new(f32::NEG_INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction3d::new(Vec3::new(f32::NAN, 0.0, 0.0)),
            Err(InvalidDirectionError::NaN)
        );
        assert_eq!(
            Direction3d::new_and_length(Vec3::X * 6.5),
            Ok((Direction3d::X, 6.5))
        );

        // Test rotation
        assert!(
            (Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Direction3d::X)
                .abs_diff_eq(Vec3::Y, 10e-6)
        );
    }

    #[test]
    fn dir3a_creation() {
        assert_eq!(Direction3dA::new(Vec3A::X * 12.5), Ok(Direction3dA::X));
        assert_eq!(
            Direction3dA::new(Vec3A::new(0.0, 0.0, 0.0)),
            Err(InvalidDirectionError::Zero)
        );
        assert_eq!(
            Direction3dA::new(Vec3A::new(f32::INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction3dA::new(Vec3A::new(f32::NEG_INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Direction3dA::new(Vec3A::new(f32::NAN, 0.0, 0.0)),
            Err(InvalidDirectionError::NaN)
        );
        assert_eq!(
            Direction3dA::new_and_length(Vec3A::X * 6.5),
            Ok((Direction3dA::X, 6.5))
        );

        // Test rotation
        assert!(
            (Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Direction3dA::X)
                .abs_diff_eq(Vec3A::Y, 10e-6)
        );
    }
}
