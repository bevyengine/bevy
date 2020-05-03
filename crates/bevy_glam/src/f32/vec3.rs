use super::{Vec2, Vec3Mask, Vec4};
use core::{fmt, ops::*};

#[cfg(all(vec3sse2, target_arch = "x86"))]
use core::arch::x86::*;
#[cfg(all(vec3sse2, target_arch = "x86_64"))]
use core::arch::x86_64::*;

#[cfg(vec3sse2)]
use core::{cmp::Ordering, f32, mem::MaybeUninit};

#[cfg(vec3sse2)]
use crate::{
    f32::{X_AXIS, Y_AXIS, Z_AXIS},
    Align16,
};

/// A 3-dimensional vector.
///
/// This type is 16 byte aligned and thus contains 4 bytes padding.
#[cfg(vec3sse2)]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vec3(pub(crate) __m128);

/// A 3-dimensional vector.
#[cfg(vec3f32)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
// if compiling with simd enabled assume alignment needs to match the simd type
#[cfg_attr(vec3f32_align16, repr(align(16)))]
#[repr(C)]
pub struct Vec3(pub(crate) f32, pub(crate) f32, pub(crate) f32);

#[cfg(vec3sse2)]
impl Vec3 {
    /// Calculates the Vec3 dot product and returns answer in x lane of __m128.
    #[inline]
    unsafe fn dot_as_m128(self, other: Self) -> __m128 {
        let x2_y2_z2_w2 = _mm_mul_ps(self.0, other.0);
        let y2_0_0_0 = _mm_shuffle_ps(x2_y2_z2_w2, x2_y2_z2_w2, 0b00_00_00_01);
        let z2_0_0_0 = _mm_shuffle_ps(x2_y2_z2_w2, x2_y2_z2_w2, 0b00_00_00_10);
        let x2y2_0_0_0 = _mm_add_ss(x2_y2_z2_w2, y2_0_0_0);
        _mm_add_ss(x2y2_0_0_0, z2_0_0_0)
    }
}

#[cfg(vec3sse2)]
impl Default for Vec3 {
    #[inline]
    fn default() -> Self {
        Vec3::zero()
    }
}

#[cfg(vec3sse2)]
impl PartialEq for Vec3 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmpeq(*other).all()
    }
}

#[cfg(vec3sse2)]
impl PartialOrd for Vec3 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

#[cfg(vec3sse2)]
impl From<Vec3> for __m128 {
    // TODO: write test
    #[cfg_attr(tarpaulin, skip)]
    #[inline]
    fn from(t: Vec3) -> Self {
        t.0
    }
}

#[cfg(vec3sse2)]
impl From<__m128> for Vec3 {
    #[inline]
    fn from(t: __m128) -> Self {
        Self(t)
    }
}

#[inline]
pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

impl Vec3 {
    /// Creates a new `Vec3`.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_set_ps(z, z, y, x))
        }

        #[cfg(vec3f32)]
        {
            Self(x, y, z)
        }
    }

    /// Creates a new `Vec3` with all elements set to `0.0`.
    #[inline]
    pub fn zero() -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_setzero_ps())
        }

        #[cfg(vec3f32)]
        {
            Self(0.0, 0.0, 0.0)
        }
    }

    /// Creates a new `Vec3` with all elements set to `1.0`.
    #[inline]
    pub fn one() -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_set1_ps(1.0))
        }

        #[cfg(vec3f32)]
        {
            Self(1.0, 1.0, 1.0)
        }
    }

    /// Creates a new `Vec3` with values `[x: 1.0, y: 0.0, z: 0.0]`.
    #[inline]
    pub fn unit_x() -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_load_ps(X_AXIS.0.as_ptr()))
        }

        #[cfg(vec3f32)]
        {
            Self(1.0, 0.0, 0.0)
        }
    }

    /// Creates a new `Vec3` with values `[x: 0.0, y: 1.0, z: 0.0]`.
    #[inline]
    pub fn unit_y() -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_load_ps(Y_AXIS.0.as_ptr()))
        }

        #[cfg(vec3f32)]
        {
            Self(0.0, 1.0, 0.0)
        }
    }

    /// Creates a new `Vec3` with values `[x: 0.0, y: 0.0, z: 1.0]`.
    #[inline]
    pub fn unit_z() -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_load_ps(Z_AXIS.0.as_ptr()))
        }

        #[cfg(vec3f32)]
        {
            Self(0.0, 0.0, 1.0)
        }
    }

    /// Creates a new `Vec3` with all elements set to `v`.
    #[inline]
    pub fn splat(v: f32) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_set_ps1(v))
        }

        #[cfg(vec3f32)]
        {
            Self(v, v, v)
        }
    }

    /// Creates a new `Vec4` from `self` and the given `w` value.
    #[inline]
    pub fn extend(self, w: f32) -> Vec4 {
        #[cfg(vec3sse2)]
        {
            let mut temp: Vec4 = self.0.into();
            temp.set_w(w);
            temp
        }

        #[cfg(vec3f32)]
        {
            Vec4::new(self.0, self.1, self.2, w)
        }
    }

    /// Creates a `Vec2` from the first three elements of `self`,
    /// removing `z`.
    #[inline]
    pub fn truncate(self) -> Vec2 {
        #[cfg(vec3sse2)]
        {
            let (x, y, _) = self.into();
            Vec2::new(x, y)
        }

        #[cfg(vec3f32)]
        {
            Vec2::new(self.0, self.1)
        }
    }

    /// Returns element `x`.
    #[inline]
    pub fn x(self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            _mm_cvtss_f32(self.0)
        }

        #[cfg(vec3f32)]
        {
            self.0
        }
    }

    /// Returns element `y`.
    #[inline]
    pub fn y(self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b01_01_01_01))
        }

        #[cfg(vec3f32)]
        {
            self.1
        }
    }

    /// Returns element `z`.
    #[inline]
    pub fn z(self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b10_10_10_10))
        }

        #[cfg(vec3f32)]
        {
            self.2
        }
    }

    /// Returns a mutable reference to element `x`.
    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        #[cfg(vec3sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32)
        }

        #[cfg(vec3f32)]
        {
            &mut self.0
        }
    }

    /// Returns a mutable reference to element `y`.
    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        #[cfg(vec3sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32).offset(1)
        }

        #[cfg(vec3f32)]
        {
            &mut self.1
        }
    }

    /// Returns a mutable reference to element `z`.
    #[inline]
    pub fn z_mut(&mut self) -> &mut f32 {
        #[cfg(vec3sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32).offset(2)
        }

        #[cfg(vec3f32)]
        {
            &mut self.2
        }
    }

    /// Sets element `x`.
    #[inline]
    pub fn set_x(&mut self, x: f32) {
        #[cfg(vec3sse2)]
        unsafe {
            self.0 = _mm_move_ss(self.0, _mm_set_ss(x));
        }

        #[cfg(vec3f32)]
        {
            self.0 = x;
        }
    }

    /// Sets element `y`.
    #[inline]
    pub fn set_y(&mut self, y: f32) {
        #[cfg(vec3sse2)]
        unsafe {
            let mut t = _mm_move_ss(self.0, _mm_set_ss(y));
            t = _mm_shuffle_ps(t, t, 0b11_10_00_00);
            self.0 = _mm_move_ss(t, self.0);
        }

        #[cfg(vec3f32)]
        {
            self.1 = y;
        }
    }

    /// Sets element `z`.
    #[inline]
    pub fn set_z(&mut self, z: f32) {
        #[cfg(vec3sse2)]
        unsafe {
            let mut t = _mm_move_ss(self.0, _mm_set_ss(z));
            t = _mm_shuffle_ps(t, t, 0b11_00_01_00);
            self.0 = _mm_move_ss(t, self.0);
        }

        #[cfg(vec3f32)]
        {
            self.2 = z;
        }
    }

    /// Returns a `Vec3` with all elements set to the value of element `x`.
    #[inline]
    pub(crate) fn dup_x(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b00_00_00_00))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0, self.0, self.0)
        }
    }

    /// Returns a `Vec3` with all elements set to the value of element `y`.
    #[inline]
    pub(crate) fn dup_y(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b01_01_01_01))
        }

        #[cfg(vec3f32)]
        {
            Self(self.1, self.1, self.1)
        }
    }

    /// Returns a `Vec3` with all elements set to the value of element `z`.
    #[inline]
    pub(crate) fn dup_z(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b10_10_10_10))
        }

        #[cfg(vec3f32)]
        {
            Self(self.2, self.2, self.2)
        }
    }

    /// Computes the dot product of `self` and `other`.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            _mm_cvtss_f32(self.dot_as_m128(other))
        }

        #[cfg(vec3f32)]
        {
            (self.0 * other.0) + (self.1 * other.1) + (self.2 * other.2)
        }
    }

    /// Returns Vec3 dot in all lanes of Vec3
    #[inline]
    pub(crate) fn dot_as_vec3(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            let dot_in_x = self.dot_as_m128(other);
            Vec3(_mm_shuffle_ps(dot_in_x, dot_in_x, 0b00_00_00_00))
        }

        #[cfg(vec3f32)]
        {
            let dot = self.dot(other);
            Vec3::new(dot, dot, dot)
        }
    }

    /// Computes the cross product of `self` and `other`.
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            // x  <-  a.y*b.z - a.z*b.y
            // y  <-  a.z*b.x - a.x*b.z
            // z  <-  a.x*b.y - a.y*b.x
            // We can save a shuffle by grouping it in this wacky order:
            // (self.zxy() * other - self * other.zxy()).zxy()
            let lhszxy = _mm_shuffle_ps(self.0, self.0, 0b01_01_00_10);
            let rhszxy = _mm_shuffle_ps(other.0, other.0, 0b01_01_00_10);
            let lhszxy_rhs = _mm_mul_ps(lhszxy, other.0);
            let rhszxy_lhs = _mm_mul_ps(rhszxy, self.0);
            let sub = _mm_sub_ps(lhszxy_rhs, rhszxy_lhs);
            Self(_mm_shuffle_ps(sub, sub, 0b01_01_00_10))
        }

        #[cfg(vec3f32)]
        {
            Self(
                self.1 * other.2 - other.1 * self.2,
                self.2 * other.0 - other.2 * self.0,
                self.0 * other.1 - other.0 * self.1,
            )
        }
    }

    /// Computes the length of `self`.
    #[inline]
    pub fn length(self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            _mm_cvtss_f32(_mm_sqrt_ss(self.dot_as_m128(self)))
        }

        #[cfg(vec3f32)]
        {
            self.dot(self).sqrt()
        }
    }

    /// Computes the squared length of `self`.
    ///
    /// This is generally faster than `Vec3::length()` as it avoids a square
    /// root operation.
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes `1.0 / Vec3::length()`.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn length_reciprocal(self) -> f32 {
        #[cfg(vec3sse2)]
        {
            let dot = self.dot_as_vec3(self);
            unsafe {
                // _mm_rsqrt_ps is lower precision
                _mm_cvtss_f32(_mm_div_ps(_mm_set_ps1(1.0), _mm_sqrt_ps(dot.0)))
            }
        }

        #[cfg(vec3f32)]
        {
            1.0 / self.length()
        }
    }

    /// Returns `self` normalized to length 1.0.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn normalize(self) -> Self {
        #[cfg(vec3sse2)]
        {
            let dot = self.dot_as_vec3(self);
            unsafe { Self(_mm_div_ps(self.0, _mm_sqrt_ps(dot.0))) }
        }

        #[cfg(vec3f32)]
        {
            self * self.length_reciprocal()
        }
    }

    /// Returns the vertical minimum of `self` and `other`.
    ///
    /// In other words, this computes
    /// `[x: min(x1, x2), y: min(y1, y2), z: min(z1, z2)]`,
    /// taking the minimum of each element individually.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_min_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Self(
                self.0.min(other.0),
                self.1.min(other.1),
                self.2.min(other.2),
            )
        }
    }

    /// Returns the vertical maximum of `self` and `other`.
    ///
    /// In other words, this computes
    /// `[x: max(x1, x2), y: max(y1, y2), z: max(z1, z2)]`,
    /// taking the maximum of each element individually.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_max_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Self(
                self.0.max(other.0),
                self.1.max(other.1),
                self.2.max(other.2),
            )
        }
    }

    /// Returns the horizontal minimum of `self`'s elements.
    ///
    /// In other words, this computes `min(x, y, z)`.
    #[inline]
    pub fn min_element(self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            let v = self.0;
            let v = _mm_min_ps(v, _mm_shuffle_ps(v, v, 0b01_01_10_10));
            let v = _mm_min_ps(v, _mm_shuffle_ps(v, v, 0b00_00_00_01));
            _mm_cvtss_f32(v)
        }

        #[cfg(vec3f32)]
        {
            self.0.min(self.1.min(self.2))
        }
    }

    /// Returns the horizontal maximum of `self`'s elements.
    ///
    /// In other words, this computes `max(x, y, z)`.
    #[inline]
    pub fn max_element(self) -> f32 {
        #[cfg(vec3sse2)]
        unsafe {
            let v = self.0;
            let v = _mm_max_ps(v, _mm_shuffle_ps(v, v, 0b00_00_10_10));
            let v = _mm_max_ps(v, _mm_shuffle_ps(v, v, 0b00_00_00_01));
            _mm_cvtss_f32(v)
        }

        #[cfg(vec3f32)]
        {
            self.0.max(self.1.max(self.2))
        }
    }

    /// Performs a vertical `==` comparison between `self` and `other`,
    /// returning a `Vec3Mask` of the results.
    ///
    /// In other words, this computes `[x1 == x2, y1 == y2, z1 == z2, w1 == w2]`.
    #[inline]
    pub fn cmpeq(self, other: Self) -> Vec3Mask {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3Mask(_mm_cmpeq_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3Mask::new(
                self.0.eq(&other.0),
                self.1.eq(&other.1),
                self.2.eq(&other.2),
            )
        }
    }

    /// Performs a vertical `!=` comparison between `self` and `other`,
    /// returning a `Vec3Mask` of the results.
    ///
    /// In other words, this computes `[x1 != x2, y1 != y2, z1 != z2, w1 != w2]`.
    #[inline]
    pub fn cmpne(self, other: Self) -> Vec3Mask {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3Mask(_mm_cmpneq_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3Mask::new(
                self.0.ne(&other.0),
                self.1.ne(&other.1),
                self.2.ne(&other.2),
            )
        }
    }

    /// Performs a vertical `>=` comparison between `self` and `other`,
    /// returning a `Vec3Mask` of the results.
    ///
    /// In other words, this computes `[x1 >= x2, y1 >= y2, z1 >= z2, w1 >= w2]`.
    #[inline]
    pub fn cmpge(self, other: Self) -> Vec3Mask {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3Mask(_mm_cmpge_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3Mask::new(
                self.0.ge(&other.0),
                self.1.ge(&other.1),
                self.2.ge(&other.2),
            )
        }
    }

    /// Performs a vertical `>` comparison between `self` and `other`,
    /// returning a `Vec3Mask` of the results.
    ///
    /// In other words, this computes `[x1 > x2, y1 > y2, z1 > z2, w1 > w2]`.
    #[inline]
    pub fn cmpgt(self, other: Self) -> Vec3Mask {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3Mask(_mm_cmpgt_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3Mask::new(
                self.0.gt(&other.0),
                self.1.gt(&other.1),
                self.2.gt(&other.2),
            )
        }
    }

    /// Performs a vertical `<=` comparison between `self` and `other`,
    /// returning a `Vec3Mask` of the results.
    ///
    /// In other words, this computes `[x1 <= x2, y1 <= y2, z1 <= z2, w1 <= w2]`.
    #[inline]
    pub fn cmple(self, other: Self) -> Vec3Mask {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3Mask(_mm_cmple_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3Mask::new(
                self.0.le(&other.0),
                self.1.le(&other.1),
                self.2.le(&other.2),
            )
        }
    }

    /// Performs a vertical `<` comparison between `self` and `other`,
    /// returning a `Vec3Mask` of the results.
    ///
    /// In other words, this computes `[x1 < x2, y1 < y2, z1 < z2, w1 < w2]`.
    #[inline]
    pub fn cmplt(self, other: Self) -> Vec3Mask {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3Mask(_mm_cmplt_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3Mask::new(
                self.0.lt(&other.0),
                self.1.lt(&other.1),
                self.2.lt(&other.2),
            )
        }
    }

    /// Creates a new `Vec3` from the first four values in `slice`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is less than three elements long.
    #[inline]
    pub fn from_slice_unaligned(slice: &[f32]) -> Self {
        Self::new(slice[0], slice[1], slice[2])
    }

    /// Writes the elements of `self` to the first three elements in `slice`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is less than three elements long.
    #[inline]
    pub fn write_to_slice_unaligned(self, slice: &mut [f32]) {
        let a = self.as_ref();
        slice[0] = a[0];
        slice[1] = a[1];
        slice[2] = a[2];
    }

    /// Per element multiplication/addition of the three inputs: b + (self * a)
    #[inline]
    pub(crate) fn mul_add(self, a: Self, b: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_add_ps(_mm_mul_ps(self.0, a.0), b.0))
        }

        #[cfg(vec3f32)]
        {
            Self(
                (self.0 * a.0) + b.0,
                (self.1 * a.1) + b.1,
                (self.2 * a.2) + b.2,
            )
        }
    }

    /// Returns a new `Vec3` containing the absolute value of each element of the original
    /// `Vec3`.
    #[inline]
    pub fn abs(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_and_ps(
                self.0,
                _mm_castsi128_ps(_mm_set1_epi32(0x7f_ff_ff_ff)),
            ))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0.abs(), self.1.abs(), self.2.abs())
        }
    }

    #[inline]
    pub fn round(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            use crate::f32::funcs::sse2::m128_round;
            Self(m128_round(self.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0.round(), self.1.round(), self.2.round())
        }
    }

    #[inline]
    pub fn floor(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            use crate::f32::funcs::sse2::m128_floor;
            Self(m128_floor(self.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0.floor(), self.1.floor(), self.2.floor())
        }
    }

    #[inline]
    pub fn ceil(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            use crate::f32::funcs::sse2::m128_ceil;
            Self(m128_ceil(self.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0.ceil(), self.1.ceil(), self.2.ceil())
        }
    }

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
    /// results in a new `Vec3`.
    #[inline]
    pub fn reciprocal(self) -> Self {
        // TODO: Optimize
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

    /// Returns whether `self` of length `1.0` or not.
    ///
    /// Uses a precision threshold of `1e-6`.
    #[inline]
    pub fn is_normalized(self) -> bool {
        is_normalized!(self)
    }

    /// Returns true if the absolute difference of all elements between `self`
    /// and `other` is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two `Vec3`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(self, other: Self, max_abs_diff: f32) -> bool {
        abs_diff_eq!(self, other, max_abs_diff)
    }

    /// Returns the angle between two vectors, in radians.
    ///
    /// The vectors do not need to be unit length, but this function does
    /// perform a `sqrt`.
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        crate::f32::funcs::scalar_acos(self.dot(other) / (self.dot(self) * other.dot(other)).sqrt())
    }
}

impl AsRef<[f32; 3]> for Vec3 {
    #[inline]
    fn as_ref(&self) -> &[f32; 3] {
        unsafe { &*(self as *const Vec3 as *const [f32; 3]) }
    }
}

impl AsMut<[f32; 3]> for Vec3 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 3] {
        unsafe { &mut *(self as *mut Vec3 as *mut [f32; 3]) }
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(vec3sse2)]
        {
            let (x, y, z) = (*self).into();
            write!(f, "[{}, {}, {}]", x, y, z)
        }

        #[cfg(vec3f32)]
        {
            write!(f, "[{}, {}, {}]", self.0, self.1, self.2)
        }
    }
}

impl Div<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_div_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0 / other.0, self.1 / other.1, self.2 / other.2)
        }
    }
}

impl DivAssign<Vec3> for Vec3 {
    #[inline]
    fn div_assign(&mut self, other: Self) {
        #[cfg(vec3sse2)]
        {
            self.0 = unsafe { _mm_div_ps(self.0, other.0) };
        }

        #[cfg(vec3f32)]
        {
            self.0 /= other.0;
            self.1 /= other.1;
            self.2 /= other.2;
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, other: f32) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_div_ps(self.0, _mm_set1_ps(other)))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0 / other, self.1 / other, self.2 / other)
        }
    }
}

impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, other: f32) {
        #[cfg(vec3sse2)]
        {
            self.0 = unsafe { _mm_div_ps(self.0, _mm_set1_ps(other)) };
        }

        #[cfg(vec3f32)]
        {
            self.0 /= other;
            self.1 /= other;
            self.2 /= other;
        }
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_mul_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0 * other.0, self.1 * other.1, self.2 * other.2)
        }
    }
}

impl MulAssign<Vec3> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        #[cfg(vec3sse2)]
        {
            self.0 = unsafe { _mm_mul_ps(self.0, other.0) };
        }

        #[cfg(vec3f32)]
        {
            self.0 *= other.0;
            self.1 *= other.1;
            self.2 *= other.2;
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, other: f32) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_mul_ps(self.0, _mm_set1_ps(other)))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0 * other, self.1 * other, self.2 * other)
        }
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, other: f32) {
        #[cfg(vec3sse2)]
        {
            self.0 = unsafe { _mm_mul_ps(self.0, _mm_set1_ps(other)) };
        }

        #[cfg(vec3f32)]
        {
            self.0 *= other;
            self.1 *= other;
            self.2 *= other;
        }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, other: Vec3) -> Vec3 {
        #[cfg(vec3sse2)]
        unsafe {
            Vec3(_mm_mul_ps(_mm_set1_ps(self), other.0))
        }

        #[cfg(vec3f32)]
        {
            Vec3(self * other.0, self * other.1, self * other.2)
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_add_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0 + other.0, self.1 + other.1, self.2 + other.2)
        }
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        #[cfg(vec3sse2)]
        {
            self.0 = unsafe { _mm_add_ps(self.0, other.0) };
        }

        #[cfg(vec3f32)]
        {
            self.0 += other.0;
            self.1 += other.1;
            self.2 += other.2;
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_sub_ps(self.0, other.0))
        }

        #[cfg(vec3f32)]
        {
            Self(self.0 - other.0, self.1 - other.1, self.2 - other.2)
        }
    }
}

impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        #[cfg(vec3sse2)]
        {
            self.0 = unsafe { _mm_sub_ps(self.0, other.0) };
        }

        #[cfg(vec3f32)]
        {
            self.0 -= other.0;
            self.1 -= other.1;
            self.2 -= other.2;
        }
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        #[cfg(vec3sse2)]
        unsafe {
            Self(_mm_sub_ps(_mm_set1_ps(0.0), self.0))
        }

        #[cfg(vec3f32)]
        {
            Self(-self.0, -self.1, -self.2)
        }
    }
}

impl Index<usize> for Vec3 {
    type Output = f32;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl IndexMut<usize> for Vec3 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl From<(f32, f32, f32)> for Vec3 {
    #[inline]
    fn from(t: (f32, f32, f32)) -> Self {
        Self::new(t.0, t.1, t.2)
    }
}

impl From<Vec3> for (f32, f32, f32) {
    #[inline]
    fn from(v: Vec3) -> Self {
        #[cfg(vec3sse2)]
        {
            let mut out: MaybeUninit<Align16<(f32, f32, f32)>> = MaybeUninit::uninit();
            unsafe {
                // out is 16 bytes in size due to alignment
                _mm_store_ps(out.as_mut_ptr() as *mut f32, v.0);
                out.assume_init().0
            }
        }

        #[cfg(vec3f32)]
        {
            (v.0, v.1, v.2)
        }
    }
}

impl From<[f32; 3]> for Vec3 {
    #[inline]
    fn from(a: [f32; 3]) -> Self {
        Self::new(a[0], a[1], a[2])
    }
}

impl From<Vec3> for [f32; 3] {
    #[inline]
    fn from(v: Vec3) -> Self {
        #[cfg(vec3sse2)]
        {
            let mut out: MaybeUninit<Align16<[f32; 3]>> = MaybeUninit::uninit();
            unsafe {
                // out is 16 bytes in size due to alignment
                _mm_store_ps(out.as_mut_ptr() as *mut f32, v.0);
                out.assume_init().0
            }
        }

        #[cfg(vec3f32)]
        {
            [v.0, v.1, v.2]
        }
    }
}

#[test]
fn test_vec3_private() {
    assert_eq!(
        vec3(1.0, 1.0, 1.0).mul_add(vec3(0.5, 2.0, -4.0), vec3(-1.0, -1.0, -1.0)),
        vec3(-0.5, 1.0, -5.0)
    );
    assert_eq!(vec3(1.0, 2.0, 3.0).dup_x(), vec3(1.0, 1.0, 1.0));
    assert_eq!(vec3(1.0, 2.0, 3.0).dup_y(), vec3(2.0, 2.0, 2.0));
    assert_eq!(vec3(1.0, 2.0, 3.0).dup_z(), vec3(3.0, 3.0, 3.0));
}
