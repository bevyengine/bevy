use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use super::{float::Float, Vec2};
use glam::DVec2;

/// An angle in radians.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Angle<T: Float>(T);

impl<T: Float> Default for Angle<T> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Angle<f32> {
    /// Rotates the given vector by `self`.
    #[inline]
    pub fn rotate(&self, vec: Vec2) -> Vec2 {
        Vec2::new(
            vec.x * self.sin() + vec.y * self.cos(),
            vec.x * self.cos() - vec.y * self.sin(),
        )
    }
}

impl Angle<f64> {
    /// Rotates the given vector by `self`.
    #[inline]
    pub fn rotate(&self, vec: DVec2) -> DVec2 {
        DVec2::new(
            vec.x * self.sin() + vec.y * self.cos(),
            vec.x * self.cos() - vec.y * self.sin(),
        )
    }
}

impl<T: Float> Angle<T> {
    /// An angle of zero.
    pub const ZERO: Self = Self(T::ZERO);
    /// The angle 2π in radians.
    pub const TAU: Self = Self(T::TAU);
    /// The angle π (pi) in radians.
    pub const PI: Self = Self(T::PI);
    /// The angle π/2 in radians.
    pub const FRAC_PI_2: Self = Self(T::FRAC_PI_2);
    /// The angle π/3 in radians.
    pub const FRAC_PI_3: Self = Self(T::FRAC_PI_3);
    /// The angle π/4 in radians.
    pub const FRAC_PI_4: Self = Self(T::FRAC_PI_4);
    /// The angle π/6 in radians.
    pub const FRAC_PI_6: Self = Self(T::FRAC_PI_6);
    /// The angle π/8 in radians.
    pub const FRAC_PI_8: Self = Self(T::FRAC_PI_8);

    /// Returns the sine of the angle in radians.
    #[inline]
    pub fn sin(&self) -> T {
        self.0.sin()
    }

    /// Returns the cosine of the angle in radians.
    #[inline]
    pub fn cos(&self) -> T {
        self.0.cos()
    }

    /// Returns the tangent of the angle in radians.
    #[inline]
    pub fn tan(&self) -> T {
        self.0.tan()
    }

    /// Returns the sine and cosine of the angle in radians.
    #[inline]
    pub fn sin_cos(&self) -> (T, T) {
        self.0.sin_cos()
    }

    /// Creates an [`Angle`] from radians.
    #[inline]
    pub const fn radians(radians: T) -> Self {
        Self(radians)
    }

    /// Creates an [`Angle`] from degrees.
    #[inline]
    pub fn degrees(degrees: T) -> Self {
        Self(degrees.to_radians())
    }

    /// Returns the angle in radians.
    #[inline]
    pub fn as_radians(&self) -> T {
        self.0
    }

    /// Returns the angle in degrees.
    #[inline]
    pub fn as_degrees(&self) -> T {
        self.as_radians().to_degrees()
    }

    /// Returns the absolute angle.
    #[inline]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// Returns the minimum of the two angles, ignoring NaN.
    #[inline]
    pub fn min(&self, other: Self) -> Self {
        Angle::radians(self.0.min(other.0))
    }
}

impl<T: Float> Add for Angle<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<T: Float> Sub for Angle<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl<T: Float> Mul for Angle<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl<T: Float> Div for Angle<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl<T: Float> Mul<T> for Angle<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Angle<f32>> for f32 {
    type Output = Angle<f32>;

    fn mul(self, rhs: Angle<f32>) -> Self::Output {
        Angle::radians(self * rhs.as_radians())
    }
}

impl Mul<Angle<f64>> for f64 {
    type Output = Angle<f64>;

    fn mul(self, rhs: Angle<f64>) -> Self::Output {
        Angle::radians(self * rhs.as_radians())
    }
}

impl<T: Float> Div<T> for Angle<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self::radians(self.as_radians() / rhs)
    }
}

impl Div<Angle<f32>> for f32 {
    type Output = Angle<f32>;

    fn div(self, rhs: Angle<f32>) -> Self::Output {
        Angle::radians(self / rhs.as_radians())
    }
}

impl Div<Angle<f64>> for f64 {
    type Output = Angle<f64>;

    fn div(self, rhs: Angle<f64>) -> Self::Output {
        Angle::radians(self / rhs.as_radians())
    }
}

impl<T: Float> Neg for Angle<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl<T: Float> AddAssign<Angle<T>> for Angle<T> {
    fn add_assign(&mut self, rhs: Angle<T>) {
        self.0 += rhs.0;
    }
}

impl<T: Float> SubAssign<Angle<T>> for Angle<T> {
    fn sub_assign(&mut self, rhs: Angle<T>) {
        self.0 -= rhs.0;
    }
}

impl<T: Float> MulAssign<Angle<T>> for Angle<T> {
    fn mul_assign(&mut self, rhs: Angle<T>) {
        self.0 *= rhs.0;
    }
}

impl<T: Float> DivAssign<Angle<T>> for Angle<T> {
    fn div_assign(&mut self, rhs: Angle<T>) {
        self.0 /= rhs.0;
    }
}

impl<T: Float> AddAssign<T> for Angle<T> {
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs;
    }
}

impl<T: Float> SubAssign<T> for Angle<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs;
    }
}

impl<T: Float> MulAssign<T> for Angle<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.0 *= rhs;
    }
}

impl<T: Float> DivAssign<T> for Angle<T> {
    fn div_assign(&mut self, rhs: T) {
        self.0 /= rhs;
    }
}
