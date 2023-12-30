use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

mod degrees;
mod radians;

pub use degrees::Degrees;
pub use radians::Radians;

/// A trait for functionality and constants shared by the [`Degrees`] and [`Radians`] types.
pub trait Angle:
    Sized
    + Clone
    + Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + Neg<Output = Self>
    + PartialEq
    + PartialOrd
{
    /// An angle of zero.
    const ZERO: Self;
    /// A `45` degree angle. Equivalent to `π/4` radians.
    const EIGHTH: Self;
    /// A `90` degree angle (right angle). Equivalent to `π/2` radians.
    #[doc(alias = "RIGHT_ANGLE")]
    const QUARTER: Self;
    /// A `180` degree angle (straight angle). Equivalent to `π` radians.
    #[doc(alias = "STRAIGHT_ANGLE")]
    const HALF: Self;
    /// A `360` degree angle (perigon). Equivalent to `2π` radians.
    #[doc(alias = "FULL_ANGLE")]
    const FULL: Self;

    /// Creates a new angle.
    fn new(angle: f32) -> Self;

    /// Returns the angle value contained within `self`.
    fn value(self) -> f32;

    /// Returns the sine of the angle.
    fn sin(self) -> f32;

    /// Returns the cosine of the angle.
    fn cos(self) -> f32;

    /// Returns the tangent of the angle.
    fn tan(self) -> f32;

    /// Returns the sine and cosine of the angle.
    fn sin_cos(self) -> (f32, f32);

    /// Returns the absolute angle.
    #[inline]
    fn abs(self) -> Self {
        Self::new(self.value().abs())
    }

    /// Returns the minimum of the two angles, ignoring NaN.
    #[inline]
    fn min(self, other: Self) -> Self {
        Self::new(self.value().min(other.value()))
    }

    /// Returns the maximum of the two angles, ignoring NaN.
    #[inline]
    fn max(self, other: Self) -> Self {
        Self::new(self.value().max(other.value()))
    }

    /// Normalizes the angle to be within the range of `[0, 360)` degrees or `[0, 2π)` radians.
    #[inline]
    fn normalized(self) -> Self {
        if self < Self::FULL && self > Self::ZERO {
            self
        } else {
            let remainder = self % Self::FULL;
            if remainder >= Self::ZERO {
                remainder
            } else {
                remainder + Self::FULL
            }
        }
    }

    /// Returns the largest integer angle less than or equal to `self`.
    #[inline]
    fn floor(self) -> Self {
        Self::new(self.value().floor())
    }

    /// Returns the smallest integer angle greater than or equal to `self`.
    #[inline]
    fn ceil(self) -> Self {
        Self::new(self.value().ceil())
    }

    /// Returns the nearest integer to `self`. If a value is half-way
    /// between two integers, round away from `0.0`.
    #[inline]
    fn round(self) -> Self {
        Self::new(self.value().round())
    }

    /// Adds the given `angle` to `self`, returning the inferred type of angle.
    fn add_angle<T: Angle, Output: Angle>(self, angle: T) -> Output
    where
        Output: From<Self> + From<T>,
    {
        Output::from(self) + Output::from(angle)
    }

    /// Subtracts the given `angle` from `self`, returning the inferred type of angle.
    fn sub_angle<T: Angle, Output: Angle>(self, angle: T) -> Output
    where
        Output: From<Self> + From<T>,
    {
        Output::from(self) - Output::from(angle)
    }

    /// Multiplies `self` by the given `angle`, returning the inferred type of angle.
    fn mul_angle<T: Angle, Output: Angle>(self, angle: T) -> Output
    where
        Output: From<Self> + From<T>,
    {
        Output::from(self) * Output::from(angle)
    }

    /// Divides `self` by the given `angle`, returning the inferred type of angle.
    fn div_angle<T: Angle, Output: Angle>(self, angle: T) -> Output
    where
        Output: From<Self> + From<T>,
    {
        Output::from(self) + Output::from(angle)
    }

    /// Computes the remainder of `self` divided by the given `angle`, returning the inferred type of angle.
    fn rem_angle<T: Angle, Output: Angle>(self, angle: T) -> Output
    where
        Output: From<Self> + From<T>,
    {
        Output::from(self) % Output::from(angle)
    }
}

impl PartialEq<Radians> for Degrees {
    fn eq(&self, other: &Radians) -> bool {
        *self == other.to_degrees()
    }
}

impl PartialEq<Degrees> for Radians {
    fn eq(&self, other: &Degrees) -> bool {
        *self == other.to_radians()
    }
}

impl PartialOrd<Radians> for Degrees {
    fn partial_cmp(&self, other: &Radians) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.to_degrees())
    }
}

impl PartialOrd<Degrees> for Radians {
    fn partial_cmp(&self, other: &Degrees) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.to_radians())
    }
}

impl Add<Radians> for Degrees {
    type Output = Radians;

    fn add(self, rhs: Radians) -> Self::Output {
        self.to_radians() + rhs
    }
}

impl Add<Degrees> for Radians {
    type Output = Radians;

    fn add(self, rhs: Degrees) -> Self::Output {
        self + rhs.to_radians()
    }
}

impl Sub<Radians> for Degrees {
    type Output = Radians;

    fn sub(self, rhs: Radians) -> Self::Output {
        self.to_radians() - rhs
    }
}

impl Sub<Degrees> for Radians {
    type Output = Radians;

    fn sub(self, rhs: Degrees) -> Self::Output {
        self - rhs.to_radians()
    }
}

impl Mul<Radians> for Degrees {
    type Output = Radians;

    fn mul(self, rhs: Radians) -> Self::Output {
        self.to_radians() * rhs
    }
}

impl Mul<Degrees> for Radians {
    type Output = Radians;

    fn mul(self, rhs: Degrees) -> Self::Output {
        self * rhs.to_radians()
    }
}

impl Div<Radians> for Degrees {
    type Output = Radians;

    fn div(self, rhs: Radians) -> Self::Output {
        self.to_radians() / rhs
    }
}

impl Div<Degrees> for Radians {
    type Output = Radians;

    fn div(self, rhs: Degrees) -> Self::Output {
        self / rhs.to_radians()
    }
}

impl Rem<Radians> for Degrees {
    type Output = Radians;

    fn rem(self, rhs: Radians) -> Self::Output {
        self.to_radians() % rhs
    }
}

impl Rem<Degrees> for Radians {
    type Output = Radians;

    fn rem(self, rhs: Degrees) -> Self::Output {
        self % rhs.to_radians()
    }
}

impl AddAssign<Radians> for Degrees {
    fn add_assign(&mut self, rhs: Radians) {
        *self += rhs.to_degrees();
    }
}

impl AddAssign<Degrees> for Radians {
    fn add_assign(&mut self, rhs: Degrees) {
        *self += rhs.to_radians();
    }
}

impl SubAssign<Radians> for Degrees {
    fn sub_assign(&mut self, rhs: Radians) {
        *self -= rhs.to_degrees();
    }
}

impl SubAssign<Degrees> for Radians {
    fn sub_assign(&mut self, rhs: Degrees) {
        *self -= rhs.to_radians();
    }
}

impl MulAssign<Radians> for Degrees {
    fn mul_assign(&mut self, rhs: Radians) {
        *self *= rhs.to_degrees();
    }
}

impl MulAssign<Degrees> for Radians {
    fn mul_assign(&mut self, rhs: Degrees) {
        *self *= rhs.to_radians();
    }
}

impl DivAssign<Radians> for Degrees {
    fn div_assign(&mut self, rhs: Radians) {
        *self /= rhs.to_degrees();
    }
}

impl DivAssign<Degrees> for Radians {
    fn div_assign(&mut self, rhs: Degrees) {
        *self /= rhs.to_radians();
    }
}

impl RemAssign<Radians> for Degrees {
    fn rem_assign(&mut self, rhs: Radians) {
        *self %= rhs.to_degrees();
    }
}

impl RemAssign<Degrees> for Radians {
    fn rem_assign(&mut self, rhs: Degrees) {
        *self %= rhs.to_radians();
    }
}
