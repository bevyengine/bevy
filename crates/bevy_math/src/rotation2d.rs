use glam::FloatExt;

use crate::prelude::{Direction2d, Mat2, Vec2};

/// A counterclockwise 2D rotation in radians.
///
/// The rotation angle is wrapped to be within the `(-pi, pi]` range.
///
/// # Example
///
/// ```
/// # use approx::assert_relative_eq;
/// # use bevy_math::{Rotation2d, Vec2};
/// use std::f32::consts::PI;
///
/// // Create rotatons from radians or degrees
/// let rotation1 = Rotation2d::radians(PI / 2.0);
/// let rotation2 = Rotation2d::degrees(45.0);
///
/// // Get the angle back as radians or degrees
/// assert_eq!(rotation1.as_degrees(), 90.0);
/// assert_eq!(rotation2.as_radians(), PI / 4.0);
///
/// // "Add" rotations together using `*`
/// assert_relative_eq!(rotation1 * rotation2, Rotation2d::degrees(135.0));
///
/// // Rotate vectors
/// assert_relative_eq!(rotation1 * Vec2::X, Vec2::Y);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Rotation2d {
    /// The cosine of the rotation angle in radians.
    ///
    /// This is the real part of the unit complex number representing the rotation.
    pub cos: f32,
    /// The sine of the rotation angle in radians.
    ///
    /// This is the imaginary part of the unit complex number representing the rotation.
    pub sin: f32,
}

impl Default for Rotation2d {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rotation2d {
    /// No rotation.
    pub const IDENTITY: Self = Self { cos: 1.0, sin: 0.0 };

    /// A rotation of π radians.
    pub const PI: Self = Self {
        cos: -1.0,
        sin: 0.0,
    };

    /// A counterclockwise rotation of π/2 radians.
    pub const FRAC_PI_2: Self = Self { cos: 0.0, sin: 1.0 };

    /// A counterclockwise rotation of π/3 radians.
    pub const FRAC_PI_3: Self = Self {
        cos: 0.5,
        sin: 0.866_025_4,
    };

    /// A counterclockwise rotation of π/4 radians.
    pub const FRAC_PI_4: Self = Self {
        cos: std::f32::consts::FRAC_1_SQRT_2,
        sin: std::f32::consts::FRAC_1_SQRT_2,
    };

    /// A counterclockwise rotation of π/6 radians.
    pub const FRAC_PI_6: Self = Self {
        cos: 0.866_025_4,
        sin: 0.5,
    };

    /// A counterclockwise rotation of π/8 radians.
    pub const FRAC_PI_8: Self = Self {
        cos: 0.923_879_5,
        sin: 0.382_683_43,
    };

    /// Creates a [`Rotation2d`] from a counterclockwise angle in radians.
    #[inline]
    pub fn radians(radians: f32) -> Self {
        #[cfg(feature = "libm")]
        let (sin, cos) = (
            libm::sin(radians as f64) as f32,
            libm::cos(radians as f64) as f32,
        );
        #[cfg(not(feature = "libm"))]
        let (sin, cos) = radians.sin_cos();

        Self::from_sin_cos(sin, cos)
    }

    /// Creates a [`Rotation2d`] from a counterclockwise angle in degrees.
    #[inline]
    pub fn degrees(degrees: f32) -> Self {
        Self::radians(degrees.to_radians())
    }

    /// Creates a [`Rotation2d`] from the sine and cosine of an angle in radians.
    ///
    /// The rotation is only valid if `sin * sin + cos * cos == 1.0`.
    #[inline]
    pub fn from_sin_cos(sin: f32, cos: f32) -> Self {
        debug_assert!(
            (sin.powi(2) + cos.powi(2) - 1.0).abs() < 10.0e-7,
            "the given sine and cosine produce an invalid rotation"
        );
        Self { sin, cos }
    }

    /// Returns the rotation in radians in the `(-pi, pi]` range.
    #[inline]
    pub fn as_radians(self) -> f32 {
        #[cfg(feature = "libm")]
        {
            libm::atan2(self.sin as f64, self.cos as f64) as f32
        }
        #[cfg(not(feature = "libm"))]
        {
            f32::atan2(self.sin, self.cos)
        }
    }

    /// Returns the rotation in degrees in the `(-180, 180]` range.
    #[inline]
    pub fn as_degrees(self) -> f32 {
        self.as_radians().to_degrees()
    }

    /// Returns the sine and cosine of the rotation angle in radians.
    #[inline]
    pub const fn sin_cos(self) -> (f32, f32) {
        (self.sin, self.cos)
    }

    /// Returns `true` if the rotation is neither infinite nor NaN.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.sin.is_finite() && self.cos.is_finite()
    }

    /// Returns `true` if the rotation is NaN.
    #[inline]
    pub fn is_nan(self) -> bool {
        self.sin.is_nan() || self.cos.is_nan()
    }

    /// Returns `true` if the rotation is near [`Rotation2d::IDENTITY`].
    #[inline]
    pub fn is_near_identity(self) -> bool {
        // TODO: We might be able to use the rotation sine and cosine
        //       directly instead of first converting to radians.
        // Same as `Quat::is_near_identity`
        let threshold_angle = 0.002_847_144_6;
        self.as_radians().abs() < threshold_angle
    }

    /// Returns the angle in radians needed to make `self` and `other` coincide.
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        (other * self.inverse()).as_radians()
    }

    /// Returns the inverse of the rotation. This is also the conjugate
    /// of the unit complex number representing the rotation.
    #[inline]
    #[must_use]
    #[doc(alias = "conjugate")]
    pub fn inverse(self) -> Self {
        Self {
            cos: self.cos,
            sin: -self.sin,
        }
    }

    /// Performs a linear interpolation between `self` and `rhs` based on
    /// the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `rhs`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::Rotation2d;
    /// #
    /// let rot1 = Rotation2d::IDENTITY;
    /// let rot2 = Rotation2d::radians(std::f32::consts::FRAC_PI_2);
    ///
    /// let result = rot1.lerp(rot2, 0.5);
    ///
    /// assert_eq!(result.as_radians(), std::f32::consts::FRAC_PI_4);
    /// ```
    #[inline]
    pub fn lerp(self, end: Self, s: f32) -> Self {
        Self::from_sin_cos(self.sin.lerp(end.sin, s), self.cos.lerp(end.cos, s))
    }

    /// Performs a spherical linear interpolation between `self` and `end`
    /// based on the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `end`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::Rotation2d;
    /// #
    /// let rot1 = Rotation2d::radians(std::f32::consts::FRAC_PI_4);
    /// let rot2 = Rotation2d::radians(-std::f32::consts::PI);
    ///
    /// let result = rot1.slerp(rot2, 1.0 / 3.0);
    ///
    /// assert_eq!(result.as_radians(), std::f32::consts::FRAC_PI_2);
    /// ```
    #[inline]
    pub fn slerp(self, end: Self, s: f32) -> Self {
        self * Self::radians(self.angle_between(end) * s)
    }
}

impl From<f32> for Rotation2d {
    /// Creates a [`Rotation2d`] from a counterclockwise angle in radians.
    fn from(rotation: f32) -> Self {
        Self::radians(rotation)
    }
}

impl From<Rotation2d> for Mat2 {
    /// Creates a [`Mat2`] rotation matrix from a [`Rotation2d`].
    fn from(rot: Rotation2d) -> Self {
        Mat2::from_cols_array(&[rot.cos, -rot.sin, rot.sin, rot.cos])
    }
}

impl std::ops::Mul for Rotation2d {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            cos: self.cos * rhs.cos - self.sin * rhs.sin,
            sin: self.sin * rhs.cos + self.cos * rhs.sin,
        }
    }
}

impl std::ops::MulAssign for Rotation2d {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::ops::Mul<Vec2> for Rotation2d {
    type Output = Vec2;

    /// Rotates a [`Vec2`] by a [`Rotation2d`].
    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2::new(
            rhs.x * self.cos - rhs.y * self.sin,
            rhs.x * self.sin + rhs.y * self.cos,
        )
    }
}

impl std::ops::Mul<Direction2d> for Rotation2d {
    type Output = Direction2d;

    /// Rotates a [`Direction2d`] by a [`Rotation2d`].
    fn mul(self, rhs: Direction2d) -> Self::Output {
        Direction2d::new_unchecked(self * *rhs)
    }
}

#[cfg(feature = "approx")]
impl approx::AbsDiffEq for Rotation2d {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.cos.abs_diff_eq(&other.cos, epsilon) && self.sin.abs_diff_eq(&other.sin, epsilon)
    }
}

#[cfg(feature = "approx")]
impl approx::RelativeEq for Rotation2d {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.cos.relative_eq(&other.cos, epsilon, max_relative)
            && self.sin.relative_eq(&other.sin, epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl approx::UlpsEq for Rotation2d {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.cos.ulps_eq(&other.cos, epsilon, max_ulps)
            && self.sin.ulps_eq(&other.sin, epsilon, max_ulps)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use crate::{primitives::Direction2d, Rotation2d, Vec2};

    #[test]
    fn creation() {
        let rotation1 = Rotation2d::radians(std::f32::consts::FRAC_PI_2);
        let rotation2 = Rotation2d::degrees(90.0);
        let rotation3 = Rotation2d::from_sin_cos(1.0, 0.0);

        // All three rotations should be equal
        assert_relative_eq!(rotation1.sin, rotation2.sin);
        assert_relative_eq!(rotation1.cos, rotation2.cos);
        assert_relative_eq!(rotation1.sin, rotation3.sin);
        assert_relative_eq!(rotation1.cos, rotation3.cos);

        // The rotation should be 90 degrees
        assert_relative_eq!(rotation1.as_radians(), std::f32::consts::FRAC_PI_2);
        assert_relative_eq!(rotation1.as_degrees(), 90.0);
    }

    #[test]
    fn rotate() {
        let rotation = Rotation2d::degrees(90.0);

        assert_relative_eq!(rotation * Vec2::X, Vec2::Y);
        assert_relative_eq!(rotation * Direction2d::Y, Direction2d::NEG_X);
    }

    #[test]
    fn add() {
        let rotation1 = Rotation2d::degrees(90.0);
        let rotation2 = Rotation2d::degrees(180.0);

        // 90 deg + 180 deg becomes -90 deg after it wraps around to be within the ]-180, 180] range
        assert_eq!((rotation1 * rotation2).as_degrees(), -90.0);
    }

    #[test]
    fn subtract() {
        let rotation1 = Rotation2d::degrees(90.0);
        let rotation2 = Rotation2d::degrees(45.0);

        assert_relative_eq!((rotation1 * rotation2.inverse()).as_degrees(), 45.0);

        // This should be equivalent to the above
        assert_relative_eq!(
            rotation2.angle_between(rotation1),
            std::f32::consts::FRAC_PI_4
        );
    }

    #[test]
    fn lerp() {
        let rotation1 = Rotation2d::IDENTITY;
        let rotation2 = Rotation2d::radians(std::f32::consts::FRAC_PI_2);
        let result = rotation1.lerp(rotation2, 0.5);
        assert_eq!(result.as_radians(), std::f32::consts::FRAC_PI_4);
    }

    #[test]
    fn slerp() {
        let rotation1 = Rotation2d::radians(std::f32::consts::FRAC_PI_4);
        let rotation2 = Rotation2d::radians(-std::f32::consts::PI);
        let result = rotation1.slerp(rotation2, 1.0 / 3.0);
        assert_eq!(result.as_radians(), std::f32::consts::FRAC_PI_2);
    }
}
