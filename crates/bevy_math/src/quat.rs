// #![allow(unsafe_code)]
use core::fmt;
use std::arch::x86_64::__m128;
use std::fmt::{Debug, Display};
use std::iter::{Product, Sum};
use std::ops::{Add, Deref, DerefMut, Div, Mul, MulAssign, Neg, Sub};

use glam::{Affine3A, DQuat, EulerRot, Mat3, Mat3A, Mat4, Vec2, Vec3, Vec3A, Vec4};
use bevy_reflect::prelude::{Reflect, ReflectDefault};
// use bevy_reflect::{ApplyError, DynamicTypePath, FromReflect, GetTypeRegistration, ReflectMut, ReflectOwned, ReflectRef, Typed, TypeInfo, TypePath, TypeRegistration, ValueInfo};
// use bevy_reflect::utility::NonGenericTypeInfoCell;

#[derive(Clone, Copy, Reflect)]
#[reflect(Default)]
#[repr(C)]
pub struct QuatReprC {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// A quaternion representing an orientation.
///
/// This quaternion is intended to be of unit length but may denormalize due to
/// floating point "error creep" which can occur when successive quaternion
/// operations are applied.
///
/// SIMD vector types are used for storage on supported platforms unless repr_c feature flag is enabled.
/// SIMD types currently aren't compatible with FFI.
///
/// This type is 16 byte aligned.
#[derive(Clone, Copy, Reflect)]
#[reflect(Default)]
#[repr(transparent)]
pub struct Quat {
    #[cfg(feature = "repr_c")]
    inner: QuatReprC,
    #[cfg(not(feature = "repr_c"))]
    inner: glam::Quat,
}

impl Quat {
    /// All zeros.
    const ZERO: Self = Self::from_array([0.0; 4]);

    /// The identity quaternion. Corresponds to no rotation.
    pub const IDENTITY: Self = Self::from_xyzw(0.0, 0.0, 0.0, 1.0);

    /// All NANs.
    pub const NAN: Self = Self::from_array([f32::NAN; 4]);

    /// Creates a new rotation quaternion.
    ///
    /// This should generally not be called manually unless you know what you are doing.
    /// Use one of the other constructors instead such as `identity` or `from_axis_angle`.
    ///
    /// `from_xyzw` is mostly used by unit tests and `serde` deserialization.
    ///
    /// # Preconditions
    ///
    /// This function does not check if the input is normalized, it is up to the user to
    /// provide normalized input or to normalized the resulting quaternion.
    #[inline(always)]
    #[must_use]
    pub const fn from_xyzw(x: f32, y: f32, z: f32, w: f32) -> Self {
        #[cfg(feature = "repr_c")]
        return Self {
            inner: QuatReprC { x, y, z, w },
        };
        #[cfg(not(feature = "repr_c"))]
        return Self {
            inner: glam::Quat::from_xyzw(x, y, z, w),
        };
    }

    /// Creates a rotation quaternion from an array.
    ///
    /// # Preconditions
    ///
    /// This function does not check if the input is normalized, it is up to the user to
    /// provide normalized input or to normalized the resulting quaternion.
    #[inline]
    #[must_use]
    pub const fn from_array(a: [f32; 4]) -> Self {
        #[cfg(feature = "repr_c")]
        return Self {
            inner: QuatReprC {
                x: a[0],
                y: a[1],
                z: a[2],
                w: a[3],
            },
        };
        #[cfg(not(feature = "repr_c"))]
        return Self {
            inner: glam::Quat::from_xyzw(a[0], a[1], a[2], a[3]),
        };
    }

    /// Creates a new rotation quaternion from a 4D vector.
    ///
    /// # Preconditions
    ///
    /// This function does not check if the input is normalized, it is up to the user to
    /// provide normalized input or to normalized the resulting quaternion.
    #[inline]
    #[must_use]
    pub const fn from_vec4(v: Vec4) -> Self {
        let [x, y, z, w] = v.to_array();
        Self::from_xyzw(x, y, z, w)
    }

    /// Creates a rotation quaternion from a slice.
    ///
    /// # Preconditions
    ///
    /// This function does not check if the input is normalized, it is up to the user to
    /// provide normalized input or to normalized the resulting quaternion.
    ///
    /// # Panics
    ///
    /// Panics if `slice` length is less than 4.
    #[inline]
    #[must_use]
    pub fn from_slice(slice: &[f32]) -> Self {
        glam::Quat::from_slice(slice).into()
    }

    /// Writes the quaternion to an unaligned slice.
    ///
    /// # Panics
    ///
    /// Panics if `slice` length is less than 4.
    #[inline]
    pub fn write_to_slice(self, slice: &mut [f32]) {
        Into::<glam::Quat>::into(self).write_to_slice(slice)
    }

    /// Create a quaternion for a normalized rotation `axis` and `angle` (in radians).
    ///
    /// The axis must be a unit vector.
    ///
    /// # Panics
    ///
    /// Will panic if `axis` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        glam::Quat::from_axis_angle(axis, angle).into()
    }

    /// Create a quaternion that rotates `v.length()` radians around `v.normalize()`.
    ///
    /// `from_scaled_axis(Vec3::ZERO)` results in the identity quaternion.
    #[inline]
    #[must_use]
    pub fn from_scaled_axis(v: Vec3) -> Self {
        glam::Quat::from_scaled_axis(v).into()
    }

    /// Creates a quaternion from the `angle` (in radians) around the x axis.
    #[inline]
    #[must_use]
    pub fn from_rotation_x(angle: f32) -> Self {
        glam::Quat::from_rotation_x(angle).into()
    }

    /// Creates a quaternion from the `angle` (in radians) around the y axis.
    #[inline]
    #[must_use]
    pub fn from_rotation_y(angle: f32) -> Self {
        glam::Quat::from_rotation_y(angle).into()
    }

    /// Creates a quaternion from the `angle` (in radians) around the z axis.
    #[inline]
    #[must_use]
    pub fn from_rotation_z(angle: f32) -> Self {
        glam::Quat::from_rotation_z(angle).into()
    }

    /// Creates a quaternion from the given Euler rotation sequence and the angles (in radians).
    #[inline]
    #[must_use]
    pub fn from_euler(euler: EulerRot, a: f32, b: f32, c: f32) -> Self {
        glam::Quat::from_euler(euler, a, b, c).into()
    }

    /// Creates a quaternion from a 3x3 rotation matrix.
    #[inline]
    #[must_use]
    pub fn from_mat3(mat: &Mat3) -> Self {
        glam::Quat::from_mat3(mat).into()
    }

    /// Creates a 3D rotation matrix from the given quaternion.
    ///
    /// # Panics
    ///
    /// Will panic if `rotation` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn to_mat3(self) -> Mat3 {
        Mat3::from_quat(self.into())
    }

    /// Creates a quaternion from a 3x3 SIMD aligned rotation matrix.
    #[inline]
    #[must_use]
    pub fn from_mat3a(mat: &Mat3A) -> Self {
        glam::Quat::from_mat3a(mat).into()
    }

    /// Creates a 3D rotation matrix from the given quaternion.
    ///
    /// # Panics
    ///
    /// Will panic if `rotation` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn to_mat3a(self) -> Mat3A {
        Mat3A::from_quat(self.into())
    }

    /// Creates a quaternion from a 3x3 rotation matrix inside a homogeneous 4x4 matrix.
    #[inline]
    #[must_use]
    pub fn from_mat4(mat: &Mat4) -> Self {
        glam::Quat::from_mat4(mat).into()
    }

    /// Creates an affine transformation matrix from the given `rotation` quaternion.
    ///
    /// The resulting matrix can be used to transform 3D points and vectors. See
    /// [`Self::transform_point3()`] and [`Self::transform_vector3()`].
    ///
    /// # Panics
    ///
    /// Will panic if `rotation` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_quat(self.into())
    }

    /// Creates an affine transformation matrix from the given 3D `scale`, `rotation` and
    /// `translation`.
    ///
    /// The resulting matrix can be used to transform 3D points and vectors. See
    /// [`Self::transform_point3()`] and [`Self::transform_vector3()`].
    ///
    /// # Panics
    ///
    /// Will panic if `rotation` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn scale_rotation_translation_to_mat4(scale: Vec3, rotation: Quat, translation: Vec3) -> Mat4 {
        Mat4::from_scale_rotation_translation(scale, rotation.into(), translation)
    }

    /// Extracts `scale`, `rotation` and `translation` from `self`. The input matrix is
    /// expected to be a 3D affine transformation matrix otherwise the output will be invalid.
    ///
    /// # Panics
    ///
    /// Will panic if the determinant of `self` is zero or if the resulting scale vector
    /// contains any zero elements when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn mat4_to_scale_rotation_translation(mat: Mat4) -> (Vec3, Quat, Vec3) {
        let (scale, rotation, translation) = mat.to_scale_rotation_translation();
        (scale, rotation.into(), translation)
    }

    /// Creates an affine transform from the given 3D `scale`, `rotation` and
    /// `translation`.
    ///
    /// Equivalent to `Affine3A::from_translation(translation) *
    /// Affine3A::from_quat(rotation) * Affine3A::from_scale(scale)`
    #[inline]
    #[must_use]
    pub fn scale_rotation_translation_to_affine3a(scale: Vec3, rotation: Quat, translation: Vec3) -> Affine3A {
        Affine3A::from_scale_rotation_translation(
            scale,
            rotation.into(),
            translation,
        )
    }

    /// Creates an affine transform from the given 3D `rotation` and `translation`.
    ///
    /// Equivalent to `Affine3A::from_translation(translation) * Affine3A::from_quat(rotation)`
    #[inline]
    #[must_use]
    pub fn rotation_translation_to_affine3a(rotation: Quat, translation: Vec3) -> Affine3A {
        Affine3A::from_rotation_translation(rotation.into(), translation)
    }

    /// Extracts `scale`, `rotation` and `translation` from `self`.
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output
    /// will be invalid.
    ///
    /// # Panics
    ///
    /// Will panic if the determinant `self.matrix3` is zero or if the resulting scale
    /// vector contains any zero elements when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn affine3a_to_scale_rotation_translation(affine: Affine3A) -> (Vec3, Quat, Vec3) {
        let (scale, rotation, translation) = affine.to_scale_rotation_translation();
        (scale, rotation.into(), translation)
    }

    /// Gets the minimal rotation for transforming `from` to `to`.  The rotation is in the
    /// plane spanned by the two vectors.  Will rotate at most 180 degrees.
    ///
    /// The inputs must be unit vectors.
    ///
    /// `from_rotation_arc(from, to) * from ≈ to`.
    ///
    /// For near-singular cases (from≈to and from≈-to) the current implementation
    /// is only accurate to about 0.001 (for `f32`).
    ///
    /// # Panics
    ///
    /// Will panic if `from` or `to` are not normalized when `glam_assert` is enabled.
    #[must_use]
    pub fn from_rotation_arc(from: Vec3, to: Vec3) -> Self {
        glam::Quat::from_rotation_arc(from, to).into()
    }

    /// Gets the minimal rotation for transforming `from` to either `to` or `-to`.  This means
    /// that the resulting quaternion will rotate `from` so that it is colinear with `to`.
    ///
    /// The rotation is in the plane spanned by the two vectors.  Will rotate at most 90
    /// degrees.
    ///
    /// The inputs must be unit vectors.
    ///
    /// `to.dot(from_rotation_arc_colinear(from, to) * from).abs() ≈ 1`.
    ///
    /// # Panics
    ///
    /// Will panic if `from` or `to` are not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn from_rotation_arc_colinear(from: Vec3, to: Vec3) -> Self {
        glam::Quat::from_rotation_arc_colinear(from, to).into()
    }

    /// Gets the minimal rotation for transforming `from` to `to`.  The resulting rotation is
    /// around the z axis. Will rotate at most 180 degrees.
    ///
    /// The inputs must be unit vectors.
    ///
    /// `from_rotation_arc_2d(from, to) * from ≈ to`.
    ///
    /// For near-singular cases (from≈to and from≈-to) the current implementation
    /// is only accurate to about 0.001 (for `f32`).
    ///
    /// # Panics
    ///
    /// Will panic if `from` or `to` are not normalized when `glam_assert` is enabled.
    #[must_use]
    pub fn from_rotation_arc_2d(from: Vec2, to: Vec2) -> Self {
        glam::Quat::from_rotation_arc_2d(from, to).into()
    }

    /// Returns the rotation axis (normalized) and angle (in radians) of `self`.
    #[inline]
    #[must_use]
    pub fn to_axis_angle(self) -> (Vec3, f32) {
        Into::<glam::Quat>::into(self).to_axis_angle()
    }

    /// Returns the rotation axis scaled by the rotation in radians.
    #[inline]
    #[must_use]
    pub fn to_scaled_axis(self) -> Vec3 {
        Into::<glam::Quat>::into(self).to_scaled_axis()
    }

    /// Returns the rotation angles for the given euler rotation sequence.
    #[inline]
    #[must_use]
    pub fn to_euler(self, euler: EulerRot) -> (f32, f32, f32) {
        Into::<glam::Quat>::into(self).to_euler(euler)
    }

    /// `[x, y, z, w]`
    #[inline]
    #[must_use]
    pub fn to_array(&self) -> [f32; 4] {
        Into::<glam::Quat>::into(*self).to_array()
    }

    /// Returns the vector part of the quaternion.
    #[inline]
    #[must_use]
    pub fn xyz(self) -> Vec3 {
        Into::<glam::Quat>::into(self).xyz()
    }

    /// Returns the quaternion conjugate of `self`. For a unit quaternion the
    /// conjugate is also the inverse.
    #[inline]
    #[must_use]
    pub fn conjugate(self) -> Self {
        Into::<glam::Quat>::into(self).conjugate().into()
    }

    /// Returns the inverse of a normalized quaternion.
    ///
    /// Typically quaternion inverse returns the conjugate of a normalized quaternion.
    /// Because `self` is assumed to already be unit length this method *does not* normalize
    /// before returning the conjugate.
    ///
    /// # Panics
    ///
    /// Will panic if `self` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn inverse(self) -> Self {
        Into::<glam::Quat>::into(self).inverse().into()
    }

    /// Computes the dot product of `self` and `rhs`. The dot product is
    /// equal to the cosine of the angle between two quaternion rotations.
    #[inline]
    #[must_use]
    pub fn dot(self, rhs: Self) -> f32 {
        Into::<glam::Quat>::into(self).dot(Into::<glam::Quat>::into(rhs))
    }

    /// Computes the length of `self`.
    #[doc(alias = "magnitude")]
    #[inline]
    #[must_use]
    pub fn length(self) -> f32 {
        Into::<glam::Quat>::into(self).length()
    }

    /// Computes the squared length of `self`.
    ///
    /// This is generally faster than `length()` as it avoids a square
    /// root operation.
    #[doc(alias = "magnitude2")]
    #[inline]
    #[must_use]
    pub fn length_squared(self) -> f32 {
        Into::<glam::Quat>::into(self).length_squared()
    }

    /// Computes `1.0 / length()`.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    #[inline]
    #[must_use]
    pub fn length_recip(self) -> f32 {
        Into::<glam::Quat>::into(self).length_recip()
    }

    /// Returns `self` normalized to length 1.0.
    ///
    /// For valid results, `self` must _not_ be of length zero.
    ///
    /// Panics
    ///
    /// Will panic if `self` is zero length when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn normalize(self) -> Self {
        Into::<glam::Quat>::into(self).normalize().into()
    }

    /// Returns `true` if, and only if, all elements are finite.
    /// If any element is either `NaN`, positive or negative infinity, this will return `false`.
    #[inline]
    #[must_use]
    pub fn is_finite(self) -> bool {
        Into::<glam::Quat>::into(self).is_finite()
    }

    /// Returns a vector with elements representing the sign of `self`.
    ///
    /// - `1.0` if the number is positive, `+0.0` or `INFINITY`
    /// - `-1.0` if the number is negative, `-0.0` or `NEG_INFINITY`
    /// - `NAN` if the number is `NAN`
    #[inline]
    #[must_use]
    pub fn is_nan(self) -> bool {
        Into::<glam::Quat>::into(self).is_nan()
    }

    /// Returns whether `self` of length `1.0` or not.
    ///
    /// Uses a precision threshold of `1e-6`.
    #[inline]
    #[must_use]
    pub fn is_normalized(self) -> bool {
        Into::<glam::Quat>::into(self).is_normalized()
    }

    /// Checks if the quaternion is near the identity rotation.
    ///
    /// # Details
    ///
    /// This function determines if the quaternion represents a rotation that is
    /// very close to the identity rotation (no rotation). Due to floating point
    /// precision limitations, very small rotations cannot be accurately represented.
    ///
    /// The threshold for considering a quaternion as near identity is set to
    /// `0.0028471446` radians. This threshold is based on the closest value to
    /// `1.0` that a `f32` can represent which is not `1.0` itself:
    ///
    /// ```text
    /// 0.99999994.acos() * 2.0 = 0.000690533954 rad
    /// ```
    ///
    /// An error threshold of `1.e-6` is used by default:
    ///
    /// ```text
    /// (1.0 - 1.e-6).acos() * 2.0 = 0.00284714461 rad
    /// (1.0 - 1.e-7).acos() * 2.0 = 0.00097656250 rad
    /// ```
    ///
    /// The function calculates the angle based on the w-component of the quaternion
    /// and compares it to the threshold. The absolute value of `quat.w` is taken
    /// to ensure the shortest path is considered, as `quat.w` close to `-1.0`
    /// would indicate a near 2*PI rotation, which is essentially a negative zero rotation.
    ///
    /// # Returns
    ///
    /// * `true` if the quaternion is near the identity rotation.
    /// * `false` otherwise.
    ///
    /// # References
    ///
    /// This implementation is based on the algorithm from the [`rtm`](https://github.com/nfrechette/rtm)
    /// library's `rtm::quat_near_identity` function.
    #[inline]
    #[must_use]
    pub fn is_near_identity(self) -> bool {
        Into::<glam::Quat>::into(self).is_near_identity()
    }

    /// Returns the angle (in radians) for the minimal rotation
    /// for transforming this quaternion into another.
    ///
    /// Both quaternions must be normalized.
    ///
    /// # Panics
    ///
    /// Will panic if `self` or `rhs` are not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn angle_between(self, rhs: Self) -> f32 {
        Into::<glam::Quat>::into(self).angle_between(Into::<glam::Quat>::into(rhs))
    }

    /// Returns true if the absolute difference of all elements between `self` and `rhs`
    /// is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two quaternions contain similar elements. It works
    /// best when comparing with a known value. The `max_abs_diff` that should be used used
    /// depends on the values being compared against.
    ///
    /// For more see
    /// [comparing floating point numbers](https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/).
    #[inline]
    #[must_use]
    pub fn abs_diff_eq(self, rhs: Self, max_abs_diff: f32) -> bool {
        Into::<glam::Quat>::into(self).abs_diff_eq(Into::<glam::Quat>::into(rhs), max_abs_diff)
    }

    /// Performs a linear interpolation between `self` and `rhs` based on
    /// the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `rhs`.
    ///
    /// # Panics
    ///
    /// Will panic if `self` or `end` are not normalized when `glam_assert` is enabled.
    #[doc(alias = "mix")]
    #[inline]
    #[must_use]
    pub fn lerp(self, end: Self, s: f32) -> Self {
        Into::<glam::Quat>::into(self).lerp(Into::<glam::Quat>::into(end), s).into()
    }

    /// Performs a spherical linear interpolation between `self` and `end`
    /// based on the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s`
    /// is `1.0`, the result will be equal to `end`.
    ///
    /// # Panics
    ///
    /// Will panic if `self` or `end` are not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    #[allow(unused_mut)]
    pub fn slerp(self, mut end: Self, s: f32) -> Self {
        Into::<glam::Quat>::into(self).slerp(end.into(), s).into()
    }

    /// Multiplies a quaternion and a 3D vector, returning the rotated vector.
    ///
    /// # Panics
    ///
    /// Will panic if `self` is not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn mul_vec3(self, rhs: Vec3) -> Vec3 {
        Into::<glam::Quat>::into(self).mul_vec3(rhs)
    }

    /// Multiplies two quaternions. If they each represent a rotation, the result will
    /// represent the combined rotation.
    ///
    /// Note that due to floating point rounding the result may not be perfectly normalized.
    ///
    /// # Panics
    ///
    /// Will panic if `self` or `rhs` are not normalized when `glam_assert` is enabled.
    #[inline]
    #[must_use]
    pub fn mul_quat(self, rhs: Self) -> Self {
        Into::<glam::Quat>::into(self).mul_quat(Into::<glam::Quat>::into(rhs)).into()
    }

    /// Creates a quaternion from a 3x3 rotation matrix inside a 3D affine transform.
    #[inline]
    #[must_use]
    pub fn from_affine3(a: &Affine3A) -> Self {
        glam::Quat::from_affine3(a).into()
    }

    /// Multiplies a quaternion and a 3D vector, returning the rotated vector.
    #[inline]
    #[must_use]
    pub fn mul_vec3a(self, rhs: Vec3A) -> Vec3A {
        Into::<glam::Quat>::into(self).mul_vec3a(rhs)
    }

    /// Creates a new rotation quaternion.
    ///
    /// This should generally not be called manually unless you know what you are doing.
    /// Use one of the other constructors instead such as `identity` or `from_axis_angle`.
    ///
    /// `from_xyzw` is mostly used by unit tests and `serde` deserialization.
    ///
    /// # Preconditions
    ///
    /// This function does not check if the input is normalized, it is up to the user to
    /// provide normalized input or to normalized the resulting quaternion.
    #[inline]
    #[must_use]
    pub fn as_dquat(self) -> DQuat {
        Into::<glam::Quat>::into(self).as_dquat()
    }
}

impl Debug for Quat {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&Into::<glam::Quat>::into(*self), fmt)
    }
}

impl Display for Quat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&Into::<glam::Quat>::into(*self), f)
    }
}

impl Add<Quat> for Quat {
    type Output = Self;
    /// Adds two quaternions.
    ///
    /// The sum is not guaranteed to be normalized.
    ///
    /// Note that addition is not the same as combining the rotations represented by the
    /// two quaternions! That corresponds to multiplication.
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Into::<glam::Quat>::into(self).add(Into::<glam::Quat>::into(rhs)).into()
    }
}

impl Sub<Quat> for Quat {
    type Output = Self;
    /// Subtracts the `rhs` quaternion from `self`.
    ///
    /// The difference is not guaranteed to be normalized.
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Into::<glam::Quat>::into(self).sub(Into::<glam::Quat>::into(rhs)).into()
    }
}

impl Mul<f32> for Quat {
    type Output = Self;
    /// Multiplies a quaternion by a scalar value.
    ///
    /// The product is not guaranteed to be normalized.
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Into::<glam::Quat>::into(self).mul(rhs).into()
    }
}

impl Div<f32> for Quat {
    type Output = Self;
    /// Divides a quaternion by a scalar value.
    /// The quotient is not guaranteed to be normalized.
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Into::<glam::Quat>::into(self).div(rhs).into()
    }
}

impl Mul<Quat> for Quat {
    type Output = Self;
    /// Multiplies two quaternions. If they each represent a rotation, the result will
    /// represent the combined rotation.
    ///
    /// Note that due to floating point rounding the result may not be perfectly
    /// normalized.
    ///
    /// # Panics
    ///
    /// Will panic if `self` or `rhs` are not normalized when `glam_assert` is enabled.
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Into::<glam::Quat>::into(self).mul(Into::<glam::Quat>::into(rhs)).into()
    }
}

impl MulAssign<Quat> for Quat {
    /// Multiplies two quaternions. If they each represent a rotation, the result will
    /// represent the combined rotation.
    ///
    /// Note that due to floating point rounding the result may not be perfectly
    /// normalized.
    ///
    /// # Panics
    ///
    /// Will panic if `self` or `rhs` are not normalized when `glam_assert` is enabled.
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = Into::<glam::Quat>::into(*self).mul(Into::<glam::Quat>::into(rhs)).into()
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;
    /// Multiplies a quaternion and a 3D vector, returning the rotated vector.
    ///
    /// # Panics
    ///
    /// Will panic if `self` is not normalized when `glam_assert` is enabled.
    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        self.mul_vec3(rhs)
    }
}

impl Neg for Quat {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Into::<glam::Quat>::into(self).neg().into()
    }
}

impl Default for Quat {
    #[inline]
    fn default() -> Self {
        glam::Quat::default().into()
    }
}

impl PartialEq for Quat {
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        Into::<glam::Quat>::into(*self) == Into::<glam::Quat>::into(*rhs)
    }
}

impl Sum<Self> for Quat {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::ZERO, Self::add)
    }
}

impl<'a> Sum<&'a Self> for Quat {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Self::ZERO, |a, &b| Self::add(a, b))
    }
}

impl Product for Quat {
    fn product<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::IDENTITY, Self::mul)
    }
}

impl<'a> Product<&'a Self> for Quat {
    fn product<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Self::IDENTITY, |a, &b| Self::mul(a, b))
    }
}

impl Mul<Vec3A> for Quat {
    type Output = Vec3A;
    #[inline]
    fn mul(self, rhs: Vec3A) -> Self::Output {
        self.mul_vec3a(rhs)
    }
}

impl From<Quat> for Vec4 {
    #[inline]
    fn from(q: Quat) -> Self {
        Into::<glam::Quat>::into(q).into()
    }
}

impl From<Quat> for (f32, f32, f32, f32) {
    #[inline]
    fn from(q: Quat) -> Self {
        Into::<glam::Quat>::into(q).into()
    }
}

impl From<Quat> for [f32; 4] {
    #[inline]
    fn from(q: Quat) -> Self {
        Into::<glam::Quat>::into(q).into()
    }
}

impl From<Quat> for __m128 {
    #[inline]
    fn from(q: Quat) -> Self {
        Into::<glam::Quat>::into(q).into()
    }
}

impl Into<glam::Quat> for QuatReprC {
    fn into(self) -> glam::Quat {
        glam::Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

#[cfg(feature = "repr_c")]
impl Into<Quat> for QuatReprC {
    fn into(self) -> Quat {
        Quat {
            inner: QuatReprC {
                x: self.x,
                y: self.y,
                z: self.z,
                w: self.w,
            },
        }
    }
}

impl Into<Quat> for glam::Quat {
    fn into(self) -> Quat {
        #[cfg(feature = "repr_c")]
        let inner = QuatReprC {
            x: self.x,
            y: self.y,
            z: self.z,
            w: self.w,
        };
        #[cfg(not(feature = "repr_c"))]
        let inner = glam::Quat::from_xyzw(self.x, self.y, self.z, self.w);
        Quat {
            inner
        }
    }
}

impl From<glam::Quat> for QuatReprC {
    fn from(q: glam::Quat) -> Self {
        QuatReprC {
            x: q.x,
            y: q.y,
            z: q.z,
            w: q.w,
        }
    }
}

impl From<Quat> for glam::Quat {
    fn from(q: Quat) -> Self {
        glam::Quat::from_xyzw(q.x, q.y, q.z, q.w)
    }
}

impl Default for QuatReprC {
    fn default() -> Self {
        glam::Quat::default().into()
    }
}

impl Deref for Quat {
    #[cfg(feature = "repr_c")]
    type Target = QuatReprC;
    #[cfg(not(feature = "repr_c"))]
    type Target = glam::Quat;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Quat {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}