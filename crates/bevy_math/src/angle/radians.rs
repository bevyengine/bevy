use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
};

use crate::{Angle, Degrees};

/// An angle in radians.
///
/// # Example
///
/// ```
/// use std::f32::consts::PI;
/// use bevy_math::{Angle, Degrees, Radians};
///
/// // Create angles from radians or degrees
/// let alpha = Radians(PI);
/// let beta = Radians::from_degrees(180.0);
/// assert_eq!(alpha, beta);
/// assert_eq!(beta, Degrees(180.0));
///
/// // Add degrees and radians together (result is always radians)
/// assert_eq!(Radians(PI) + Degrees(180.0), Radians(2.0 * PI));
///
/// // Get float values
/// assert_eq!(alpha.0, PI);
/// assert_eq!(alpha.to_degrees().0, 180.0);
///
/// // Use trigonometric operations
/// assert_eq!(alpha.cos(), -1.0);
///
/// // Normalize 3π to range [0, 2π) to get π
/// let gamma = 3.0 * alpha;
/// let normalized = gamma.normalized();
///
/// // Small threshold for floating point error
/// assert!((normalized - alpha).abs() < Radians(0.000001));
/// ```
// TODO: Reflect
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Radians(pub f32);

impl Radians {
    /// The angle 2π. Equivalent to `Radians::FULL`.
    #[doc(alias = "TWO_PI")]
    pub const TAU: Self = Self(std::f32::consts::TAU);
    /// The angle π (π). Equivalent to `Radians::HALF`.
    pub const PI: Self = Self(std::f32::consts::PI);
    /// The angle π/2. Equivalent to `Radians::QUARTER`.
    #[doc(alias = "HALF_PI")]
    pub const FRAC_PI_2: Self = Self(std::f32::consts::FRAC_PI_2);
    /// The angle π/3.
    pub const FRAC_PI_3: Self = Self(std::f32::consts::FRAC_PI_3);
    /// The angle π/4. Equivalent to `Radians::EIGHTH`.
    pub const FRAC_PI_4: Self = Self(std::f32::consts::FRAC_PI_4);
    /// The angle π/6.
    pub const FRAC_PI_6: Self = Self(std::f32::consts::FRAC_PI_6);
    /// The angle π/8.
    pub const FRAC_PI_8: Self = Self(std::f32::consts::FRAC_PI_8);

    /// Creates an angle in [`Radians`] from degrees.
    #[inline]
    pub fn from_degrees(degrees: f32) -> Self {
        Self(degrees.to_radians())
    }

    /// Returns the angle in [`Degrees`].
    #[inline]
    pub fn to_degrees(self) -> Degrees {
        Degrees::from_radians(self.0)
    }
}

impl Angle for Radians {
    const ZERO: Self = Self(0.0);
    const EIGHTH: Self = Self::FRAC_PI_4;
    const QUARTER: Self = Self::FRAC_PI_2;
    const HALF: Self = Self::PI;
    const FULL: Self = Self::TAU;

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
        self.0.sin()
    }

    #[inline]
    fn cos(self) -> f32 {
        self.0.cos()
    }

    #[inline]
    fn tan(self) -> f32 {
        self.0.tan()
    }

    #[inline]
    fn sin_cos(self) -> (f32, f32) {
        self.0.sin_cos()
    }
}

impl From<Degrees> for Radians {
    fn from(angle: Degrees) -> Self {
        angle.to_radians()
    }
}

impl Add for Radians {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Radians {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<f32> for Radians {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Radians> for f32 {
    type Output = Radians;

    fn mul(self, rhs: Radians) -> Self::Output {
        Radians(self * rhs.0)
    }
}

impl Div for Radians {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Div<f32> for Radians {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Rem for Radians {
    type Output = f32;

    fn rem(self, rhs: Self) -> Self::Output {
        self.0 % rhs.0
    }
}

impl Rem<f32> for Radians {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self::Output {
        Self(self.0 % rhs)
    }
}

impl Neg for Radians {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl AddAssign for Radians {
    fn add_assign(&mut self, rhs: Radians) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Radians {
    fn sub_assign(&mut self, rhs: Radians) {
        self.0 -= rhs.0;
    }
}

impl MulAssign<f32> for Radians {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl DivAssign<f32> for Radians {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

impl RemAssign<f32> for Radians {
    fn rem_assign(&mut self, rhs: f32) {
        self.0 %= rhs;
    }
}
