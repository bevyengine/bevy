use crate::{interpolation::utils::*, Quat, Vec2, Vec3, Vec3A, Vec4};

/// Defines how a particular type will be interpolated
pub trait Lerp: Sized {
    /// Lerp, `t` is unclamped
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self;

    /// Lerp, `t` is clamped in [0; 1] range
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::lerp_unclamped(a, b, t)
    }
}

impl Lerp for bool {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        if t > 0.99 {
            *b
        } else {
            *a
        }
    }
}

impl Lerp for f32 {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec2 {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

/// **NOTE** Prefer [`Vec3A`] or [`Vec4`] whenever possible, using [`Vec3`] is 2 times slower
impl Lerp for Vec3 {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec3A {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec4 {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

// TODO: Color can't be interpolated because color operations are undefined, see pr #1870
// impl Lerp for Color {
//     #[inline]
//     fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
//         // ? NOTE: Make sure alpha is interpolated (pr #1870 Mul and Add doesn't include alpha)
//         (*a) * (1.0 - t) + (*b) * t
//     }
// }

impl Lerp for Quat {
    /// Performs an nlerp, because it's cheaper and easier to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        let mut b = *b;

        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        if a.dot(b) < 0.0 {
            b = -b;
        }

        let a: Vec4 = (*a).into();
        let b: Vec4 = b.into();

        let rot = Vec4::lerp_unclamped(&a, &b, t);
        let inv_mag = fast_inv_sqrt(rot.dot(rot));
        (rot * inv_mag).into()
    }
}

impl<T: Lerp + Clone> Lerp for Option<T> {
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        match (a, b) {
            (Some(a), Some(b)) => Some(T::lerp_unclamped(a, b, t)),
            _ => step_unclamped(a, b, t), // change from `Some(T)` to `None` and vice versa
        }
    }
}
