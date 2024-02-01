use glam::FloatExt;

use crate::prelude::{Direction2d, Mat2, Vec2};

/// A counterclockwise 2D rotation in radians.
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

    /// Creates a [`Rotation2d`] from a counterclockwise angle in radians.
    ///
    /// This is equivalent to [`Self::from_radians`].
    #[inline]
    pub fn new(radians: f32) -> Self {
        Self::from_radians(radians)
    }

    /// Creates a [`Rotation2d`] from the sine and cosine of an angle in radians.
    #[inline]
    pub const fn from_sin_cos(sin: f32, cos: f32) -> Self {
        Self { sin, cos }
    }

    /// Creates a [`Rotation2d`] from a counterclockwise angle in radians.
    #[inline]
    pub fn from_radians(radians: f32) -> Self {
        #[cfg(feature = "libm")]
        let (sin, cos) = libm::sin_cos(radians);
        #[cfg(not(feature = "libm"))]
        let (sin, cos) = radians.sin_cos();

        Self::from_sin_cos(sin, cos)
    }

    /// Creates a [`Rotation2d`] from a counterclockwise angle in degrees.
    #[inline]
    pub fn from_degrees(degrees: f32) -> Self {
        Self::from_radians(degrees.to_radians())
    }

    /// Returns the rotation in radians in the `]-pi, pi]` range.
    #[inline]
    pub fn as_radians(self) -> f32 {
        #[cfg(feature = "libm")]
        {
            libm::atan2(self.sin, self.cos)
        }
        #[cfg(not(feature = "libm"))]
        {
            f32::atan2(self.sin, self.cos)
        }
    }

    /// Returns the rotation in degrees in the `]-180, 180]` range.
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
        self.sin.is_nan() && self.cos.is_nan()
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
    /// let rot2 = Rotation2d::from_radians(std::f32::consts::FRAC_PI_2);
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
    /// let rot1 = Rotation2d::from_radians(std::f32::consts::FRAC_PI_4);
    /// let rot2 = Rotation2d::from_radians(-std::f32::consts::PI);
    ///
    /// let result = rot1.slerp(rot2, 1.0 / 3.0);
    ///
    /// assert_eq!(result.as_radians(), std::f32::consts::FRAC_PI_2);
    /// ```
    #[inline]
    pub fn slerp(self, end: Self, s: f32) -> Self {
        let delta = end * self.inverse();
        self * Self::from_radians(delta.as_radians() * s)
    }
}

impl From<f32> for Rotation2d {
    /// Creates a [`Rotation2d`] from a counterclockwise angle in radians.
    fn from(rotation: f32) -> Self {
        Self::from_radians(rotation)
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

    fn mul(self, rhs: Direction2d) -> Self::Output {
        Direction2d::new_unchecked(self * *rhs)
    }
}
