// use bevy_asset::prelude::*;
use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;

pub trait LerpValue {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

impl LerpValue for f32 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl LerpValue for Vec2 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl LerpValue for Vec3 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl LerpValue for Vec4 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl LerpValue for Quat {
    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        let mut b = *b;
        if a.dot(b) < 0.0 {
            b = -b;
        }

        Vec4::lerp((*a).into(), b.into(), t).normalize().into()
    }
}

impl<T: Asset + 'static> LerpValue for Handle<T> {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        if t > 0.99 {
            b.clone()
        } else {
            a.clone()
        }
    }
}

impl LerpValue for HandleUntyped {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        if t > 0.99 {
            b.clone()
        } else {
            a.clone()
        }
    }
}
