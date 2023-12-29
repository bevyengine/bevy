//! Contains a generic trait for floating-point numbers.

use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// A trait for generic floats.
pub trait Float:
    Sized
    + Clone
    + Copy
    + Debug
    + Default
    + PartialEq<Self>
    + PartialOrd<Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self>
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
    + DivAssign<Self>
    + Neg<Output = Self>
{
    /// A value of 0.
    const ZERO: Self;
    /// A value of 1.
    const ONE: Self;
    /// A value of -1.
    const NEG_ONE: Self;
    /// 2π
    const TAU: Self;
    /// π (pi)
    const PI: Self;
    /// π/2
    const FRAC_PI_2: Self;
    /// π/3
    const FRAC_PI_3: Self;
    /// π/4
    const FRAC_PI_4: Self;
    /// π/6
    const FRAC_PI_6: Self;
    /// π/8
    const FRAC_PI_8: Self;

    /// Computes the sine of a number (in radians).
    fn sin(self) -> Self;
    /// Computes the cosine of a number (in radians).
    fn cos(self) -> Self;
    /// Computes the tangent of a number (in radians).
    fn tan(self) -> Self;
    /// Simultaneously computes the sine and cosine of the number, `x`. Returns `(sin(x), cos(x))`.
    fn sin_cos(self) -> (Self, Self);
    /// Computes the four quadrant arctangent of `self` (`y`) and `other` (`x`) in radians.
    ///
    /// - `x = 0`, `y = 0`: `0`
    /// - `x >= 0`: `arctan(y/x)` -> `[-pi/2, pi/2]`
    /// - `y >= 0`: `arctan(y/x) + pi` -> `(pi/2, pi]`
    /// - `y < 0`: `arctan(y/x) - pi` -> `(-pi, -pi/2)`
    fn atan2(self, other: Self) -> Self;
    /// Converts radians to degrees.
    fn to_degrees(self) -> Self;
    /// Converts degrees to radians.
    fn to_radians(self) -> Self;
    /// Computes the absolute value of `self`.
    fn abs(self) -> Self;
    /// Returns the minimum of the two numbers, ignoring NaN.
    fn min(self, other: Self) -> Self;
    /// Returns the maximum of the two numbers, ignoring NaN.
    fn max(self, other: Self) -> Self;
}

impl Float for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const NEG_ONE: Self = -1.0;
    const TAU: Self = std::f32::consts::TAU;
    const PI: Self = std::f32::consts::PI;
    const FRAC_PI_2: Self = std::f32::consts::FRAC_PI_2;
    const FRAC_PI_3: Self = std::f32::consts::FRAC_PI_3;
    const FRAC_PI_4: Self = std::f32::consts::FRAC_PI_4;
    const FRAC_PI_6: Self = std::f32::consts::FRAC_PI_6;
    const FRAC_PI_8: Self = std::f32::consts::FRAC_PI_8;

    fn sin(self) -> Self {
        self.sin()
    }
    fn cos(self) -> Self {
        self.cos()
    }
    fn tan(self) -> Self {
        self.tan()
    }
    fn sin_cos(self) -> (Self, Self) {
        self.sin_cos()
    }
    fn atan2(self, other: Self) -> Self {
        self.atan2(other)
    }
    fn to_degrees(self) -> Self {
        self.to_degrees()
    }
    fn to_radians(self) -> Self {
        self.to_radians()
    }
    fn abs(self) -> Self {
        self.abs()
    }
    fn min(self, other: Self) -> Self {
        self.min(other)
    }
    fn max(self, other: Self) -> Self {
        self.max(other)
    }
}

impl Float for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const NEG_ONE: Self = -1.0;
    const TAU: Self = std::f64::consts::TAU;
    const PI: Self = std::f64::consts::PI;
    const FRAC_PI_2: Self = std::f64::consts::FRAC_PI_2;
    const FRAC_PI_3: Self = std::f64::consts::FRAC_PI_3;
    const FRAC_PI_4: Self = std::f64::consts::FRAC_PI_4;
    const FRAC_PI_6: Self = std::f64::consts::FRAC_PI_6;
    const FRAC_PI_8: Self = std::f64::consts::FRAC_PI_8;

    fn sin(self) -> Self {
        self.sin()
    }
    fn cos(self) -> Self {
        self.cos()
    }
    fn tan(self) -> Self {
        self.tan()
    }
    fn sin_cos(self) -> (Self, Self) {
        self.sin_cos()
    }
    fn atan2(self, other: Self) -> Self {
        self.atan2(other)
    }
    fn to_degrees(self) -> Self {
        self.to_degrees()
    }
    fn to_radians(self) -> Self {
        self.to_radians()
    }
    fn abs(self) -> Self {
        self.abs()
    }
    fn min(self, other: Self) -> Self {
        self.min(other)
    }
    fn max(self, other: Self) -> Self {
        self.max(other)
    }
}
