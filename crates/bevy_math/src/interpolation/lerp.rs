use crate::{interpolation::utils::*, Quat, Vec2, Vec3, Vec4};

/// Defines how a particular type will be interpolated
pub trait Lerp: Sized {
    /// Lerp, `t` is unclamped
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self;

    /// Lerp, `t` is clamped in [0; 1] range
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let t = t.min(1.0).max(0.0);
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

impl Lerp for Vec3 {
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

// impl Lerp for Color {
//     #[inline]
//     fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
//         (*a) * (1.0 - t) + (*b) * t
//     }
// }

impl Lerp for Quat {
    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
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
        let inv_mag = inv_sqrt(rot.dot(rot));
        (rot * inv_mag).into()
    }
}

impl<T: Lerp + Clone> Lerp for Option<T> {
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        match (a.is_some(), b.is_some()) {
            (true, true) => Some(T::lerp_unclamped(
                a.as_ref().unwrap(),
                b.as_ref().unwrap(),
                t,
            )),
            (false, true) | (true, false) | (false, false) => {
                if t > 0.99 {
                    b.clone()
                } else {
                    a.clone()
                }
            }
        }
    }
}
