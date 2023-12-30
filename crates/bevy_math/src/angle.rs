use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// An angle in radians.
///
/// # Example
///
/// ```
/// use std::f32::consts::PI;
/// use bevy_math::Angle;
///
/// // Create angles from radians or degrees
/// let alpha = Angle::radians(PI);
/// let beta = Angle::degrees(180.0);
/// assert_eq!(alpha, beta);
///
/// // Get float values
/// assert_eq!(alpha.as_radians(), PI);
/// assert_eq!(alpha.as_degrees(), 180.0);
///
/// // Use trigonometric operations
/// assert_eq!(alpha.cos(), -1.0);
///
/// // Wrap 3pi to range [-2pi, 2pi) to get pi
/// let gamma = 3.0 * alpha;
/// let wrapped = gamma.wrap();
///
/// // Small threshold for floating point error
/// assert!((wrapped - alpha).abs() < Angle::radians(0.000001));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Angle(f32);

impl Angle {
    /// An angle of zero.
    pub const ZERO: Self = Self(0.0);
    /// The angle 2π in radians.
    #[doc(alias = "TWO_PI")]
    pub const TAU: Self = Self(std::f32::consts::TAU);
    /// The angle π (pi) in radians.
    pub const PI: Self = Self(std::f32::consts::PI);
    /// The angle π/2 in radians.
    #[doc(alias = "HALF_PI")]
    pub const FRAC_PI_2: Self = Self(std::f32::consts::FRAC_PI_2);
    /// The angle π/3 in radians.
    pub const FRAC_PI_3: Self = Self(std::f32::consts::FRAC_PI_3);
    /// The angle π/4 in radians.
    pub const FRAC_PI_4: Self = Self(std::f32::consts::FRAC_PI_4);
    /// The angle π/6 in radians.
    pub const FRAC_PI_6: Self = Self(std::f32::consts::FRAC_PI_6);
    /// The angle π/8 in radians.
    pub const FRAC_PI_8: Self = Self(std::f32::consts::FRAC_PI_8);

    /// Returns the sine of the angle in radians.
    #[inline]
    pub fn sin(self) -> f32 {
        self.0.sin()
    }

    /// Returns the cosine of the angle in radians.
    #[inline]
    pub fn cos(self) -> f32 {
        self.0.cos()
    }

    /// Returns the tangent of the angle in radians.
    #[inline]
    pub fn tan(self) -> f32 {
        self.0.tan()
    }

    /// Returns the sine and cosine of the angle in radians.
    #[inline]
    pub fn sin_cos(self) -> (f32, f32) {
        self.0.sin_cos()
    }

    /// Creates an [`Angle`] from radians.
    #[inline]
    pub const fn radians(radians: f32) -> Self {
        Self(radians)
    }

    /// Creates an [`Angle`] from degrees.
    #[inline]
    pub fn degrees(degrees: f32) -> Self {
        Self(degrees.to_radians())
    }

    /// Returns the angle in radians.
    #[inline]
    pub fn as_radians(self) -> f32 {
        self.0
    }

    /// Returns the angle in degrees.
    #[inline]
    pub fn as_degrees(self) -> f32 {
        self.0.to_degrees()
    }

    /// Returns the absolute angle.
    #[inline]
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Returns the minimum of the two angles, ignoring NaN.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Returns the maximum of the two angles, ignoring NaN.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Wraps the angle to be within the `[-2pi, 2pi)` range.
    #[inline]
    #[doc(alias = "normalize")]
    pub fn wrap(self) -> Self {
        Self(self.0 % std::f32::consts::TAU)
    }

    /// Returns the largest integer angle less than or equal to `self`.
    #[inline]
    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }

    /// Returns the smallest integer angle greater than or equal to `self`.
    #[inline]
    pub fn ceil(self) -> Self {
        Self(self.0.ceil())
    }

    /// Returns the nearest integer to `self`. If a value is half-way
    /// between two integers, round away from `0.0`.
    #[inline]
    pub fn round(self) -> Self {
        Self(self.0.round())
    }
}

impl Add for Angle {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Angle {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for Angle {
    type Output = Self;

    fn mul(self, rhs: Angle) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Mul<f32> for Angle {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Angle> for f32 {
    type Output = Angle;

    fn mul(self, rhs: Angle) -> Self::Output {
        Angle::radians(self * rhs.0)
    }
}

impl Div for Angle {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl Div<f32> for Angle {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl AddAssign<Angle> for Angle {
    fn add_assign(&mut self, rhs: Angle) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Angle> for Angle {
    fn sub_assign(&mut self, rhs: Angle) {
        self.0 -= rhs.0;
    }
}

impl MulAssign<Angle> for Angle {
    fn mul_assign(&mut self, rhs: Angle) {
        self.0 *= rhs.0;
    }
}

impl DivAssign<Angle> for Angle {
    fn div_assign(&mut self, rhs: Angle) {
        self.0 /= rhs.0;
    }
}

impl AddAssign<f32> for Angle {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs;
    }
}

impl SubAssign<f32> for Angle {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 -= rhs;
    }
}

impl MulAssign<f32> for Angle {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl DivAssign<f32> for Angle {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}
