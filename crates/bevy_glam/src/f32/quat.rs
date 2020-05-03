use super::{scalar_acos, scalar_sin_cos, Mat3, Mat4, Vec3, Vec4};
#[cfg(all(vec4sse2, target_arch = "x86",))]
use core::arch::x86::*;
#[cfg(all(vec4sse2, target_arch = "x86_64",))]
use core::arch::x86_64::*;
use core::{
    cmp::Ordering,
    fmt,
    ops::{Mul, MulAssign, Neg},
};

/// A quaternion representing an orientation.
///
/// This quaternion is intended to be of unit length but may denormalize due to
/// floating point "error creep" which can occur when successive quaternion
/// operations are applied.
///
/// This type is 16 byte aligned.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Quat(pub(crate) Vec4);

#[inline]
pub fn quat(x: f32, y: f32, z: f32, w: f32) -> Quat {
    Quat::from_xyzw(x, y, z, w)
}

impl Quat {
    /// Creates a new rotation quaternion.
    ///
    /// This should generally not be called manually unless you know what you are doing. Use one of
    /// the other constructors instead such as `identity` or `from_axis_angle`.
    ///
    /// `from_xyzw` is mostly used by unit tests and `serde` deserialization.
    #[inline]
    pub fn from_xyzw(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self(Vec4::new(x, y, z, w))
    }

    #[inline]
    pub fn identity() -> Self {
        Self(Vec4::new(0.0, 0.0, 0.0, 1.0))
    }

    /// Creates a new rotation quaternion from an unaligned `&[f32]`.
    ///
    /// # Preconditions
    ///
    /// The resulting quaternion is expected to be of unit length.
    ///
    /// # Panics
    ///
    /// Panics if `slice` length is less than 4.
    #[inline]
    pub fn from_slice_unaligned(slice: &[f32]) -> Self {
        let q = Self(Vec4::from_slice_unaligned(slice));
        glam_assert!(q.is_normalized());
        q
    }

    /// Writes the quaternion to an unaligned `&mut [f32]`.
    ///
    /// # Panics
    ///
    /// Panics if `slice` length is less than 4.
    #[inline]
    pub fn write_to_slice_unaligned(self, slice: &mut [f32]) {
        self.0.write_to_slice_unaligned(slice)
    }

    /// Create a new quaterion for a normalized rotation axis and angle
    /// (in radians).
    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        glam_assert!(axis.is_normalized());
        let (s, c) = scalar_sin_cos(angle * 0.5);
        Self((axis * s).extend(c))
    }

    /// Creates a new quaternion from the angle (in radians) around the x axis.
    #[inline]
    pub fn from_rotation_x(angle: f32) -> Self {
        let (s, c) = scalar_sin_cos(angle * 0.5);
        Self::from_xyzw(s, 0.0, 0.0, c)
    }

    /// Creates a new quaternion from the angle (in radians) around the y axis.
    #[inline]
    pub fn from_rotation_y(angle: f32) -> Self {
        let (s, c) = scalar_sin_cos(angle * 0.5);
        Self::from_xyzw(0.0, s, 0.0, c)
    }

    /// Creates a new quaternion from the angle (in radians) around the z axis.
    #[inline]
    pub fn from_rotation_z(angle: f32) -> Self {
        let (s, c) = scalar_sin_cos(angle * 0.5);
        Self::from_xyzw(0.0, 0.0, s, c)
    }

    #[inline]
    /// Create a quaternion from the given yaw (around y), pitch (around x) and roll (around z)
    /// in radians.
    pub fn from_rotation_ypr(yaw: f32, pitch: f32, roll: f32) -> Self {
        // Self::from_rotation_y(yaw) * Self::from_rotation_x(pitch) * Self::from_rotation_z(roll)
        let (y0, w0) = scalar_sin_cos(yaw * 0.5);
        let (x1, w1) = scalar_sin_cos(pitch * 0.5);
        let (z2, w2) = scalar_sin_cos(roll * 0.5);

        let x3 = w0 * x1;
        let y3 = y0 * w1;
        let z3 = -y0 * x1;
        let w3 = w0 * w1;

        let x4 = x3 * w2 + y3 * z2;
        let y4 = -x3 * z2 + y3 * w2;
        let z4 = w3 * z2 + z3 * w2;
        let w4 = w3 * w2 - z3 * z2;

        Self(Vec4::new(x4, y4, z4, w4))
    }

    #[inline]
    fn from_rotation_axes(x_axis: Vec3, y_axis: Vec3, z_axis: Vec3) -> Self {
        // from DirectXMath XMQuaternionRotationMatrix
        // TODO: sse2 version
        let (m00, m01, m02) = x_axis.into();
        let (m10, m11, m12) = y_axis.into();
        let (m20, m21, m22) = z_axis.into();
        if m22 <= 0.0 {
            // x^2 + y^2 >= z^2 + w^2
            let dif10 = m11 - m00;
            let omm22 = 1.0 - m22;
            if dif10 <= 0.0 {
                // x^2 >= y^2
                let four_xsq = omm22 - dif10;
                let inv4x = 0.5 / four_xsq.sqrt();
                Self::from_xyzw(
                    four_xsq * inv4x,
                    (m01 + m10) * inv4x,
                    (m02 + m20) * inv4x,
                    (m12 - m21) * inv4x,
                )
            } else {
                // y^2 >= x^2
                let four_ysq = omm22 + dif10;
                let inv4y = 0.5 / four_ysq.sqrt();
                Self::from_xyzw(
                    (m01 + m10) * inv4y,
                    four_ysq * inv4y,
                    (m12 + m21) * inv4y,
                    (m20 - m02) * inv4y,
                )
            }
        } else {
            // z^2 + w^2 >= x^2 + y^2
            let sum10 = m11 + m00;
            let opm22 = 1.0 + m22;
            if sum10 <= 0.0 {
                // z^2 >= w^2
                let four_zsq = opm22 - sum10;
                let inv4z = 0.5 / four_zsq.sqrt();
                Self::from_xyzw(
                    (m02 + m20) * inv4z,
                    (m12 + m21) * inv4z,
                    four_zsq * inv4z,
                    (m01 - m10) * inv4z,
                )
            } else {
                // w^2 >= z^2
                let four_wsq = opm22 + sum10;
                let inv4w = 0.5 / four_wsq.sqrt();
                Self::from_xyzw(
                    (m12 - m21) * inv4w,
                    (m20 - m02) * inv4w,
                    (m01 - m10) * inv4w,
                    four_wsq * inv4w,
                )
            }
        }
    }

    /// Creates a new quaternion from a 3x3 rotation matrix.
    #[inline]
    pub fn from_rotation_mat3(mat: &Mat3) -> Self {
        Self::from_rotation_axes(mat.x_axis(), mat.y_axis(), mat.z_axis())
    }

    /// Creates a new quaternion from a 3x3 rotation matrix inside a homogeneous
    /// 4x4 matrix.
    #[inline]
    pub fn from_rotation_mat4(mat: &Mat4) -> Self {
        Self::from_rotation_axes(
            mat.x_axis().truncate(),
            mat.y_axis().truncate(),
            mat.z_axis().truncate(),
        )
    }

    /// Returns the rotation axis and angle of `self`.
    #[inline]
    pub fn to_axis_angle(self) -> (Vec3, f32) {
        const EPSILON: f32 = 1.0e-8;
        const EPSILON_SQUARED: f32 = EPSILON * EPSILON;
        let (x, y, z, w) = self.0.into();
        let angle = scalar_acos(w) * 2.0;
        let scale_sq = (1.0 - w * w).max(0.0);
        if scale_sq >= EPSILON_SQUARED {
            (Vec3::new(x, y, z) / scale_sq.sqrt(), angle)
        } else {
            (Vec3::unit_x(), angle)
        }
    }

    /// Returns the quaternion conjugate of `self`. For a unit quaternion the
    /// conjugate is also the inverse.
    #[inline]
    pub fn conjugate(self) -> Self {
        #[cfg(vec4sse2)]
        unsafe {
            Self(Vec4(_mm_xor_ps(
                (self.0).0,
                _mm_set_ps(0.0, -0.0, -0.0, -0.0),
            )))
        }

        #[cfg(vec4f32)]
        {
            Self::from_xyzw(-(self.0).0, -(self.0).1, -(self.0).2, (self.0).3)
        }
    }

    /// Computes the dot product of `self` and `other`. The dot product is
    /// equal to the the cosine of the angle between two quaterion rotations.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.0.dot(other.0)
    }

    /// Computes the length of `self`.
    #[inline]
    pub fn length(self) -> f32 {
        self.0.length()
    }

    /// Computes the squared length of `self`.
    ///
    /// This is generally faster than `Quat::length()` as it avoids a square
    /// root operation.
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.0.length_squared()
    }

    /// Computes `1.0 / Quat::length()`.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn length_reciprocal(self) -> f32 {
        1.0 / self.0.length()
    }

    /// Returns `self` normalized to length 1.0.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    pub fn normalize(self) -> Self {
        let inv_len = self.0.length_reciprocal();
        Self(self.0.mul(inv_len))
    }

    /// Returns whether `self` of length `1.0` or not.
    ///
    /// Uses a precision threshold of `1e-6`.
    #[inline]
    pub fn is_normalized(self) -> bool {
        is_normalized!(self)
    }

    #[inline]
    pub fn is_near_identity(self) -> bool {
        // from rtm quat_near_identity
        const THRESHOLD_ANGLE: f32 = 0.002_847_144_6;
        // Because of floating point precision, we cannot represent very small rotations.
        // The closest f32 to 1.0 that is not 1.0 itself yields:
        // 0.99999994.acos() * 2.0  = 0.000690533954 rad
        //
        // An error threshold of 1.e-6 is used by default.
        // (1.0 - 1.e-6).acos() * 2.0 = 0.00284714461 rad
        // (1.0 - 1.e-7).acos() * 2.0 = 0.00097656250 rad
        //
        // We don't really care about the angle value itself, only if it's close to 0.
        // This will happen whenever quat.w is close to 1.0.
        // If the quat.w is close to -1.0, the angle will be near 2*PI which is close to
        // a negative 0 rotation. By forcing quat.w to be positive, we'll end up with
        // the shortest path.
        let positive_w_angle = scalar_acos(self.0.w().abs()) * 2.0;
        positive_w_angle < THRESHOLD_ANGLE
    }

    /// Returns true if the absolute difference of all elements between `self`
    /// and `other` is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two `Quat`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(self, other: Self, max_abs_diff: f32) -> bool {
        self.0.abs_diff_eq(other.0, max_abs_diff)
    }

    /// Performs a linear interpolation between `self` and `other` based on
    /// the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `other`.
    #[inline]
    pub fn lerp(self, end: Self, s: f32) -> Self {
        glam_assert!(self.is_normalized());
        glam_assert!(end.is_normalized());

        #[cfg(vec4sse2)]
        unsafe {
            let start = self.0;
            let end = end.0;
            let dot = start.dot_as_vec4(end);
            // Calculate the bias, if the dot product is positive or zero, there is no bias
            // but if it is negative, we want to flip the 'end' rotation XYZW components
            let bias = _mm_and_ps(dot.into(), _mm_set_ps1(-0.0));
            let interpolated = Vec4(_mm_add_ps(
                _mm_mul_ps(
                    _mm_sub_ps(_mm_xor_ps(end.into(), bias), start.0),
                    _mm_set_ps1(s),
                ),
                start.0,
            ));
            Self(interpolated.normalize())
        }

        #[cfg(vec4f32)]
        {
            let start = self.0;
            let end = end.0;
            let dot = start.dot(end);
            let bias = if dot >= 0.0 { 1.0 } else { -1.0 };
            let interpolated = start + (s * ((end * bias) - start));
            Self(interpolated.normalize())
        }
    }

    /// Performs a spherical linear interpolation between `self` and `end`
    /// based on the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `end`.
    ///
    /// Note that a rotation can be represented by two quaternions: `q` and
    /// `-q`. The slerp path between `q` and `end` will be different from the
    /// path between `-q` and `end`. One path will take the long way around and
    /// one will take the short way. In order to correct for this, the `dot`
    /// product between `self` and `end` should be positive. If the `dot`
    /// product is negative, slerp between `-self` and `end`.
    #[inline]
    pub fn slerp(self, end: Self, s: f32) -> Self {
        // http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/

        glam_assert!(self.is_normalized());
        glam_assert!(end.is_normalized());

        const DOT_THRESHOLD: f32 = 0.9995;

        let dot = self.dot(end);

        if dot > DOT_THRESHOLD {
            // assumes lerp returns a normalized quaternion
            self.lerp(end, s)
        } else {
            #[cfg(vec4f32)]
            {
                // assumes scalar_acos clamps the input to [-1.0, 1.0]
                let theta = crate::f32::funcs::scalar_acos(dot);
                let scale1 = f32::sin(theta * (1.0 - s));
                let scale2 = f32::sin(theta * s);
                let theta_sin = f32::sin(theta);

                Quat((self.0 * scale1 + end.0 * scale2) * theta_sin.recip())
            }

            #[cfg(vec4sse2)]
            {
                // assumes scalar_acos clamps the input to [-1.0, 1.0]
                let theta = crate::f32::funcs::scalar_acos(dot);

                let x = 1.0 - s;
                let y = s;
                let z = 1.0;

                unsafe {
                    let tmp = Vec4::splat(theta) * Vec4::new(x, y, z, 0.0);
                    let tmp = crate::f32::funcs::sse2::m128_sin(tmp.0);

                    let scale1 = _mm_shuffle_ps(tmp, tmp, 0b00_00_00_00);
                    let scale2 = _mm_shuffle_ps(tmp, tmp, 0b01_01_01_01);
                    let theta_sin = _mm_shuffle_ps(tmp, tmp, 0b10_10_10_10);

                    let theta_sin_recip = Vec4(_mm_rcp_ps(theta_sin));

                    Quat((self.0 * Vec4(scale1) + end.0 * Vec4(scale2)) * theta_sin_recip)
                }
            }
        }
    }

    #[inline]
    /// Multiplies a quaternion and a 3D vector, rotating it.
    pub fn mul_vec3(self, other: Vec3) -> Vec3 {
        glam_assert!(self.is_normalized());

        #[cfg(vec4sse2)]
        {
            let w = self.0.dup_w().truncate();
            let two = Vec3::splat(2.0);
            let b = self.0.truncate();
            let b2 = b.dot_as_vec3(b);
            other * (w * w - b2) + b * (other.dot_as_vec3(b) * two) + b.cross(other) * (w * two)
        }

        #[cfg(vec4f32)]
        {
            let w = self.0.w();
            let b = self.0.truncate();
            let b2 = b.dot(b);
            other * (w * w - b2) + b * (other.dot(b) * 2.0) + b.cross(other) * (w * 2.0)
        }
    }

    #[inline]
    /// Multiplies two quaternions.
    /// Note that due to floating point rounding the result may not be perfectly normalized.
    pub fn mul_quat(self, other: Self) -> Self {
        glam_assert!(self.is_normalized());
        glam_assert!(other.is_normalized());

        #[cfg(vec4sse2)]
        unsafe {
            // from rtm quat_mul
            let lhs = self.0.into();
            let rhs = other.0.into();

            let control_wzyx = _mm_set_ps(-1.0, 1.0, -1.0, 1.0);
            let control_zwxy = _mm_set_ps(-1.0, -1.0, 1.0, 1.0);
            let control_yxwz = _mm_set_ps(-1.0, 1.0, 1.0, -1.0);

            let r_xxxx = _mm_shuffle_ps(lhs, lhs, 0b00_00_00_00);
            let r_yyyy = _mm_shuffle_ps(lhs, lhs, 0b01_01_01_01);
            let r_zzzz = _mm_shuffle_ps(lhs, lhs, 0b10_10_10_10);
            let r_wwww = _mm_shuffle_ps(lhs, lhs, 0b11_11_11_11);

            let lxrw_lyrw_lzrw_lwrw = _mm_mul_ps(r_wwww, rhs);
            let l_wzyx = _mm_shuffle_ps(rhs, rhs, 0b00_01_10_11);

            let lwrx_lzrx_lyrx_lxrx = _mm_mul_ps(r_xxxx, l_wzyx);
            let l_zwxy = _mm_shuffle_ps(l_wzyx, l_wzyx, 0b10_11_00_01);

            let lwrx_nlzrx_lyrx_nlxrx = _mm_mul_ps(lwrx_lzrx_lyrx_lxrx, control_wzyx);

            let lzry_lwry_lxry_lyry = _mm_mul_ps(r_yyyy, l_zwxy);
            let l_yxwz = _mm_shuffle_ps(l_zwxy, l_zwxy, 0b00_01_10_11);

            let lzry_lwry_nlxry_nlyry = _mm_mul_ps(lzry_lwry_lxry_lyry, control_zwxy);

            let lyrz_lxrz_lwrz_lzrz = _mm_mul_ps(r_zzzz, l_yxwz);
            let result0 = _mm_add_ps(lxrw_lyrw_lzrw_lwrw, lwrx_nlzrx_lyrx_nlxrx);

            let nlyrz_lxrz_lwrz_wlzrz = _mm_mul_ps(lyrz_lxrz_lwrz_lzrz, control_yxwz);
            let result1 = _mm_add_ps(lzry_lwry_nlxry_nlyry, nlyrz_lxrz_lwrz_wlzrz);
            Self(Vec4(_mm_add_ps(result0, result1)))
        }

        #[cfg(vec4f32)]
        {
            let (x0, y0, z0, w0) = self.0.into();
            let (x1, y1, z1, w1) = other.0.into();
            Self::from_xyzw(
                w0 * x1 + x0 * w1 + y0 * z1 - z0 * y1,
                w0 * y1 - x0 * z1 + y0 * w1 + z0 * x1,
                w0 * z1 + x0 * y1 - y0 * x1 + z0 * w1,
                w0 * w1 - x0 * x1 - y0 * y1 - z0 * z1,
            )
        }
    }
    /// Returns element `x`.
    #[inline]
    pub fn x(self) -> f32 {
        self.0.x()
    }

    /// Returns element `y`.
    #[inline]
    pub fn y(self) -> f32 {
        self.0.y()
    }

    /// Returns element `z`.
    #[inline]
    pub fn z(self) -> f32 {
        self.0.z()
    }

    /// Returns element `w`.
    #[inline]
    pub fn w(self) -> f32 {
        self.0.w()
    }
}

impl fmt::Debug for Quat {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(vec4sse2)]
        {
            fmt.debug_tuple("Quat").field(&(self.0).0).finish()
        }

        #[cfg(vec4f32)]
        {
            fmt.debug_tuple("Quat")
                .field(&self.0.x())
                .field(&self.0.y())
                .field(&self.0.z())
                .field(&self.0.w())
                .finish()
        }
    }
}

impl fmt::Display for Quat {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let (x, y, z, w) = self.0.into();
        write!(fmt, "[{}, {}, {}, {}]", x, y, z, w)
    }
}

impl Mul<Quat> for Quat {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        self.mul_quat(other)
    }
}

impl MulAssign<Quat> for Quat {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        *self = self.mul_quat(other);
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;
    #[inline]
    fn mul(self, other: Vec3) -> Vec3 {
        self.mul_vec3(other)
    }
}

impl Neg for Quat {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(-1.0 * self.0)
    }
}

impl Default for Quat {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl PartialEq for Quat {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.cmpeq(other.0).all()
    }
}

impl PartialOrd for Quat {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl AsRef<[f32; 4]> for Quat {
    #[inline]
    fn as_ref(&self) -> &[f32; 4] {
        self.0.as_ref()
    }
}

impl AsMut<[f32; 4]> for Quat {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32; 4] {
        self.0.as_mut()
    }
}

impl From<Vec4> for Quat {
    #[inline]
    fn from(v: Vec4) -> Self {
        Self(v)
    }
}

impl From<Quat> for Vec4 {
    #[inline]
    fn from(q: Quat) -> Self {
        q.0
    }
}

impl From<(f32, f32, f32, f32)> for Quat {
    #[inline]
    fn from(t: (f32, f32, f32, f32)) -> Self {
        Quat::from_xyzw(t.0, t.1, t.2, t.3)
    }
}

impl From<Quat> for (f32, f32, f32, f32) {
    #[inline]
    fn from(q: Quat) -> Self {
        q.0.into()
    }
}

impl From<[f32; 4]> for Quat {
    #[inline]
    fn from(a: [f32; 4]) -> Self {
        Self(a.into())
    }
}

impl From<Quat> for [f32; 4] {
    #[inline]
    fn from(q: Quat) -> Self {
        q.0.into()
    }
}

#[cfg(vec4sse2)]
impl From<Quat> for __m128 {
    // TODO: write test
    #[cfg_attr(tarpaulin, skip)]
    #[inline]
    fn from(q: Quat) -> Self {
        (q.0).0
    }
}

#[cfg(vec4sse2)]
impl From<__m128> for Quat {
    #[inline]
    fn from(t: __m128) -> Self {
        Self(Vec4(t))
    }
}
