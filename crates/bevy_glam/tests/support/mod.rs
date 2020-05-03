#[macro_use]
mod macros;

use glam::{Mat2, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

#[cfg(feature = "transform-types")]
use glam::{TransformRT, TransformSRT};

/// Helper function for migrating away from `glam::angle::deg`.
#[allow(dead_code)]
#[inline]
pub fn deg(angle: f32) -> f32 {
    angle.to_radians()
}

/// Helper function for migrating away from `glam::angle::rad`.
#[allow(dead_code)]
#[inline]
pub fn rad(angle: f32) -> f32 {
    angle
}

/// Trait used by the `assert_approx_eq` macro for floating point comparisons.
pub trait FloatCompare<Rhs: ?Sized = Self> {
    /// Return true if the absolute difference between `self` and `other` is
    /// less then or equal to `max_abs_diff`.
    fn approx_eq(&self, other: &Rhs, max_abs_diff: f32) -> bool;
    /// Returns the absolute difference of `self` and `other` which is printed
    /// if `assert_approx_eq` fails.
    fn abs_diff(&self, other: &Rhs) -> Rhs;
}

impl FloatCompare for f32 {
    #[inline]
    fn approx_eq(&self, other: &f32, max_abs_diff: f32) -> bool {
        (self - other).abs() <= max_abs_diff
    }
    #[inline]
    fn abs_diff(&self, other: &f32) -> f32 {
        (self - other).abs()
    }
}

impl FloatCompare for Mat2 {
    #[inline]
    fn approx_eq(&self, other: &Mat2, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Mat2) -> Mat2 {
        Mat2::from_cols(
            (self.x_axis() - other.x_axis()).abs(),
            (self.y_axis() - other.y_axis()).abs(),
        )
    }
}

impl FloatCompare for Mat3 {
    #[inline]
    fn approx_eq(&self, other: &Mat3, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Mat3) -> Mat3 {
        Mat3::from_cols(
            (self.x_axis() - other.x_axis()).abs(),
            (self.y_axis() - other.y_axis()).abs(),
            (self.z_axis() - other.z_axis()).abs(),
        )
    }
}

impl FloatCompare for Mat4 {
    #[inline]
    fn approx_eq(&self, other: &Mat4, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Mat4) -> Mat4 {
        Mat4::from_cols(
            (self.x_axis() - other.x_axis()).abs(),
            (self.y_axis() - other.y_axis()).abs(),
            (self.z_axis() - other.z_axis()).abs(),
            (self.w_axis() - other.w_axis()).abs(),
        )
    }
}

impl FloatCompare for Quat {
    #[inline]
    fn approx_eq(&self, other: &Quat, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Quat) -> Quat {
        let a: Vec4 = (*self).into();
        let b: Vec4 = (*other).into();
        (a - b).abs().into()
    }
}

impl FloatCompare for Vec2 {
    #[inline]
    fn approx_eq(&self, other: &Vec2, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Vec2) -> Vec2 {
        (*self - *other).abs()
    }
}

impl FloatCompare for Vec3 {
    #[inline]
    fn approx_eq(&self, other: &Vec3, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Vec3) -> Vec3 {
        (*self - *other).abs()
    }
}

impl FloatCompare for Vec4 {
    #[inline]
    fn approx_eq(&self, other: &Vec4, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }
    #[inline]
    fn abs_diff(&self, other: &Vec4) -> Vec4 {
        (*self - *other).abs()
    }
}

#[cfg(feature = "transform-types")]
impl FloatCompare for TransformSRT {
    #[inline]
    fn approx_eq(&self, other: &Self, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }

    #[inline]
    fn abs_diff(&self, other: &Self) -> Self {
        Self::from_scale_rotation_translation(
            self.scale.abs_diff(&other.scale),
            self.rotation.abs_diff(&other.rotation),
            self.translation.abs_diff(&other.translation),
        )
    }
}

#[cfg(feature = "transform-types")]
impl FloatCompare for TransformRT {
    #[inline]
    fn approx_eq(&self, other: &Self, max_abs_diff: f32) -> bool {
        self.abs_diff_eq(*other, max_abs_diff)
    }

    #[inline]
    fn abs_diff(&self, other: &Self) -> Self {
        Self::from_rotation_translation(
            self.rotation.abs_diff(&other.rotation),
            self.translation.abs_diff(&other.translation),
        )
    }
}
