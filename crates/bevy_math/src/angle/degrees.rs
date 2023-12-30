use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
};

use crate::{Angle, Radians};

/// An angle in degrees.
///
/// # Example
///
/// ```
/// use std::f32::consts::PI;
/// use bevy_math::{Angle, Degrees, Radians};
///
/// // Create angles from radians or degrees
/// let alpha = Degrees(180.0);
/// let beta = Degrees::from_radians(PI);
/// assert_eq!(alpha, beta);
/// assert_eq!(beta, Radians(PI));
///
/// // Add degrees and radians together (result is always radians)
/// assert_eq!(Radians(PI) + Degrees(180.0), Radians(2.0 * PI));
///
/// // Get float values
/// assert_eq!(alpha.0, 180.0);
/// assert_eq!(alpha.to_radians().0, PI);
///
/// // Use trigonometric operations
/// assert_eq!(alpha.cos(), -1.0);
///
/// // Normalize 540 degrees to range [0, 360) to get 180 degrees
/// let gamma = 3.0 * alpha;
/// let normalized = gamma.normalized();
///
/// // Small threshold for floating point error
/// assert!((normalized - alpha).abs() < Degrees(0.000001));
/// ```
// TODO: Reflect
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Degrees(pub f32);

impl Degrees {
    /// Creates an angle in [`Degrees`] from radians.
    #[inline]
    pub fn from_radians(radians: f32) -> Self {
        Self(radians.to_degrees())
    }

    /// Returns the angle in [`Radians`].
    #[inline]
    pub fn to_radians(self) -> Radians {
        Radians::from_degrees(self.0)
    }
}

impl Angle for Degrees {
    const ZERO: Self = Self(0.0);
    const EIGHTH: Self = Self(45.0);
    const QUARTER: Self = Self(90.0);
    const HALF: Self = Self(180.0);
    const FULL: Self = Self(360.0);

    #[inline]
    fn new(angle: f32) -> Self {
        Self(angle)
    }

    #[inline]
    fn value(self) -> f32 {
        self.0
    }

    #[inline]
    fn sin(self) -> f32 {
        self.to_radians().sin()
    }

    #[inline]
    fn cos(self) -> f32 {
        self.to_radians().cos()
    }

    #[inline]
    fn tan(self) -> f32 {
        self.to_radians().tan()
    }

    #[inline]
    fn sin_cos(self) -> (f32, f32) {
        self.to_radians().sin_cos()
    }
}

impl From<Radians> for Degrees {
    fn from(angle: Radians) -> Self {
        angle.to_degrees()
    }
}

impl Add for Degrees {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Degrees {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for Degrees {
    type Output = Self;

    fn mul(self, rhs: Degrees) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Mul<f32> for Degrees {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Degrees> for f32 {
    type Output = Degrees;

    fn mul(self, rhs: Degrees) -> Self::Output {
        Degrees(self * rhs.0)
    }
}

impl Div for Degrees {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl Div<f32> for Degrees {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Rem for Degrees {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
    }
}

impl Rem<f32> for Degrees {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self::Output {
        Self(self.0 % rhs)
    }
}

impl Neg for Degrees {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl AddAssign for Degrees {
    fn add_assign(&mut self, rhs: Degrees) {
        self.0 += rhs.0;
    }
}

impl AddAssign<f32> for Degrees {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs;
    }
}

impl SubAssign for Degrees {
    fn sub_assign(&mut self, rhs: Degrees) {
        self.0 -= rhs.0;
    }
}

impl SubAssign<f32> for Degrees {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 -= rhs;
    }
}

impl MulAssign for Degrees {
    fn mul_assign(&mut self, rhs: Degrees) {
        self.0 *= rhs.0;
    }
}

impl MulAssign<f32> for Degrees {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl DivAssign for Degrees {
    fn div_assign(&mut self, rhs: Degrees) {
        self.0 /= rhs.0;
    }
}

impl DivAssign<f32> for Degrees {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

impl RemAssign for Degrees {
    fn rem_assign(&mut self, rhs: Degrees) {
        self.0 %= rhs.0;
    }
}

impl RemAssign<f32> for Degrees {
    fn rem_assign(&mut self, rhs: f32) {
        self.0 %= rhs;
    }
}
