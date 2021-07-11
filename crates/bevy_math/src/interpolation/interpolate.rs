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

    /// Interpolates between two keyframes using a predefined function
    /// controlled by the factor `u` clamped in the range from 0 to 1.
    ///
    /// **NOTE** `delta_time` refers to the time difference between the keyframes.
    #[inline]
    fn interpolate(
        value0: &Self,
        tangent0: &Self::Tangent,
        value1: &Self,
        tangent1: &Self::Tangent,
        interp: Interpolation,
        u: f32,
        delta_time: f32,
    ) -> Self {
        Self::interpolate_unclamped(
            value0,
            tangent0,
            value1,
            tangent1,
            interp,
            u.clamp(0.0, 1.0),
            delta_time,
        )
    }

    /// Interpolates between two keyframes using a predefined function
    /// controlled by the factor `u` whiting the 0 to 1 range.
    ///
    /// **NOTE** `delta_time` refers to the time difference between the keyframes.
    fn interpolate_unclamped(
        value0: &Self,
        tangent0: &Self::Tangent,
        value1: &Self,
        tangent1: &Self::Tangent,
        interp: Interpolation,
        u: f32,
        delta_time: f32,
    ) -> Self;

    fn auto_tangent(
        time0: f32,
        time1: f32,
        time2: f32,
        value0: &Self,
        value1: &Self,
        value2: &Self,
    ) -> Self::Tangent;
}

impl Interpolate for bool {
    type Tangent = TangentIgnore;
    const FLAT_TANGENT: Self::Tangent = TangentIgnore;

    #[inline]
    fn interpolate_unclamped(
        value0: &Self,
        _: &Self::Tangent,
        value1: &Self,
        _: &Self::Tangent,
        _: Interpolation,
        u: f32,
        _: f32,
    ) -> Self {
        utils::step_unclamped(value0, value1, u)
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
                value0: &Self,
                tangent0: &Self::Tangent,
                value1: &Self,
                tangent1: &Self::Tangent,
                interp: Interpolation,
                u: f32,
                delta_time: f32,
            ) -> Self {
                match interp {
                    Interpolation::Step => utils::step_unclamped(value0, value1, u),
                    Interpolation::Linear => utils::lerp_unclamped(*value0, *value1, u),
                    Interpolation::Hermite => utils::hermite_unclamped(
                        *value0, *tangent0, *value1, *tangent1, u, delta_time,
                    ),
                }
            }

            #[inline]
            fn auto_tangent(
                time0: f32,
                time1: f32,
                time2: f32,
                value0: &Self,
                value1: &Self,
                value2: &Self,
            ) -> Self::Tangent {
                utils::auto_tangent(time0, time1, time2, *value0, *value1, *value2)
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

//     fn interpolate(value0: &Self, value1: &Self, interp: Interpolation, u: f32, delta_time: f32) -> Self {
//         match interp {
//             Interpolation::Step => utils::step(value0, value1, u),
//             Interpolation::Linear => utils::lerp(*value0, *value1, u),
//             Interpolation::Smooth { right, left } => utils::hermite_unclamped::<Vec4>(
//                 (*value0).into(),
//                 (*right).into(),
//                 (*value1).into(),
//                 (*left).into(),
//                 u,
//                 delta_time,
//             )
//             .into(),
//         }
//     }

//     fn auto_tangent<T>(time0: f32, time1: f32, time2: f32, value0: Self, value1: Self, value2: Self) -> Self::Tangent {
//         utils::auto_tangent(time0, time1, time2, value0, value1, value2)
//     }
// }

impl Interpolate for Quat {
    type Tangent = Self;
    const FLAT_TANGENT: Self::Tangent = unsafe { std::mem::transmute([0.0f32; 4]) };

    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    fn interpolate_unclamped(
        value0: &Self,
        tangent0: &Self::Tangent,
        value1: &Self,
        tangent1: &Self::Tangent,
        interp: Interpolation,
        u: f32,
        delta_time: f32,
    ) -> Self {
        match interp {
            Interpolation::Step => utils::step_unclamped(value0, value1, u),
            Interpolation::Linear => {
                // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
                let mut value1 = *value1;
                if value0.dot(value1) < 0.0 {
                    value1 = -value1;
                }

                let q = utils::lerp_unclamped::<Vec4>((*value0).into(), value1.into(), u);
                let d = utils::fast_inv_sqrt(q.dot(q));
                Quat::from_vec4(q * d)
            }
            Interpolation::Hermite => {
                // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
                let mut value1 = *value1;
                if value0.dot(value1) < 0.0 {
                    value1 = -value1;
                }

                let q = utils::hermite_unclamped::<Vec4>(
                    (*value0).into(),
                    (*tangent0).into(),
                    value1.into(),
                    (*tangent1).into(),
                    u,
                    delta_time,
                );
                let d = utils::fast_inv_sqrt(q.dot(q));
                Quat::from_vec4(q * d)
            }
        }
    }

    #[inline]
    fn auto_tangent(
        time0: f32,
        time1: f32,
        time2: f32,
        value0: &Self,
        value1: &Self,
        value2: &Self,
    ) -> Self::Tangent {
        utils::auto_tangent(time0, time1, time2, *value0, *value1, *value2)
    }
}
