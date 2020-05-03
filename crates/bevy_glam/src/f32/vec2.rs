use crate::f32::{Vec2Mask, Vec3};
use core::{f32, fmt, ops::*};

/// A 2-dimensional vector.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
#[repr(C)]
pub struct Vec2(pub(crate) f32, pub(crate) f32);

#[inline]
pub fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2(x, y)
}

impl Vec2 {
    /// Returns a new `Vec4` with elements representing the sign of `self`.
    ///
    /// - `1.0` if the number is positive, `+0.0` or `INFINITY`
    /// - `-1.0` if the number is negative, `-0.0` or `NEG_INFINITY`
    #[inline]
    pub fn sign(self) -> Self {
        let mask = self.cmpge(Self::zero());
        mask.select(Self::splat(1.0), Self::splat(-1.0))
    }

    /// Computes the reciprocal `1.0/n` of each element, returning the
    /// results in a new `Vec2`.
    #[inline]
    pub fn reciprocal(self) -> Self {
        Self::one() / self
    }

    /// Performs a linear interpolation between `self` and `other` based on
    /// the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `other`.
    #[inline]
    pub fn lerp(self, other: Self, s: f32) -> Self {
        self + ((other - self) * s)
    }

    /// Returns whether `self` is length `1.0` or not.
    ///
    /// Uses a precision threshold of `1e-6`.
    #[inline]
    pub fn is_normalized(self) -> bool {
        is_normalized!(self)
    }

    /// Returns true if the absolute difference of all elements between `self`
    /// and `other` is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two `Vec2`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(self, other: Self, max_abs_diff: f32) -> bool {
        abs_diff_eq!(self, other, max_abs_diff)
    }

    /// Creates a new `Vec2`.
    #[inline]
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2(x, y)
    }

    /// Creates a new `Vec2` with all elements set to `0.0`.
    #[inline]
    pub fn zero() -> Vec2 {
        Vec2(0.0, 0.0)
    }

    /// Creates a new `Vec2` with all elements set to `1.0`.
    #[inline]
    pub fn one() -> Vec2 {
        Vec2(1.0, 1.0)
    }

    /// Creates a new `Vec2` with values `[x: 1.0, y: 0.0]`.
    #[inline]
    pub fn unit_x() -> Vec2 {
        Vec2(1.0, 0.0)
    }

    /// Creates a new `Vec2` with values `[x: 0.0, y: 1.0]`.
    #[inline]
    pub fn unit_y() -> Vec2 {
        Vec2(0.0, 1.0)
    }

    /// Creates a new `Vec2` with all elements set to `v`.
    #[inline]
    pub fn splat(v: f32) -> Vec2 {
        Vec2(v, v)
    }

    /// Creates a new `Vec3` from `self` and the given `z` value.
    #[inline]
    pub fn extend(self, z: f32) -> Vec3 {
        Vec3::new(self.0, self.1, z)
    }

    /// Returns element `x`.
    #[inline]
    pub fn x(self) -> f32 {
        self.0
    }

    /// Returns element `y`.
    #[inline]
    pub fn y(self) -> f32 {
        self.1
    }

    /// Returns a mutable reference to element `x`.
    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        &mut self.0
    }

    /// Returns a mutable reference to element `y`.
    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        &mut self.1
    }

    /// Sets element `x`.
    #[inline]
    pub fn set_x(&mut self, x: f32) {
        self.0 = x;
    }

    /// Sets element `y`.
    #[inline]
    pub fn set_y(&mut self, y: f32) {
        self.1 = y;
    }

    /// Computes the dot product of `self` and `other`.
    #[inline]
    pub fn dot(self, other: Vec2) -> f32 {
        (self.0 * other.0) + (self.1 * other.1)
    }

    /// Computes the length of `self`.
    #[inline]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Computes the squared length of `self`.
    ///
    /// This is generally faster than `Vec2::length()` as it avoids a square
    /// root operation.
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes `1.0 / Vec2::length()`.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn length_reciprocal(self) -> f32 {
        1.0 / self.length()
    }

    /// Returns `self` normalized to length 1.0.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn normalize(self) -> Vec2 {
        self * self.length_reciprocal()
    }

    /// Returns the vertical minimum of `self` and `other`.
    ///
    /// In other words, this computes
    /// `[x: min(x1, x2), y: min(y1, y2)]`,
    /// taking the minimum of each element individually.
    #[inline]
    pub fn min(self, other: Vec2) -> Vec2 {
        Vec2(self.0.min(other.0), self.1.min(other.1))
    }

    /// Returns the vertical maximum of `self` and `other`.
    ///
    /// In other words, this computes
    /// `[x: max(x1, x2), y: max(y1, y2)]`,
    /// taking the maximum of each element individually.
    #[inline]
    pub fn max(self, other: Vec2) -> Vec2 {
        Vec2(self.0.max(other.0), self.1.max(other.1))
    }

    /// Returns the horizontal minimum of `self`'s elements.
    ///
    /// In other words, this computes `min(x, y)`.
    #[inline]
    pub fn min_element(self) -> f32 {
        self.0.min(self.1)
    }

    /// Returns the horizontal maximum of `self`'s elements.
    ///
    /// In other words, this computes `max(x, y)`.
    #[inline]
    pub fn max_element(self) -> f32 {
        self.0.max(self.1)
    }

    /// Performs a vertical `==` comparison between `self` and `other`,
    /// returning a `Vec2Mask` of the results.
    ///
    /// In other words, this computes `[x1 == x2, y1 == y2, z1 == z2, w1 == w2]`.
    #[inline]
    pub fn cmpeq(self, other: Vec2) -> Vec2Mask {
        Vec2Mask::new(self.0.eq(&other.0), self.1.eq(&other.1))
    }

    /// Performs a vertical `!=` comparison between `self` and `other`,
    /// returning a `Vec2Mask` of the results.
    ///
    /// In other words, this computes `[x1 != x2, y1 != y2, z1 != z2, w1 != w2]`.
    #[inline]
    pub fn cmpne(self, other: Vec2) -> Vec2Mask {
        Vec2Mask::new(self.0.ne(&other.0), self.1.ne(&other.1))
    }

    /// Performs a vertical `>=` comparison between `self` and `other`,
    /// returning a `Vec2Mask` of the results.
    ///
    /// In other words, this computes `[x1 >= x2, y1 >= y2, z1 >= z2, w1 >= w2]`.
    #[inline]
    pub fn cmpge(self, other: Vec2) -> Vec2Mask {
        Vec2Mask::new(self.0.ge(&other.0), self.1.ge(&other.1))
    }

    /// Performs a vertical `>` comparison between `self` and `other`,
    /// returning a `Vec2Mask` of the results.
    ///
    /// In other words, this computes `[x1 > x2, y1 > y2, z1 > z2, w1 > w2]`.
    #[inline]
    pub fn cmpgt(self, other: Vec2) -> Vec2Mask {
        Vec2Mask::new(self.0.gt(&other.0), self.1.gt(&other.1))
    }

    /// Performs a vertical `<=` comparison between `self` and `other`,
    /// returning a `Vec2Mask` of the results.
    ///
    /// In other words, this computes `[x1 <= x2, y1 <= y2, z1 <= z2, w1 <= w2]`.
    #[inline]
    pub fn cmple(self, other: Vec2) -> Vec2Mask {
        Vec2Mask::new(self.0.le(&other.0), self.1.le(&other.1))
    }

    /// Performs a vertical `<` comparison between `self` and `other`,
    /// returning a `Vec2Mask` of the results.
    ///
    /// In other words, this computes `[x1 < x2, y1 < y2, z1 < z2, w1 < w2]`.
    #[inline]
    pub fn cmplt(self, other: Vec2) -> Vec2Mask {
        Vec2Mask::new(self.0.lt(&other.0), self.1.lt(&other.1))
    }

    /// Creates a new `Vec2` from the first two values in `slice`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is less than two elements long.
    #[inline]
    pub fn from_slice_unaligned(slice: &[f32]) -> Self {
        Self(slice[0], slice[1])
    }

    /// Writes the elements of `self` to the first two elements in `slice`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is less than two elements long.
    #[inline]
    pub fn write_to_slice_unaligned(self, slice: &mut [f32]) {
        slice[0] = self.0;
        slice[1] = self.1;
    }

    /// Returns a new `Vec2` containing the absolute value of each element of the original
    /// `Vec2`.
    #[inline]
    pub fn abs(self) -> Self {
        Self(self.0.abs(), self.1.abs())
    }

    #[inline]
    pub fn round(self) -> Self {
        Self(self.0.round(), self.1.round())
    }

    #[inline]
    pub fn floor(self) -> Self {
        Self(self.0.floor(), self.1.floor())
    }

    #[inline]
    pub fn ceil(self) -> Self {
        Self(self.0.ceil(), self.1.ceil())
    }

    /// The perpendicular dot product of the vector and `other`.
    #[inline]
    pub fn perp_dot(self, other: Vec2) -> f32 {
        (self.0 * other.1) - (self.1 * other.0)
    }

    /// Returns the angle between two vectors, in radians.
    ///
    /// The vectors do not need to be unit length, but this function does
    /// perform a `sqrt`.
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        let angle = crate::f32::funcs::scalar_acos(
            self.dot(other) / (self.dot(self) * other.dot(other)).sqrt(),
        );

        if self.perp_dot(other) < 0.0 {
            -angle
        } else {
            angle
        }
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}, {}]", self.0, self.1)
    }
}

impl Div<Vec2> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, other: Vec2) -> Self {
        Self(self.0 / other.0, self.1 / other.1)
    }
}

impl DivAssign<Vec2> for Vec2 {
    #[inline]
    fn div_assign(&mut self, other: Vec2) {
        self.0 /= other.0;
        self.1 /= other.1;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, other: f32) -> Self {
        Self(self.0 / other, self.1 / other)
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, other: f32) {
        self.0 /= other;
        self.1 /= other;
    }
}

impl Mul<Vec2> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Vec2) -> Self {
        Self(self.0 * other.0, self.1 * other.1)
    }
}

impl MulAssign<Vec2> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, other: Vec2) {
        self.0 *= other.0;
        self.1 *= other.1;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, other: f32) -> Self {
        Self(self.0 * other, self.1 * other)
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, other: f32) {
        self.0 *= other;
        self.1 *= other;
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, other: Vec2) -> Vec2 {
        Vec2(self * other.0, self * other.1)
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
        self.1 += other.1;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Vec2) -> Self {
        Self(self.0 - other.0, self.1 - other.1)
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, other: Vec2) {
        self.0 -= other.0;
        self.1 -= other.1;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(-self.0, -self.1)
    }
}

impl AsRef<[f32; 2]> for Vec2 {
    #[inline]
    fn as_ref(&self) -> &[f32; 2] {
        unsafe { &*(self as *const Vec2 as *const [f32; 2]) }
    }
}

impl AsMut<[f32; 2]> for Vec2 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 2] {
        unsafe { &mut *(self as *mut Vec2 as *mut [f32; 2]) }
    }
}

impl Index<usize> for Vec2 {
    type Output = f32;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl IndexMut<usize> for Vec2 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl From<(f32, f32)> for Vec2 {
    #[inline]
    fn from(t: (f32, f32)) -> Self {
        Self(t.0, t.1)
    }
}

impl From<Vec2> for (f32, f32) {
    #[inline]
    fn from(v: Vec2) -> Self {
        (v.0, v.1)
    }
}

impl From<[f32; 2]> for Vec2 {
    #[inline]
    fn from(a: [f32; 2]) -> Self {
        Self(a[0], a[1])
    }
}

impl From<Vec2> for [f32; 2] {
    #[inline]
    fn from(v: Vec2) -> Self {
        [v.0, v.1]
    }
}
