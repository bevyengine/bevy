//use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;
use bevy_render::color::Color;

use super::utils;

// http://archive.gamedev.net/archive/reference/articles/article1497.html (bit old)

pub struct TangentIgnore;

#[derive(Debug)]
pub enum Interpolation<T> {
    Step,
    Linear,
    /// Right tangent for the current keyframe and left tangent of the next keyframe
    Smooth {
        right: T,
        left: T,
    },
}

impl<T: Clone> Clone for Interpolation<T> {
    fn clone(&self) -> Self {
        match self {
            Interpolation::Step => Interpolation::Step,
            Interpolation::Linear => Interpolation::Linear,
            Interpolation::Smooth { right, left } => Interpolation::Smooth {
                right: right.clone(),
                left: left.clone(),
            },
        }
    }
}

pub trait Interpolate {
    type Tangent;

    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self;
}

impl Interpolate for bool {
    type Tangent = TangentIgnore;

    #[inline]
    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        let _ = interp;
        if t > 0.99 {
            *k0
        } else {
            *k1
        }
    }
}

impl Interpolate for f32 {
    type Tangent = Self;

    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        match interp {
            Interpolation::Step => utils::step(k0, k1, t),
            Interpolation::Linear => utils::lerp(t * k0, *k1, t),
            Interpolation::Smooth { right, left } => utils::catmull_rom(*k0, *right, *k1, *left, t),
        }
    }
}

impl Interpolate for Vec2 {
    type Tangent = Self;

    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        match interp {
            Interpolation::Step => utils::step(k0, k1, t),
            Interpolation::Linear => utils::lerp(*k0, *k1, t),
            Interpolation::Smooth { right, left } => utils::catmull_rom(*k0, *right, *k1, *left, t),
        }
    }
}

impl Interpolate for Vec3 {
    type Tangent = Self;

    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        match interp {
            Interpolation::Step => utils::step(k0, k1, t),
            Interpolation::Linear => utils::lerp(*k0, *k1, t),
            Interpolation::Smooth { right, left } => utils::catmull_rom(*k0, *right, *k1, *left, t),
        }
    }
}

impl Interpolate for Vec4 {
    type Tangent = Self;

    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        match interp {
            Interpolation::Step => utils::step(k0, k1, t),
            Interpolation::Linear => utils::lerp(*k0, *k1, t),
            Interpolation::Smooth { right, left } => utils::catmull_rom(*k0, *right, *k1, *left, t),
        }
    }
}

impl Interpolate for Color {
    type Tangent = Self;

    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        match interp {
            Interpolation::Step => utils::step(k0, k1, t),
            Interpolation::Linear => utils::lerp(*k0, *k1, t),
            Interpolation::Smooth { right, left } => utils::catmull_rom::<Vec4>(
                (*k0).into(),
                (*right).into(),
                (*k1).into(),
                (*left).into(),
                t,
            )
            .into(),
        }
    }
}

impl Interpolate for Quat {
    type Tangent = Self;

    /// Performs an nlerp, because it's much cheaper and easer to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    fn interpolate(k0: &Self, k1: &Self, interp: &Interpolation<Self::Tangent>, t: f32) -> Self {
        match interp {
            Interpolation::Step => utils::step(k0, k1, t),
            Interpolation::Linear => {
                // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
                let mut k1 = *k1;
                if k0.dot(k1) < 0.0 {
                    k1 = -k1;
                }

                let q = utils::lerp::<Vec4>((*k0).into(), k1.into(), t);
                let d = utils::inv_sqrt(q.dot(q));
                (q * d).into()
            }
            Interpolation::Smooth { right, left } => {
                // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
                let mut k1 = *k1;
                if k0.dot(k1) < 0.0 {
                    k1 = -k1;
                }

                let q = utils::catmull_rom::<Vec4>(
                    (*k0).into(),
                    (*right).into(),
                    k1.into(),
                    (*left).into(),
                    t,
                );
                let d = utils::inv_sqrt(q.dot(q));
                (q * d).into()
            }
        }
    }
}
