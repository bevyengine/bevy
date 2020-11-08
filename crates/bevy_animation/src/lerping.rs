use bevy_math::prelude::*;

pub trait LerpValue: Copy {
    fn lerp(a: Self, b: Self, t: f32) -> Self;
}

impl LerpValue for Vec3 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a * (1.0 - t) + b * t
    }
}

impl LerpValue for Quat {
    /// Actually performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        // ! NOTE: slerp is nice if you want to debug the animation code
        // a.slerp(b, t)
        Vec4::lerp(a.into(), b.into(), t).normalize().into()
    }
}
