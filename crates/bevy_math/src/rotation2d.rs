use glam::FloatExt;

use crate::prelude::{Mat2, Vec2};

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
    ///
    /// # Panics
    ///
    /// Panics if `sin * sin + cos * cos != 1.0` when the `glam_assert` feature is enabled.
    #[inline]
    pub fn from_sin_cos(sin: f32, cos: f32) -> Self {
        let rotation = Self { sin, cos };
        debug_assert!(
            rotation.is_normalized(),
            "the given sine and cosine produce an invalid rotation"
        );
        rotation
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

    /// Computes the length or norm of the complex number used to represent the rotation.
    ///
    /// The length is typically expected to be `1.0`. Unexpectedly denormalized rotations
    /// can be a result of incorrect construction or floating point error caused by
    /// successive operations.
    #[inline]
    #[doc(alias = "norm")]
    pub fn length(self) -> f32 {
        Vec2::new(self.sin, self.cos).length()
    }

    /// Computes the squared length or norm of the complex number used to represent the rotation.
    ///
    /// This is generally faster than [`Rotation2d::length()`], as it avoids a square
    /// root operation.
    ///
    /// The length is typically expected to be `1.0`. Unexpectedly denormalized rotations
    /// can be a result of incorrect construction or floating point error caused by
    /// successive operations.
    #[inline]
    #[doc(alias = "norm2")]
    pub fn length_squared(self) -> f32 {
        Vec2::new(self.sin, self.cos).length_squared()
    }

    /// Computes `1.0 / self.length()`.
    ///
    /// For valid results, `self` must _not_ have a length of zero.
    #[inline]
    pub fn length_recip(self) -> f32 {
        Vec2::new(self.sin, self.cos).length_recip()
    }

    /// Returns `self` with a length of `1.0` if possible, and `None` otherwise.
    ///
    /// `None` will be returned if the sine and cosine of `self` are both zero (or very close to zero),
    /// or if either of them is NaN or infinite.
    ///
    /// Note that [`Rotation2d`] should typically already be normalized by design.
    /// Manual normalization is only needed when successive operations result in
    /// accumulated floating point error, or if the rotation was constructed
    /// with invalid values.
    #[inline]
    pub fn try_normalize(self) -> Option<Self> {
        let recip = self.length_recip();
        if recip.is_finite() && recip > 0.0 {
            Some(Self::from_sin_cos(self.sin * recip, self.cos * recip))
        } else {
            None
        }
    }

    /// Returns `self` with a length of `1.0`.
    ///
    /// Note that [`Rotation2d`] should typically already be normalized by design.
    /// Manual normalization is only needed when successive operations result in
    /// accumulated floating point error, or if the rotation was constructed
    /// with invalid values.
    ///
    /// # Panics
    ///
    /// Panics if `self` has a length of zero, NaN, or infinity when the `glam_assert`
    /// feature is enabled.
    #[inline]
    pub fn normalize(self) -> Self {
        let length = self.length();
        Self::from_sin_cos(self.sin / length, self.cos / length)
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

    /// Returns whether `self` has a length of `1.0` or not.
    ///
    /// Uses a precision threshold of `1e-6`.
    #[inline]
    pub fn is_normalized(self) -> bool {
        let length = self.sin.hypot(self.cos);
        length - 1.0 <= 1e-6
    }

    /// Returns `true` if the rotation is near [`Rotation2d::IDENTITY`].
    #[inline]
    pub fn is_near_identity(self) -> bool {
        // Same as `Quat::is_near_identity`, but using sine and cosine
        let threshold_angle_sin = 0.000_049_692_047; // let threshold_angle = 0.002_847_144_6;
        self.cos > 0.0 && self.sin.abs() < threshold_angle_sin
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
    /// the value `s`, and normalizes the rotation afterwards.
    ///
    /// When `s == 0.0`, the result will be equal to `self`.
    /// When `s == 1.0`, the result will be equal to `rhs`.
    ///
    /// This is slightly more efficient than [`slerp`](Self::slerp), and produces a similar result
    /// when the difference between the two rotations is small. At larger differences,
    /// the result resembles a kind of ease-in-out effect.
    ///
    /// If you would like the angular velocity to remain constant, consider using [`slerp`](Self::slerp) instead.
    ///
    /// # Details
    ///
    /// `nlerp` corresponds to computing an angle for a point at position `s` on a line drawn
    /// between the endpoints of the arc formed by `self` and `rhs` on a unit circle,
    /// and normalizing the result afterwards.
    ///
    /// Note that if the angles are opposite like 0 and π, the line will pass through the origin,
    /// and the resulting angle will always be either `self` or `rhs` depending on `s`.
    /// If `s` happens to be `0.5` in this case, a valid rotation cannot be computed, and `self`
    /// will be returned as a fallback.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::Rotation2d;
    /// #
    /// let rot1 = Rotation2d::IDENTITY;
    /// let rot2 = Rotation2d::degrees(135.0);
    ///
    /// let result1 = rot1.nlerp(rot2, 1.0 / 3.0);
    /// assert_eq!(result1.as_degrees(), 28.675055);
    ///
    /// let result2 = rot1.nlerp(rot2, 0.5);
    /// assert_eq!(result2.as_degrees(), 67.5);
    /// ```
    #[inline]
    pub fn nlerp(self, end: Self, s: f32) -> Self {
        Self {
            sin: self.sin.lerp(end.sin, s),
            cos: self.cos.lerp(end.cos, s),
        }
        .try_normalize()
        // Fall back to the start rotation.
        // This can happen when `self` and `end` are opposite angles and `s == 0.5`,
        // because the resulting rotation would be zero, which cannot be normalized.
        .unwrap_or(self)
    }

    /// Performs a spherical linear interpolation between `self` and `end`
    /// based on the value `s`.
    ///
    /// This corresponds to interpolating between the two angles at a constant angular velocity.
    ///
    /// When `s == 0.0`, the result will be equal to `self`.
    /// When `s == 1.0`, the result will be equal to `rhs`.
    ///
    /// If you would like the rotation to have a kind of ease-in-out effect, consider
    /// using the slightly more efficient [`nlerp`](Self::nlerp) instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::Rotation2d;
    /// #
    /// let rot1 = Rotation2d::IDENTITY;
    /// let rot2 = Rotation2d::degrees(135.0);
    ///
    /// let result1 = rot1.slerp(rot2, 1.0 / 3.0);
    /// assert_eq!(result1.as_degrees(), 45.0);
    ///
    /// let result2 = rot1.slerp(rot2, 0.5);
    /// assert_eq!(result2.as_degrees(), 67.5);
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

    use crate::{Dir2, Rotation2d, Vec2};

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
        assert_relative_eq!(rotation * Dir2::Y, Dir2::NEG_X);
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
    fn length() {
        let rotation = Rotation2d {
            sin: 10.0,
            cos: 5.0,
        };

        assert_eq!(rotation.length_squared(), 125.0);
        assert_eq!(rotation.length(), 11.18034);
        assert!((rotation.normalize().length() - 1.0).abs() < 10e-7);
    }

    #[test]
    fn is_near_identity() {
        assert!(!Rotation2d::radians(0.1).is_near_identity());
        assert!(!Rotation2d::radians(-0.1).is_near_identity());
        assert!(Rotation2d::radians(0.00001).is_near_identity());
        assert!(Rotation2d::radians(-0.00001).is_near_identity());
        assert!(Rotation2d::radians(0.0).is_near_identity());
    }

    #[test]
    fn normalize() {
        let rotation = Rotation2d {
            sin: 10.0,
            cos: 5.0,
        };
        let normalized_rotation = rotation.normalize();

        assert_eq!(normalized_rotation.sin, 0.8944272);
        assert_eq!(normalized_rotation.cos, 0.4472136);

        assert!(!rotation.is_normalized());
        assert!(normalized_rotation.is_normalized());
    }

    #[test]
    fn try_normalize() {
        // Valid
        assert!(Rotation2d {
            sin: 10.0,
            cos: 5.0,
        }
        .try_normalize()
        .is_some());

        // NaN
        assert!(Rotation2d {
            sin: f32::NAN,
            cos: 5.0,
        }
        .try_normalize()
        .is_none());

        // Zero
        assert!(Rotation2d { sin: 0.0, cos: 0.0 }.try_normalize().is_none());

        // Non-finite
        assert!(Rotation2d {
            sin: f32::INFINITY,
            cos: 5.0,
        }
        .try_normalize()
        .is_none());
    }

    #[test]
    fn nlerp() {
        let rot1 = Rotation2d::IDENTITY;
        let rot2 = Rotation2d::degrees(135.0);

        assert_eq!(rot1.nlerp(rot2, 1.0 / 3.0).as_degrees(), 28.675055);
        assert!(rot1.nlerp(rot2, 0.0).is_near_identity());
        assert_eq!(rot1.nlerp(rot2, 0.5).as_degrees(), 67.5);
        assert_eq!(rot1.nlerp(rot2, 1.0).as_degrees(), 135.0);

        let rot1 = Rotation2d::IDENTITY;
        let rot2 = Rotation2d::from_sin_cos(0.0, -1.0);

        assert!(rot1.nlerp(rot2, 1.0 / 3.0).is_near_identity());
        assert!(rot1.nlerp(rot2, 0.0).is_near_identity());
        // At 0.5, there is no valid rotation, so the fallback is the original angle.
        assert_eq!(rot1.nlerp(rot2, 0.5).as_degrees(), 0.0);
        assert_eq!(rot1.nlerp(rot2, 1.0).as_degrees().abs(), 180.0);
    }

    #[test]
    fn slerp() {
        let rot1 = Rotation2d::IDENTITY;
        let rot2 = Rotation2d::degrees(135.0);

        assert_eq!(rot1.slerp(rot2, 1.0 / 3.0).as_degrees(), 45.0);
        assert!(rot1.slerp(rot2, 0.0).is_near_identity());
        assert_eq!(rot1.slerp(rot2, 0.5).as_degrees(), 67.5);
        assert_eq!(rot1.slerp(rot2, 1.0).as_degrees(), 135.0);

        let rot1 = Rotation2d::IDENTITY;
        let rot2 = Rotation2d::from_sin_cos(0.0, -1.0);

        assert!((rot1.slerp(rot2, 1.0 / 3.0).as_degrees() - 60.0).abs() < 10e-6);
        assert!(rot1.slerp(rot2, 0.0).is_near_identity());
        assert_eq!(rot1.slerp(rot2, 0.5).as_degrees(), 90.0);
        assert_eq!(rot1.slerp(rot2, 1.0).as_degrees().abs(), 180.0);
    }
}
