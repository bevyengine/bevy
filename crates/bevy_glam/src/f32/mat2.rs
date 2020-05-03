use super::{scalar_sin_cos, Vec2, Vec4};
#[cfg(all(vec4sse2, target_arch = "x86",))]
use core::arch::x86::*;
#[cfg(all(vec4sse2, target_arch = "x86_64",))]
use core::arch::x86_64::*;
use core::{
    fmt,
    ops::{Add, Mul, Sub},
};

#[inline]
pub fn mat2(x_axis: Vec2, y_axis: Vec2) -> Mat2 {
    Mat2::from_cols(x_axis, y_axis)
}

/// A 2x2 column major matrix.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
#[repr(C)]
pub struct Mat2(pub(crate) Vec4);

impl Default for Mat2 {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for Mat2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}, {}]", self.x_axis(), self.y_axis())
    }
}

impl Mat2 {
    /// Creates a 2x2 matrix with all elements set to `0.0`.
    #[inline]
    pub fn zero() -> Self {
        Mat2(Vec4::zero())
    }

    /// Creates a 2x2 identity matrix.
    #[inline]
    pub fn identity() -> Self {
        Self(Vec4::new(1.0, 0.0, 0.0, 1.0))
    }

    /// Creates a 2x2 matrix from four column vectors.
    #[inline]
    pub fn from_cols(x_axis: Vec2, y_axis: Vec2) -> Self {
        Self(Vec4::new(x_axis.x(), x_axis.y(), y_axis.x(), y_axis.y()))
    }

    /// Creates a 2x2 matrix from a `[f32; 4]` stored in column major order.  If
    /// your data is stored in row major you will need to `transpose` the
    /// returned matrix.
    #[inline]
    pub fn from_cols_array(m: &[f32; 4]) -> Self {
        Mat2(Vec4::new(m[0], m[1], m[2], m[3]))
    }

    /// Creates a `[f32; 4]` storing data in column major order.
    /// If you require data in row major order `transpose` the matrix first.
    #[inline]
    pub fn to_cols_array(&self) -> [f32; 4] {
        self.0.into()
    }

    /// Creates a 2x2 matrix from a `[[f32; 2]; 2]` stored in column major
    /// order.  If your data is in row major order you will need to `transpose`
    /// the returned matrix.
    #[inline]
    pub fn from_cols_array_2d(m: &[[f32; 2]; 2]) -> Self {
        Mat2(Vec4::new(m[0][0], m[0][1], m[1][0], m[1][1]))
    }

    /// Creates a `[[f32; 2]; 2]` storing data in column major order.
    /// If you require data in row major order `transpose` the matrix first.
    #[inline]
    pub fn to_cols_array_2d(&self) -> [[f32; 2]; 2] {
        let (x0, y0, x1, y1) = self.0.into();
        [[x0, y0], [x1, y1]]
    }

    /// Creates a 2x2 matrix containing the given `scale` and rotation of
    /// `angle` (in radians).
    #[inline]
    pub fn from_scale_angle(scale: Vec2, angle: f32) -> Self {
        let (sin, cos) = scalar_sin_cos(angle);
        let (scale_x, scale_y) = scale.into();
        Self(Vec4::new(
            cos * scale_x,
            sin * scale_x,
            -sin * scale_y,
            cos * scale_y,
        ))
    }

    /// Creates a 2x2 matrix containing a rotation of `angle` (in radians).
    #[inline]
    pub fn from_angle(angle: f32) -> Self {
        let (sin, cos) = scalar_sin_cos(angle);
        Self(Vec4::new(cos, sin, -sin, cos))
    }

    /// Creates a 2x2 matrix containing the given non-uniform `scale`.
    #[inline]
    pub fn from_scale(scale: Vec2) -> Self {
        let (x, y) = scale.into();
        Self(Vec4::new(x, 0.0, 0.0, y))
    }

    #[inline]
    pub fn set_x_axis(&mut self, x: Vec2) {
        let m = self.0.as_mut();
        m[0] = x.x();
        m[1] = x.y();
    }

    #[inline]
    pub fn set_y_axis(&mut self, y: Vec2) {
        let m = self.0.as_mut();
        m[2] = y.x();
        m[3] = y.y();
    }

    #[inline]
    pub fn x_axis(&self) -> Vec2 {
        let (x, y, _, _) = self.0.into();
        Vec2::new(x, y)
    }

    #[inline]
    pub fn y_axis(&self) -> Vec2 {
        let (_, _, x, y) = self.0.into();
        Vec2::new(x, y)
    }

    // #[inline]
    // pub(crate) fn col(&self, index: usize) -> Vec2 {
    //     match index {
    //         0 => self.x_axis(),
    //         1 => self.y_axis(),
    //         _ => panic!(
    //             "index out of bounds: the len is 2 but the index is {}",
    //             index
    //         ),
    //     }
    // }

    // #[inline]
    // pub(crate) fn col_mut(&mut self, index: usize) -> &mut Vec2 {
    //     match index {
    //         0 => unsafe { &mut *(self.0.as_mut().as_mut_ptr() as *mut Vec2) },
    //         1 => unsafe { &mut *(self.0.as_mut()[2..].as_mut_ptr() as *mut Vec2) },
    //         _ => panic!(
    //             "index out of bounds: the len is 2 but the index is {}",
    //             index
    //         ),
    //     }
    // }

    /// Returns the transpose of `self`.
    #[inline]
    pub fn transpose(&self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            let abcd = self.0.into();
            let acbd = _mm_shuffle_ps(abcd, abcd, 0b11_01_10_00);
            Self(acbd.into())
        }

        #[cfg(vec4f32)]
        {
            let (m00, m01, m10, m11) = self.0.into();
            Self(Vec4::new(m00, m10, m01, m11))
        }
    }

    /// Returns the determinant of `self`.
    #[inline]
    pub fn determinant(&self) -> f32 {
        #[cfg(vec4sse2)]
        unsafe {
            let abcd = self.0.into();
            let dcba = _mm_shuffle_ps(abcd, abcd, 0b00_01_10_11);
            let prod = _mm_mul_ps(abcd, dcba);
            let det = _mm_sub_ps(prod, _mm_shuffle_ps(prod, prod, 0b01_01_01_01));
            _mm_cvtss_f32(det)
        }

        #[cfg(vec4f32)]
        {
            let (a, b, c, d) = self.0.into();
            a * d - b * c
        }
    }

    /// Returns the inverse of `self`.
    ///
    /// If the matrix is not invertible the returned matrix will be invalid.
    #[inline]
    pub fn inverse(&self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            let abcd = self.0.into();
            let dcba = _mm_shuffle_ps(abcd, abcd, 0b00_01_10_11);
            let prod = _mm_mul_ps(abcd, dcba);
            let sub = _mm_sub_ps(prod, _mm_shuffle_ps(prod, prod, 0b01_01_01_01));
            let det = _mm_shuffle_ps(sub, sub, 0b00_00_00_00);
            let tmp = _mm_div_ps(_mm_set_ps(1.0, -1.0, -1.0, 1.0), det);
            let dbca = _mm_shuffle_ps(abcd, abcd, 0b00_10_01_11);
            Self(_mm_mul_ps(dbca, tmp).into())
        }

        #[cfg(vec4f32)]
        {
            let (a, b, c, d) = self.0.into();
            let det = a * d - b * c;
            glam_assert!(det != 0.0);
            let tmp = Vec4::new(1.0, -1.0, -1.0, 1.0) / det;
            Self(Vec4::new(d, b, c, a) * tmp)
        }
    }

    #[inline]
    pub fn mul_vec2(&self, other: Vec2) -> Vec2 {
        // TODO: SSE2
        let other = Vec4::new(other.x(), other.x(), other.y(), other.y());
        let tmp = self.0 * other;
        let (x0, y0, x1, y1) = tmp.into();
        Vec2::new(x0 + x1, y0 + y1)
    }

    #[inline]
    pub fn mul_mat2(&self, other: &Self) -> Self {
        // TODO: SSE2
        let (x0, y0, x1, y1) = other.0.into();
        Mat2::from_cols(
            self.mul_vec2(Vec2::new(x0, y0)),
            self.mul_vec2(Vec2::new(x1, y1)),
        )
    }

    #[inline]
    pub fn add_mat2(&self, other: &Self) -> Self {
        Mat2(self.0 + other.0)
    }

    #[inline]
    pub fn sub_mat2(&self, other: &Self) -> Self {
        Mat2(self.0 - other.0)
    }

    #[inline]
    pub fn mul_scalar(&self, other: f32) -> Self {
        let s = Vec4::splat(other);
        Mat2(self.0 * s)
    }

    /// Returns true if the absolute difference of all elements between `self`
    /// and `other` is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two `Mat2`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(&self, other: Self, max_abs_diff: f32) -> bool {
        self.0.abs_diff_eq(other.0, max_abs_diff)
    }
}

impl AsRef<[f32; 4]> for Mat2 {
    #[inline]
    fn as_ref(&self) -> &[f32; 4] {
        unsafe { &*(self as *const Self as *const [f32; 4]) }
    }
}

impl AsMut<[f32; 4]> for Mat2 {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 4] {
        unsafe { &mut *(self as *mut Self as *mut [f32; 4]) }
    }
}

impl Add<Mat2> for Mat2 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        self.add_mat2(&other)
    }
}

impl Sub<Mat2> for Mat2 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        self.sub_mat2(&other)
    }
}

impl Mul<Mat2> for Mat2 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        self.mul_mat2(&other)
    }
}

impl Mul<Vec2> for Mat2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, other: Vec2) -> Vec2 {
        self.mul_vec2(other)
    }
}

impl Mul<Mat2> for f32 {
    type Output = Mat2;
    #[inline]
    fn mul(self, other: Mat2) -> Mat2 {
        other.mul_scalar(self)
    }
}

impl Mul<f32> for Mat2 {
    type Output = Self;
    #[inline]
    fn mul(self, other: f32) -> Self {
        self.mul_scalar(other)
    }
}
