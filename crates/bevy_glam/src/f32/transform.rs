use super::{Mat4, Quat, Vec3};
use core::ops::Mul;

#[cfg(feature = "rand")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
#[repr(C)]
pub struct TransformSRT {
    pub scale: Vec3,
    pub rotation: Quat,
    pub translation: Vec3,
}

impl Default for TransformSRT {
    #[inline]
    fn default() -> Self {
        Self {
            scale: Vec3::one(),
            rotation: Quat::identity(),
            translation: Vec3::zero(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
#[repr(C)]
pub struct TransformRT {
    pub rotation: Quat,
    pub translation: Vec3,
}

impl Default for TransformRT {
    #[inline]
    fn default() -> Self {
        Self {
            rotation: Quat::identity(),
            translation: Vec3::zero(),
        }
    }
}

impl TransformSRT {
    #[inline]
    pub fn from_scale_rotation_translation(scale: Vec3, rotation: Quat, translation: Vec3) -> Self {
        Self {
            scale,
            rotation,
            translation,
        }
    }

    #[inline]
    pub fn from_transform_rt(scale: Vec3, rt: &TransformRT) -> Self {
        Self {
            scale,
            rotation: rt.rotation,
            translation: rt.translation,
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self {
            scale: Vec3::one(),
            rotation: Quat::identity(),
            translation: Vec3::zero(),
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        let scale = self.scale.reciprocal();
        let rotation = self.rotation.conjugate();
        let translation = -(rotation * (self.translation * scale));
        Self {
            scale,
            rotation,
            translation,
        }
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let rotation = self.rotation.normalize();
        Self {
            scale: self.scale,
            rotation,
            translation: self.translation,
        }
    }

    #[inline]
    pub fn mul_transform(&self, other: &Self) -> Self {
        mul_srt_srt(self, other)
    }

    #[inline]
    pub fn transform_vec3(self, other: Vec3) -> Vec3 {
        (self.rotation * (other * self.scale)) + self.translation
    }

    /// Returns true if the absolute difference of all elements between `self`
    /// and `other` is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two `Mat4`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(&self, other: Self, max_abs_diff: f32) -> bool {
        self.scale.abs_diff_eq(other.scale, max_abs_diff)
            && self.rotation.abs_diff_eq(other.rotation, max_abs_diff)
            && self
                .translation
                .abs_diff_eq(other.translation, max_abs_diff)
    }
}

#[inline]
fn mul_srt_srt(lhs: &TransformSRT, rhs: &TransformSRT) -> TransformSRT {
    // from rtm qvv_mul
    let min_scale = lhs.scale.min(rhs.scale);
    let scale = lhs.scale * rhs.scale;

    if min_scale.cmplt(Vec3::zero()).any() {
        // If negative scale, we go through a matrix
        let lhs_mtx =
            Mat4::from_scale_rotation_translation(lhs.scale, lhs.rotation, lhs.translation);
        let rhs_mtx =
            Mat4::from_scale_rotation_translation(rhs.scale, rhs.rotation, rhs.translation);
        let mut result_mtx = lhs_mtx * rhs_mtx;

        let sign = scale.sign();
        result_mtx
            .set_x_axis((result_mtx.x_axis().truncate().normalize() * sign.dup_x()).extend(0.0));
        result_mtx
            .set_y_axis((result_mtx.y_axis().truncate().normalize() * sign.dup_y()).extend(0.0));
        result_mtx
            .set_z_axis((result_mtx.z_axis().truncate().normalize() * sign.dup_z()).extend(0.0));

        let rotation = Quat::from_rotation_mat4(&result_mtx);
        let translation = result_mtx.w_axis().truncate();
        TransformSRT {
            scale,
            rotation,
            translation,
        }
    } else {
        let rotation = lhs.rotation * rhs.rotation;
        let translation = (rhs.rotation * (lhs.translation * rhs.scale)) + rhs.translation;
        TransformSRT {
            scale,
            rotation,
            translation,
        }
    }
}

#[inline]
fn mul_rt_rt(lhs: &TransformRT, rhs: &TransformRT) -> TransformRT {
    let rotation = lhs.rotation * rhs.rotation;
    let translation = (rhs.rotation * lhs.translation) + rhs.translation;
    TransformRT {
        rotation,
        translation,
    }
}

impl TransformRT {
    #[inline]
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        Self {
            rotation,
            translation,
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self {
            rotation: Quat::identity(),
            translation: Vec3::zero(),
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        let rotation = self.rotation.conjugate();
        let translation = -(rotation * self.translation);
        Self {
            rotation,
            translation,
        }
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let rotation = self.rotation.normalize();
        Self {
            rotation,
            translation: self.translation,
        }
    }

    #[inline]
    pub fn mul_transform(&self, other: &Self) -> Self {
        mul_rt_rt(self, other)
    }

    #[inline]
    pub fn transform_vec3(self, other: Vec3) -> Vec3 {
        (self.rotation * other) + self.translation
    }

    /// Returns true if the absolute difference of all elements between `self`
    /// and `other` is less than or equal to `max_abs_diff`.
    ///
    /// This can be used to compare if two `Mat4`'s contain similar elements. It
    /// works best when comparing with a known value. The `max_abs_diff` that
    /// should be used used depends on the values being compared against.
    ///
    /// For more on floating point comparisons see
    /// https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/
    #[inline]
    pub fn abs_diff_eq(&self, other: Self, max_abs_diff: f32) -> bool {
        self.rotation.abs_diff_eq(other.rotation, max_abs_diff)
            && self
                .translation
                .abs_diff_eq(other.translation, max_abs_diff)
    }
}

impl AsRef<TransformRT> for TransformSRT {
    #[inline]
    fn as_ref(&self) -> &TransformRT {
        unsafe { &*(self as *const Self as *const TransformRT) }
    }
}

impl AsMut<TransformRT> for TransformSRT {
    #[inline]
    fn as_mut(&mut self) -> &mut TransformRT {
        unsafe { &mut *(self as *mut Self as *mut TransformRT) }
    }
}

impl Mul<Vec3> for TransformRT {
    type Output = Vec3;
    #[inline]
    fn mul(self, other: Vec3) -> Vec3 {
        self.transform_vec3(other)
    }
}

impl Mul<Vec3> for TransformSRT {
    type Output = Vec3;
    #[inline]
    fn mul(self, other: Vec3) -> Vec3 {
        self.transform_vec3(other)
    }
}

impl Mul<TransformRT> for TransformRT {
    type Output = TransformRT;
    #[inline]
    fn mul(self, other: TransformRT) -> TransformRT {
        mul_rt_rt(&self, &other)
    }
}

impl Mul<TransformSRT> for TransformSRT {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self::Output {
        mul_srt_srt(&self, &other)
    }
}

impl Mul<TransformRT> for TransformSRT {
    type Output = TransformSRT;
    #[inline]
    fn mul(self, other: TransformRT) -> Self::Output {
        mul_srt_srt(&self, &other.into())
    }
}

impl Mul<TransformSRT> for TransformRT {
    type Output = TransformSRT;
    #[inline]
    fn mul(self, other: TransformSRT) -> Self::Output {
        mul_srt_srt(&self.into(), &other)
    }
}

impl From<TransformRT> for TransformSRT {
    #[inline]
    fn from(tr: TransformRT) -> Self {
        Self {
            translation: tr.translation,
            rotation: tr.rotation,
            scale: Vec3::one(),
        }
    }
}

#[cfg(feature = "rand")]
impl Distribution<TransformRT> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TransformRT {
        TransformRT::from_rotation_translation(
            rng.gen::<Quat>(),
            Vec3::new(
                rng.gen_range(core::f32::MIN, core::f32::MAX),
                rng.gen_range(core::f32::MIN, core::f32::MAX),
                rng.gen_range(core::f32::MIN, core::f32::MAX),
            ),
        )
    }
}

#[cfg(feature = "rand")]
impl Distribution<TransformSRT> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TransformSRT {
        let mut gen_non_zero = || loop {
            let f: f32 = rng.gen_range(core::f32::MIN, core::f32::MAX);
            if f.abs() > core::f32::MIN_POSITIVE {
                return f;
            }
        };
        TransformSRT::from_scale_rotation_translation(
            Vec3::new(gen_non_zero(), gen_non_zero(), gen_non_zero()),
            rng.gen::<Quat>(),
            Vec3::new(
                rng.gen_range(core::f32::MIN, core::f32::MAX),
                rng.gen_range(core::f32::MIN, core::f32::MAX),
                rng.gen_range(core::f32::MIN, core::f32::MAX),
            ),
        )
    }
}

impl From<TransformSRT> for Mat4 {
    #[inline]
    fn from(srt: TransformSRT) -> Self {
        Mat4::from_scale_rotation_translation(srt.scale, srt.rotation, srt.translation)
    }
}

impl From<TransformRT> for Mat4 {
    #[inline]
    fn from(rt: TransformRT) -> Self {
        Mat4::from_rotation_translation(rt.rotation, rt.translation)
    }
}
