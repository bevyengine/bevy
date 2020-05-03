use super::{Vec3, Vec4Mask};
use core::{fmt, ops::*};

#[cfg(all(vec4sse2, target_arch = "x86"))]
use core::arch::x86::*;
#[cfg(all(vec4sse2, target_arch = "x86_64"))]
use core::arch::x86_64::*;

#[cfg(vec4sse2)]
use crate::Align16;
#[cfg(vec4sse2)]
use core::{cmp::Ordering, f32, mem::MaybeUninit};

#[cfg(vec4sse2)]
pub(crate) const X_AXIS: Align16<[f32; 4]> = Align16([1.0, 0.0, 0.0, 0.0]);
#[cfg(vec4sse2)]
pub(crate) const Y_AXIS: Align16<[f32; 4]> = Align16([0.0, 1.0, 0.0, 0.0]);
#[cfg(vec4sse2)]
pub(crate) const Z_AXIS: Align16<[f32; 4]> = Align16([0.0, 0.0, 1.0, 0.0]);
#[cfg(vec4sse2)]
pub(crate) const W_AXIS: Align16<[f32; 4]> = Align16([0.0, 0.0, 0.0, 1.0]);

/// A 4-dimensional vector.
///
/// This type is 16 byte aligned.
#[cfg(vec4sse2)]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vec4(pub(crate) __m128);

/// A 4-dimensional vector.
///
/// This type is 16 byte aligned unless the `scalar-math` feature is enabed.
#[cfg(vec4f32)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
// if compiling with simd enabled assume alignment needs to match the simd type
#[cfg_attr(vec4f32_align16, repr(align(16)))]
#[repr(C)]
pub struct Vec4(
    pub(crate) f32,
    pub(crate) f32,
    pub(crate) f32,
    pub(crate) f32,
);

#[cfg(vec4sse2)]
impl Default for Vec4 {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(vec4sse2)]
impl PartialEq for Vec4 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmpeq(*other).all()
    }
}

#[cfg(vec4sse2)]
impl PartialOrd for Vec4 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

#[cfg(vec4sse2)]
impl From<Vec4> for __m128 {
    // TODO: write test
    #[cfg_attr(tarpaulin, skip)]
    #[inline]
    fn from(t: Vec4) -> Self {
        t.0
    }
}

#[cfg(vec4sse2)]
impl From<__m128> for Vec4 {
    #[inline]
    fn from(t: __m128) -> Self {
        Self(t)
    }
}

#[inline]
pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
    Vec4::new(x, y, z, w)
}

impl Vec4 {
    /// Creates a new `Vec4`.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_set_ps(w, z, y, x))
        }
        #[cfg(vec4f32)]
        {
            Self(x, y, z, w)
        }
    }

    /// Creates a new `Vec4` with all elements set to `0.0`.
    #[inline]
    pub fn zero() -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_setzero_ps())
        }

        #[cfg(vec4f32)]
        {
            Self(0.0, 0.0, 0.0, 0.0)
        }
    }

    /// Creates a new `Vec4` with all elements set to `1.0`.
    #[inline]
    pub fn one() -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_set1_ps(1.0))
        }

        #[cfg(vec4f32)]
        {
            Self(1.0, 1.0, 1.0, 1.0)
        }
    }

    /// Creates a new `Vec4` with values `[x: 1.0, y: 0.0, z: 0.0, w: 0.0]`.
    #[inline]
    pub fn unit_x() -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_load_ps(X_AXIS.0.as_ptr()))
        }

        #[cfg(vec4f32)]
        {
            Self(1.0, 0.0, 0.0, 0.0)
        }
    }

    /// Creates a new `Vec4` with values `[x: 0.0, y: 1.0, z: 0.0, w: 0.0]`.
    #[inline]
    pub fn unit_y() -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_load_ps(Y_AXIS.0.as_ptr()))
        }

        #[cfg(vec4f32)]
        {
            Self(0.0, 1.0, 0.0, 0.0)
        }
    }

    /// Creates a new `Vec4` with values `[x: 0.0, y: 0.0, z: 1.0, w: 0.0]`.
    #[inline]
    pub fn unit_z() -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_load_ps(Z_AXIS.0.as_ptr()))
        }

        #[cfg(vec4f32)]
        {
            Self(0.0, 0.0, 1.0, 0.0)
        }
    }

    /// Creates a new `Vec4` with values `[x: 0.0, y: 0.0, z: 0.0, w: 1.0]`.
    #[inline]
    pub fn unit_w() -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_load_ps(W_AXIS.0.as_ptr()))
        }

        #[cfg(vec4f32)]
        {
            Self(0.0, 0.0, 0.0, 1.0)
        }
    }

    /// Creates a new `Vec4` with all elements set to `v`.
    #[inline]
    pub fn splat(v: f32) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_set_ps1(v))
        }

        #[cfg(vec4f32)]
        {
            Self(v, v, v, v)
        }
    }

    /// Creates a `Vec3` from the first three elements of `self`,
    /// removing `w`.
    #[inline]
    pub fn truncate(self) -> Vec3 {
        #[cfg(all(vec4sse2, vec3sse2))]
        {
            self.0.into()
        }

        #[cfg(all(vec4sse2, not(vec3sse2)))]
        {
            let (x, y, z, _) = self.into();
            Vec3::new(x, y, z)
        }

        #[cfg(vec4f32)]
        {
            Vec3::new(self.0, self.1, self.2)
        }
    }

    /// Returns element `x`.
    #[inline]
    pub fn x(self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            _mm_cvtss_f32(self.0)
        }

        #[cfg(vec4f32)]
        {
            self.0
        }
    }

    /// Returns element `y`.
    #[inline]
    pub fn y(self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b01_01_01_01))
        }

        #[cfg(vec4f32)]
        {
            self.1
        }
    }

    /// Returns element `z`.
    #[inline]
    pub fn z(self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b10_10_10_10))
        }

        #[cfg(vec4f32)]
        {
            self.2
        }
    }

    /// Returns element `w`.
    #[inline]
    pub fn w(self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b11_11_11_11))
        }

        #[cfg(vec4f32)]
        {
            self.3
        }
    }

    /// Returns a mutable reference to element `x`.
    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        #[cfg(vec4sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32)
        }

        #[cfg(vec4f32)]
        {
            &mut self.0
        }
    }

    /// Returns a mutable reference to element `y`.
    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        #[cfg(vec4sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32).offset(1)
        }

        #[cfg(vec4f32)]
        {
            &mut self.1
        }
    }

    /// Returns a mutable reference to element `z`.
    #[inline]
    pub fn z_mut(&mut self) -> &mut f32 {
        #[cfg(vec4sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32).offset(2)
        }

        #[cfg(vec4f32)]
        {
            &mut self.2
        }
    }

    /// Returns a mutable reference to element `w`.
    #[inline]
    pub fn w_mut(&mut self) -> &mut f32 {
        #[cfg(vec4sse2)]
        unsafe {
            &mut *(self as *mut Self as *mut f32).offset(3)
        }

        #[cfg(vec4f32)]
        {
            &mut self.3
        }
    }

    /// Sets element `x`.
    #[inline]
    pub fn set_x(&mut self, x: f32) {
        #[cfg(vec4sse2)]
        unsafe {
            self.0 = _mm_move_ss(self.0, _mm_set_ss(x));
        }

        #[cfg(vec4f32)]
        {
            self.0 = x;
        }
    }

    /// Sets element `y`.
    #[inline]
    pub fn set_y(&mut self, y: f32) {
        #[cfg(vec4sse2)]
        unsafe {
            let mut t = _mm_move_ss(self.0, _mm_set_ss(y));
            t = _mm_shuffle_ps(t, t, 0b11_10_00_00);
            self.0 = _mm_move_ss(t, self.0);
        }

        #[cfg(vec4f32)]
        {
            self.1 = y;
        }
    }

    /// Sets element `z`.
    #[inline]
    pub fn set_z(&mut self, z: f32) {
        #[cfg(vec4sse2)]
        unsafe {
            let mut t = _mm_move_ss(self.0, _mm_set_ss(z));
            t = _mm_shuffle_ps(t, t, 0b11_00_01_00);
            self.0 = _mm_move_ss(t, self.0);
        }

        #[cfg(vec4f32)]
        {
            self.2 = z;
        }
    }

    /// Sets element `w`.
    #[inline]
    pub fn set_w(&mut self, w: f32) {
        #[cfg(vec4sse2)]
        unsafe {
            let mut t = _mm_move_ss(self.0, _mm_set_ss(w));
            t = _mm_shuffle_ps(t, t, 0b00_10_01_00);
            self.0 = _mm_move_ss(t, self.0);
        }

        #[cfg(vec4f32)]
        {
            self.3 = w;
        }
    }

    /// Returns a `Vec4` with all elements set to the value of element `x`.
    #[inline]
    pub(crate) fn dup_x(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b00_00_00_00))
        }

        #[cfg(vec4f32)]
        {
            Self(self.0, self.0, self.0, self.0)
        }
    }

    /// Returns a `Vec4` with all elements set to the value of element `y`.
    #[inline]
    pub(crate) fn dup_y(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b01_01_01_01))
        }

        #[cfg(vec4f32)]
        {
            Self(self.1, self.1, self.1, self.1)
        }
    }

    /// Returns a `Vec4` with all elements set to the value of element `z`.
    #[inline]
    pub(crate) fn dup_z(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b10_10_10_10))
        }

        #[cfg(vec4f32)]
        {
            Self(self.2, self.2, self.2, self.2)
        }
    }

    /// Returns a `Vec4` with all elements set to the value of element `w`.
    #[inline]
    pub(crate) fn dup_w(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_shuffle_ps(self.0, self.0, 0b11_11_11_11))
        }

        #[cfg(vec4f32)]
        {
            Self(self.3, self.3, self.3, self.3)
        }
    }

    /// Calculates the Vec4 dot product and returns answer in x lane of __m128.
    #[cfg(vec4sse2)]
    #[inline]
    unsafe fn dot_as_m128(self, other: Self) -> __m128 {
        let x2_y2_z2_w2 = _mm_mul_ps(self.0, other.0);
        let z2_w2_0_0 = _mm_shuffle_ps(x2_y2_z2_w2, x2_y2_z2_w2, 0b00_00_11_10);
        let x2z2_y2w2_0_0 = _mm_add_ps(x2_y2_z2_w2, z2_w2_0_0);
        let y2w2_0_0_0 = _mm_shuffle_ps(x2z2_y2w2_0_0, x2z2_y2w2_0_0, 0b00_00_00_01);
        _mm_add_ps(x2z2_y2w2_0_0, y2w2_0_0_0)
    }

    /// Returns Vec4 dot in all lanes of Vec4
    #[cfg(vec4sse2)]
    #[inline]
    pub(crate) fn dot_as_vec4(self, other: Self) -> Self {
        unsafe {
            let dot_in_x = self.dot_as_m128(other);
            Self(_mm_shuffle_ps(dot_in_x, dot_in_x, 0b00_00_00_00))
        }
    }

    /// Computes the 4D dot product of `self` and `other`.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            _mm_cvtss_f32(self.dot_as_m128(other))
        }

        #[cfg(vec4f32)]
        {
            (self.0 * other.0) + (self.1 * other.1) + (self.2 * other.2) + (self.3 * other.3)
        }
    }

    /// Computes the 4D length of `self`.
    #[inline]
    pub fn length(self) -> f32 {
        #[cfg(vec4sse2)]
        {
            let dot = self.dot_as_vec4(self);
            unsafe { _mm_cvtss_f32(_mm_sqrt_ps(dot.0)) }
        }

        #[cfg(vec4f32)]
        {
            self.dot(self).sqrt()
        }
    }

    /// Computes the squared 4D length of `self`.
    ///
    /// This is generally faster than `Vec4::length()` as it avoids a square
    /// root operation.
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes `1.0 / Vec4::length()`.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn length_reciprocal(self) -> f32 {
        #[cfg(vec4sse2)]
        {
            let dot = self.dot_as_vec4(self);
            unsafe {
                // _mm_rsqrt_ps is lower precision
                _mm_cvtss_f32(_mm_div_ps(_mm_set_ps1(1.0), _mm_sqrt_ps(dot.0)))
            }
        }

        #[cfg(vec4f32)]
        {
            1.0 / self.length()
        }
    }

    /// Returns `self` normalized to length 1.0.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn normalize(self) -> Self {
        #[cfg(vec4sse2)]
        {
            let dot = self.dot_as_vec4(self);
            unsafe { Self(_mm_div_ps(self.0, _mm_sqrt_ps(dot.0))) }
        }

        #[cfg(vec4f32)]
        {
            self * self.length_reciprocal()
        }
    }

    /// Returns the vertical minimum of `self` and `other`.
    ///
    /// In other words, this computes
    /// `[x: min(x1, x2), y: min(y1, y2), z: min(z1, z2), w: min(w1, w2)]`,
    /// taking the minimum of each element individually.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_min_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0.min(other.0),
                self.1.min(other.1),
                self.2.min(other.2),
                self.3.min(other.3),
            )
        }
    }

    /// Returns the vertical maximum of `self` and `other`.
    ///
    /// In other words, this computes
    /// `[x: max(x1, x2), y: max(y1, y2), z: max(z1, z2), w: max(w1, w2)]`,
    /// taking the maximum of each element individually.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_max_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0.max(other.0),
                self.1.max(other.1),
                self.2.max(other.2),
                self.3.max(other.3),
            )
        }
    }

    /// Returns the horizontal minimum of `self`'s elements.
    ///
    /// In other words, this computes `min(x, y, z, w)`.
    #[inline]
    pub fn min_element(self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            let v = self.0;
            let v = _mm_min_ps(v, _mm_shuffle_ps(v, v, 0b00_00_11_10));
            let v = _mm_min_ps(v, _mm_shuffle_ps(v, v, 0b00_00_00_01));
            _mm_cvtss_f32(v)
        }

        #[cfg(vec4f32)]
        {
            self.0.min(self.1.min(self.2.min(self.3)))
        }
    }

    /// Returns the horizontal maximum of `self`'s elements.
    ///
    /// In other words, this computes `max(x, y, z, w)`.
    #[inline]
    pub fn max_element(self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            let v = self.0;
            let v = _mm_max_ps(v, _mm_shuffle_ps(v, v, 0b00_00_11_10));
            let v = _mm_max_ps(v, _mm_shuffle_ps(v, v, 0b00_00_00_01));
            _mm_cvtss_f32(v)
        }

        #[cfg(vec4f32)]
        {
            self.0.max(self.1.max(self.2.min(self.3)))
        }
    }

    /// Performs a vertical `==` comparison between `self` and `other`,
    /// returning a `Vec4Mask` of the results.
    ///
    /// In other words, this computes `[x1 == x2, y1 == y2, z1 == z2, w1 == w2]`.
    #[inline]
    pub fn cmpeq(self, other: Self) -> Vec4Mask {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4Mask(_mm_cmpeq_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4Mask::new(
                self.0.eq(&other.0),
                self.1.eq(&other.1),
                self.2.eq(&other.2),
                self.3.eq(&other.3),
            )
        }
    }

    /// Performs a vertical `!=` comparison between `self` and `other`,
    /// returning a `Vec4Mask` of the results.
    ///
    /// In other words, this computes `[x1 != x2, y1 != y2, z1 != z2, w1 != w2]`.
    #[inline]
    pub fn cmpne(self, other: Self) -> Vec4Mask {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4Mask(_mm_cmpneq_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4Mask::new(
                self.0.ne(&other.0),
                self.1.ne(&other.1),
                self.2.ne(&other.2),
                self.3.ne(&other.3),
            )
        }
    }

    /// Performs a vertical `>=` comparison between `self` and `other`,
    /// returning a `Vec4Mask` of the results.
    ///
    /// In other words, this computes `[x1 >= x2, y1 >= y2, z1 >= z2, w1 >= w2]`.
    #[inline]
    pub fn cmpge(self, other: Self) -> Vec4Mask {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4Mask(_mm_cmpge_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4Mask::new(
                self.0.ge(&other.0),
                self.1.ge(&other.1),
                self.2.ge(&other.2),
                self.3.ge(&other.3),
            )
        }
    }

    /// Performs a vertical `>` comparison between `self` and `other`,
    /// returning a `Vec4Mask` of the results.
    ///
    /// In other words, this computes `[x1 > x2, y1 > y2, z1 > z2, w1 > w2]`.
    #[inline]
    pub fn cmpgt(self, other: Self) -> Vec4Mask {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4Mask(_mm_cmpgt_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4Mask::new(
                self.0.gt(&other.0),
                self.1.gt(&other.1),
                self.2.gt(&other.2),
                self.3.gt(&other.3),
            )
        }
    }

    /// Performs a vertical `<=` comparison between `self` and `other`,
    /// returning a `Vec4Mask` of the results.
    ///
    /// In other words, this computes `[x1 <= x2, y1 <= y2, z1 <= z2, w1 <= w2]`.
    #[inline]
    pub fn cmple(self, other: Self) -> Vec4Mask {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4Mask(_mm_cmple_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4Mask::new(
                self.0.le(&other.0),
                self.1.le(&other.1),
                self.2.le(&other.2),
                self.3.le(&other.3),
            )
        }
    }

    /// Performs a vertical `<` comparison between `self` and `other`,
    /// returning a `Vec4Mask` of the results.
    ///
    /// In other words, this computes `[x1 < x2, y1 < y2, z1 < z2, w1 < w2]`.
    #[inline]
    pub fn cmplt(self, other: Self) -> Vec4Mask {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4Mask(_mm_cmplt_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4Mask::new(
                self.0.lt(&other.0),
                self.1.lt(&other.1),
                self.2.lt(&other.2),
                self.3.lt(&other.3),
            )
        }
    }

    /// Creates a new `Vec4` from the first four values in `slice`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is less than four elements long.
    #[inline]
    pub fn from_slice_unaligned(slice: &[f32]) -> Self {
        #[cfg(vec4sse2)]
        {
            assert!(slice.len() >= 4);
            unsafe { Self(_mm_loadu_ps(slice.as_ptr())) }
        }

        #[cfg(vec4f32)]
        {
            Self(slice[0], slice[1], slice[2], slice[3])
        }
    }

    /// Writes the elements of `self` to the first four elements in `slice`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is less than four elements long.
    #[inline]
    pub fn write_to_slice_unaligned(self, slice: &mut [f32]) {
        #[cfg(vec4sse2)]
        unsafe {
            assert!(slice.len() >= 4);
            _mm_storeu_ps(slice.as_mut_ptr(), self.0);
        }

        #[cfg(vec4f32)]
        {
            slice[0] = self.0;
            slice[1] = self.1;
            slice[2] = self.2;
            slice[3] = self.3;
        }
    }

    /// Per element multiplication/addition of the three inputs: b + (self * a)
    #[inline]
    pub(crate) fn mul_add(self, a: Self, b: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_add_ps(_mm_mul_ps(self.0, a.0), b.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                (self.0 * a.0) + b.0,
                (self.1 * a.1) + b.1,
                (self.2 * a.2) + b.2,
                (self.3 * a.3) + b.3,
            )
        }
    }

    /// Returns a new `Vec4` containing the absolute value of each element of the original
    /// `Vec4`.
    #[inline]
    pub fn abs(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_and_ps(
                self.0,
                _mm_castsi128_ps(_mm_set1_epi32(0x7f_ff_ff_ff)),
            ))
        }

        #[cfg(vec4f32)]
        {
            Self(self.0.abs(), self.1.abs(), self.2.abs(), self.3.abs())
        }
    }

    #[inline]
    pub fn round(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            use crate::f32::funcs::sse2::m128_round;
            Self(m128_round(self.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0.round(),
                self.1.round(),
                self.2.round(),
                self.3.round(),
            )
        }
    }

    #[inline]
    pub fn floor(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            use crate::f32::funcs::sse2::m128_floor;
            Self(m128_floor(self.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0.floor(),
                self.1.floor(),
                self.2.floor(),
                self.3.floor(),
            )
        }
    }

    #[inline]
    pub fn ceil(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            use crate::f32::funcs::sse2::m128_ceil;
            Self(m128_ceil(self.0))
        }

        #[cfg(vec4f32)]
        {
            Self(self.0.ceil(), self.1.ceil(), self.2.ceil(), self.3.ceil())
        }
    }

    /// Returns a new `Vec4` with elements representing the sign of `self`.
    ///
    /// - `1.0` if the number is positive, `+0.0` or `INFINITY`
    /// - `-1.0` if the number is negative, `-0.0` or `NEG_INFINITY`
    #[inline]
    pub fn sign(self) -> Self {
        #[cfg(vec4sse2)]
        {
            let mask = self.cmpge(Self::zero());
            mask.select(Self::splat(1.0), Self::splat(-1.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0.signum(),
                self.1.signum(),
                self.2.signum(),
                self.3.signum(),
            )
        }
    }

    /// Computes the reciprocal `1.0/n` of each element, returning the
    /// results in a new `Vec4`.
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
    /// This can be used to compare if two `Vec4`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(self, other: Self, max_abs_diff: f32) -> bool {
        abs_diff_eq!(self, other, max_abs_diff)
    }
}

impl AsRef<[f32; 4]> for Vec4 {
    #[inline]
    fn as_ref(&self) -> &[f32; 4] {
        unsafe { &*(self as *const Self as *const [f32; 4]) }
    }
}

impl AsMut<[f32; 4]> for Vec4 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 4] {
        unsafe { &mut *(self as *mut Self as *mut [f32; 4]) }
    }
}

impl fmt::Display for Vec4 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(vec4sse2)]
        {
            let (x, y, z, w) = (*self).into();
            write!(fmt, "[{}, {}, {}, {}]", x, y, z, w)
        }

        #[cfg(vec4f32)]
        {
            write!(fmt, "[{}, {}, {}, {}]", self.0, self.1, self.2, self.3)
        }
    }
}

impl Div<Vec4> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_div_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0 / other.0,
                self.1 / other.1,
                self.2 / other.2,
                self.3 / other.3,
            )
        }
    }
}

impl DivAssign<Vec4> for Vec4 {
    #[inline]
    fn div_assign(&mut self, other: Self) {
        #[cfg(vec4sse2)]
        {
            self.0 = unsafe { _mm_div_ps(self.0, other.0) };
        }

        #[cfg(vec4f32)]
        {
            self.0 /= other.0;
            self.1 /= other.1;
            self.2 /= other.2;
            self.3 /= other.3;
        }
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, other: f32) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_div_ps(self.0, _mm_set1_ps(other)))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0 / other,
                self.1 / other,
                self.2 / other,
                self.3 / other,
            )
        }
    }
}

impl DivAssign<f32> for Vec4 {
    #[inline]
    fn div_assign(&mut self, other: f32) {
        #[cfg(vec4sse2)]
        {
            self.0 = unsafe { _mm_div_ps(self.0, _mm_set1_ps(other)) };
        }

        #[cfg(vec4f32)]
        {
            self.0 /= other;
            self.1 /= other;
            self.2 /= other;
            self.3 /= other;
        }
    }
}

impl Mul<Vec4> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_mul_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0 * other.0,
                self.1 * other.1,
                self.2 * other.2,
                self.3 * other.3,
            )
        }
    }
}

impl MulAssign<Vec4> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        #[cfg(vec4sse2)]
        {
            self.0 = unsafe { _mm_mul_ps(self.0, other.0) };
        }

        #[cfg(vec4f32)]
        {
            self.0 *= other.0;
            self.1 *= other.1;
            self.2 *= other.2;
            self.3 *= other.3;
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, other: f32) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_mul_ps(self.0, _mm_set1_ps(other)))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0 * other,
                self.1 * other,
                self.2 * other,
                self.3 * other,
            )
        }
    }
}

impl MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, other: f32) {
        #[cfg(vec4sse2)]
        {
            self.0 = unsafe { _mm_mul_ps(self.0, _mm_set1_ps(other)) };
        }

        #[cfg(vec4f32)]
        {
            self.0 *= other;
            self.1 *= other;
            self.2 *= other;
            self.3 *= other;
        }
    }
}

impl Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, other: Vec4) -> Vec4 {
        #[cfg(vec4sse2)]
        unsafe {
            Vec4(_mm_mul_ps(_mm_set1_ps(self), other.0))
        }

        #[cfg(vec4f32)]
        {
            Vec4(
                self * other.0,
                self * other.1,
                self * other.2,
                self * other.3,
            )
        }
    }
}

impl Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_add_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0 + other.0,
                self.1 + other.1,
                self.2 + other.2,
                self.3 + other.3,
            )
        }
    }
}

impl AddAssign for Vec4 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        #[cfg(vec4sse2)]
        {
            self.0 = unsafe { _mm_add_ps(self.0, other.0) };
        }

        #[cfg(vec4f32)]
        {
            self.0 += other.0;
            self.1 += other.1;
            self.2 += other.2;
            self.3 += other.3;
        }
    }
}

impl Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_sub_ps(self.0, other.0))
        }

        #[cfg(vec4f32)]
        {
            Self(
                self.0 - other.0,
                self.1 - other.1,
                self.2 - other.2,
                self.3 - other.3,
            )
        }
    }
}

impl SubAssign for Vec4 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        #[cfg(vec4sse2)]
        {
            self.0 = unsafe { _mm_sub_ps(self.0, other.0) };
        }

        #[cfg(vec4f32)]
        {
            self.0 -= other.0;
            self.1 -= other.1;
            self.2 -= other.2;
            self.3 -= other.3;
        }
    }
}

impl Neg for Vec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_sub_ps(_mm_set1_ps(0.0), self.0))
        }

        #[cfg(vec4f32)]
        {
            Self(-self.0, -self.1, -self.2, -self.3)
        }
    }
}

impl Index<usize> for Vec4 {
    type Output = f32;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl IndexMut<usize> for Vec4 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl From<(f32, f32, f32, f32)> for Vec4 {
    #[inline]
    fn from(t: (f32, f32, f32, f32)) -> Self {
        Self::new(t.0, t.1, t.2, t.3)
    }
}

impl From<Vec4> for (f32, f32, f32, f32) {
    #[inline]
    fn from(v: Vec4) -> Self {
        #[cfg(vec4sse2)]
        {
            let mut out: MaybeUninit<Align16<(f32, f32, f32, f32)>> = MaybeUninit::uninit();
            unsafe {
                _mm_store_ps(out.as_mut_ptr() as *mut f32, v.0);
                out.assume_init().0
            }
        }

        #[cfg(vec4f32)]
        {
            (v.0, v.1, v.2, v.3)
        }
    }
}

impl From<[f32; 4]> for Vec4 {
    #[inline]
    fn from(a: [f32; 4]) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(_mm_loadu_ps(a.as_ptr()))
        }

        #[cfg(vec4f32)]
        {
            Self(a[0], a[1], a[2], a[3])
        }
    }
}

impl From<Vec4> for [f32; 4] {
    #[inline]
    fn from(v: Vec4) -> Self {
        #[cfg(vec4sse2)]
        {
            let mut out: MaybeUninit<Align16<[f32; 4]>> = MaybeUninit::uninit();
            unsafe {
                _mm_store_ps(out.as_mut_ptr() as *mut f32, v.0);
                out.assume_init().0
            }
        }

        #[cfg(vec4f32)]
        {
            [v.0, v.1, v.2, v.3]
        }
    }
}

#[test]
fn test_vec4_private() {
    assert_eq!(
        vec4(1.0, 1.0, 1.0, 1.0).mul_add(vec4(0.5, 2.0, -4.0, 0.0), vec4(-1.0, -1.0, -1.0, -1.0)),
        vec4(-0.5, 1.0, -5.0, -1.0)
    );
    assert_eq!(vec4(1.0, 2.0, 3.0, 4.0).dup_x(), vec4(1.0, 1.0, 1.0, 1.0));
    assert_eq!(vec4(1.0, 2.0, 3.0, 4.0).dup_y(), vec4(2.0, 2.0, 2.0, 2.0));
    assert_eq!(vec4(1.0, 2.0, 3.0, 4.0).dup_z(), vec4(3.0, 3.0, 3.0, 3.0));
    assert_eq!(vec4(1.0, 2.0, 4.0, 4.0).dup_w(), vec4(4.0, 4.0, 4.0, 4.0));
}
