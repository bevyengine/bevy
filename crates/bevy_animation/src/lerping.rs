use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;
use bevy_render::color::Color;

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

impl Lerp for Vec4 {
    #[inline(always)]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
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
        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        let mut b = *b;
        if a.dot(b) < 0.0 {
            b = -b;
        }

        let q = Vec4::lerp((*a).into(), b.into(), t);
        let d = inv_sqrt(q.dot(q));
        (q * d).into()
    }
}

/// Quake 3 fast inverse sqrt
///
/// Took this one from piston: https://github.com/PistonDevelopers/skeletal_animation
fn inv_sqrt(x: f32) -> f32 {
    let x2: f32 = x * 0.5;
    let mut y: f32 = x;

    let mut i: i32 = unsafe { std::mem::transmute(y) };
    i = 0x5f3759df - (i >> 1);
    y = unsafe { std::mem::transmute(i) };

    y = y * (1.5 - (x2 * y * y));
    y
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
