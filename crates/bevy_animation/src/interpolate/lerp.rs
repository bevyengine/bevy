use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;
use bevy_render::color::Color;
use ultraviolet::{f32x4, f32x8, Vec3x4, Vec3x8, Vec4x4, Vec4x8};

use super::utils::*;

// TODO: add Rotors?

/// Defines how a particular type will be interpolated
pub trait Lerp {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

impl Lerp for bool {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        if t > 0.99 {
            b.clone()
        } else {
            a.clone()
        }
    }
}

impl Lerp for f32 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec2 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec3 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec3x4 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * f32x4::splat(1.0 - t) + (*b) * f32x4::splat(t)
    }
}

impl Lerp for Vec3x8 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * f32x8::splat(1.0 - t) + (*b) * f32x8::splat(t)
    }
}

impl Lerp for Vec4 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Vec4x4 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * f32x4::splat(1.0 - t) + (*b) * f32x4::splat(t)
    }
}

impl Lerp for Vec4x8 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * f32x8::splat(1.0 - t) + (*b) * f32x8::splat(t)
    }
}

impl Lerp for Color {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Lerp for Quat {
    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let mut b = *b;

        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        if a.dot(b) < 0.0 {
            b = -b;
        }

        let q = Vec4::lerp((*a).into(), b.into(), t);
        let d = inv_sqrt(q.dot(q));
        (q * d).into()
    }
}

impl Lerp for Quatx4 {
    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let mut b = b.0;

        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        // Flip b sing if dot product was negative
        let sign: f32x4 = a.0.dot(b) & f32x4::splat(-0.0);
        b.x = b.x ^ sign;
        b.y = b.y ^ sign;
        b.z = b.z ^ sign;
        b.w = b.w ^ sign;

        let q = Vec4x4::lerp(&a.0, &b, t);
        let d = inv_sqrt4(q.dot(q));
        Quatx4(q * d)
    }
}

impl Lerp for Quatx8 {
    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let mut b = b.0;

        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        // Flip b sing if dot product was negative
        let sign: f32x8 = a.0.dot(b) & f32x8::splat(-0.0);
        b.x = b.x ^ sign;
        b.y = b.y ^ sign;
        b.z = b.z ^ sign;
        b.w = b.w ^ sign;

        let q = Vec4x8::lerp(&a.0, &b, t);
        let d = inv_sqrt8(q.dot(q));
        Quatx8(q * d)
    }
}

impl Lerp for Scale2 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Scale2(Vec2::lerp(a.0, b.0, t))
    }
}

impl Lerp for Scale3 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Scale3(Vec3::lerp(a.0, b.0, t))
    }
}

impl<T: Asset + 'static> Lerp for Handle<T> {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        if t > 0.99 {
            b.clone()
        } else {
            a.clone()
        }
    }
}

impl Lerp for HandleUntyped {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        if t > 0.99 {
            b.clone()
        } else {
            a.clone()
        }
    }
}

impl<T: Lerp + Clone> Lerp for Option<T> {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        match (a.is_some(), b.is_some()) {
            (true, true) => Some(T::lerp(a.as_ref().unwrap(), b.as_ref().unwrap(), t)),
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
