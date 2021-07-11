use crate::{interpolation::utils::*, Quat, Vec2, Vec3, Vec3A, Vec4};

/// Defines how a particular type will be interpolated
pub trait Lerp: Sized {
    /// Lerp, `u` is unclamped
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self;

    /// Lerp, `u` is clamped in [0; 1] range
    fn lerp(value0: &Self, value1: &Self, u: f32) -> Self {
        let u = u.clamp(0.0, 1.0);
        Self::lerp_unclamped(value0, value1, u)
    }
}

impl Lerp for bool {
    #[inline]
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
        step_unclamped(value0, value1, u)
    }
}

impl Lerp for f32 {
    #[inline]
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
        (*value0) * (1.0 - u) + (*value1) * u
    }
}

impl Lerp for Vec2 {
    #[inline]
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
        (*value0) * (1.0 - u) + (*value1) * u
    }
}

/// **NOTE** Prefer [`Vec3A`] or [`Vec4`] whenever possible, using [`Vec3`] is 2 times slower
impl Lerp for Vec3 {
    #[inline]
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
        (*value0) * (1.0 - u) + (*value1) * u
    }
}

impl Lerp for Vec3A {
    #[inline]
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
        (*value0) * (1.0 - u) + (*value1) * u
    }
}

impl Lerp for Vec4 {
    #[inline]
    fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
        (*value0) * (1.0 - u) + (*value1) * u
    }
}

// TODO: Color can't be interpolated because color operations are undefined, see pr #1870
// impl Lerp for Color {
//     #[inline]
//     fn lerp_unclamped(value0: &Self, value1: &Self, u: f32) -> Self {
//         // ? NOTE: Make sure alpha is interpolated (pr #1870 Mul and Add doesn't include alpha)
//         (*value0) * (1.0 - t) + (*value1) * t
//     }
// }

impl Lerp for Quat {
    /// Performs an nlerp, because it's cheaper and easier to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, v: f32) -> Self {
        let mut b = *b;

        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        if a.dot(b) < 0.0 {
            b = -b;
        }

        let a: Vec4 = (*a).into();
        let b: Vec4 = b.into();

        let rot = Vec4::lerp_unclamped(&a, &b, v);
        let inv_mag = fast_inv_sqrt(rot.dot(rot));
        Quat::from_vec4(rot * inv_mag)
    }
}

impl<T: Lerp + Clone> Lerp for Option<T> {
    fn lerp_unclamped(a: &Self, b: &Self, v: f32) -> Self {
        match (a, b) {
            (Some(a), Some(b)) => Some(T::lerp_unclamped(a, b, v)),
            _ => step_unclamped(a, b, v), // change from `Some(T)` to `None` and vice versa
        }
    }
}
