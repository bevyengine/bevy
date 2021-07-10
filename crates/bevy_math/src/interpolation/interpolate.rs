use super::{utils, Lerp};
use crate::{Quat, Vec2, Vec3, Vec3A, Vec4};

// http://archive.gamedev.net/archive/reference/articles/article1497.html (bit old)

#[derive(Debug, Copy, Clone)]
pub struct TangentIgnore;

/// Defines which function will be used to interpolate from the current keyframe to the next one
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Interpolation {
    Step,
    Linear,
    Hermite,
}

pub trait Interpolate: Lerp + Clone {
    /// Tangent used for the hermite interpolation
    type Tangent: Copy;

    const FLAT_TANGENT: Self::Tangent;

    #[inline]
    fn interpolate(
        k0: &Self,
        t0: &Self::Tangent,
        k1: &Self,
        t1: &Self::Tangent,
        interp: Interpolation,
        t: f32,
        dt: f32,
    ) -> Self {
        Self::interpolate_unclamped(k0, t0, k1, t1, interp, t.clamp(0.0, 1.0), dt)
    }

    fn interpolate_unclamped(
        k0: &Self,
        t0: &Self::Tangent,
        k1: &Self,
        t1: &Self::Tangent,
        interp: Interpolation,
        t: f32,
        dt: f32,
    ) -> Self;

    fn auto_tangent(t0: f32, t1: f32, t2: f32, k0: &Self, k1: &Self, k2: &Self) -> Self::Tangent;
}

impl Interpolate for bool {
    type Tangent = TangentIgnore;
    const FLAT_TANGENT: Self::Tangent = TangentIgnore;

    #[inline]
    fn interpolate_unclamped(
        k0: &Self,
        _: &Self::Tangent,
        k1: &Self,
        _: &Self::Tangent,
        _: Interpolation,
        t: f32,
        _: f32,
    ) -> Self {
        utils::step_unclamped(k0, k1, t)
    }

    fn auto_tangent(_: f32, _: f32, _: f32, _: &Self, _: &Self, _: &Self) -> Self::Tangent {
        TangentIgnore
    }
}

macro_rules! interpolate {
    ($ty:ty, $flat:expr) => {
        impl Interpolate for $ty {
            type Tangent = Self;
            const FLAT_TANGENT: Self::Tangent = $flat;

            fn interpolate_unclamped(
                k0: &Self,
                t0: &Self::Tangent,
                k1: &Self,
                t1: &Self::Tangent,
                interp: Interpolation,
                t: f32,
                dt: f32,
            ) -> Self {
                match interp {
                    Interpolation::Step => utils::step_unclamped(k0, k1, t),
                    Interpolation::Linear => utils::lerp_unclamped(*k0, *k1, t),
                    Interpolation::Hermite => utils::hermite_unclamped(*k0, *t0, *k1, *t1, t, dt),
                }
            }

            #[inline]
            fn auto_tangent(
                t0: f32,
                t1: f32,
                t2: f32,
                k0: &Self,
                k1: &Self,
                k2: &Self,
            ) -> Self::Tangent {
                utils::auto_tangent(t0, t1, t2, *k0, *k1, *k2)
            }
        }
    };
}

interpolate!(f32, 0.0);
interpolate!(Vec2, Vec2::ZERO);
interpolate!(Vec3, Vec3::ZERO);
interpolate!(Vec3A, Vec3A::ZERO);
interpolate!(Vec4, Vec4::ZERO);

// TODO: Color can't be interpolated because color operations are undefined, see pr #1870
// impl Interpolate for Color {
//     type Tangent = Self;

//     fn interpolate(k0: &Self, k1: &Self, interp: Interpolation, t: f32) -> Self {
//         match interp {
//             Interpolation::Step => utils::step(k0, k1, t),
//             Interpolation::Linear => utils::lerp(*k0, *k1, t),
//             Interpolation::Smooth { right, left } => utils::catmull_rom::<Vec4>(
//                 (*k0).into(),
//                 (*right).into(),
//                 (*k1).into(),
//                 (*left).into(),
//                 t,
//             )
//             .into(),
//         }
//     }

//     fn auto_tangent<T>(t0: f32, t1: f32, t2: f32, k0: Self, k1: Self, k2: Self) -> Self::Tangent {
//         utils::auto_tangent(t0, t1, t2, k0, k1, k2)
//     }
// }

impl Interpolate for Quat {
    type Tangent = Self;
    const FLAT_TANGENT: Self::Tangent = unsafe { std::mem::transmute([0.0f32; 4]) };

    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    fn interpolate_unclamped(
        k0: &Self,
        t0: &Self::Tangent,
        k1: &Self,
        t1: &Self::Tangent,
        interp: Interpolation,
        t: f32,
        dt: f32,
    ) -> Self {
        match interp {
            Interpolation::Step => utils::step_unclamped(k0, k1, t),
            Interpolation::Linear => {
                // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
                let mut k1 = *k1;
                if k0.dot(k1) < 0.0 {
                    k1 = -k1;
                }

                let q = utils::lerp_unclamped::<Vec4>((*k0).into(), k1.into(), t);
                let d = utils::fast_inv_sqrt(q.dot(q));
                Quat::from_vec4(q * d)
            }
            Interpolation::Hermite => {
                // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
                let mut k1 = *k1;
                if k0.dot(k1) < 0.0 {
                    k1 = -k1;
                }

                let q = utils::hermite_unclamped::<Vec4>(
                    (*k0).into(),
                    (*t0).into(),
                    k1.into(),
                    (*t1).into(),
                    t,
                    dt,
                );
                let d = utils::fast_inv_sqrt(q.dot(q));
                Quat::from_vec4(q * d)
            }
        }
    }

    #[inline]
    fn auto_tangent(t0: f32, t1: f32, t2: f32, k0: &Self, k1: &Self, k2: &Self) -> Self::Tangent {
        utils::auto_tangent(t0, t1, t2, *k0, *k1, *k2)
    }
}
