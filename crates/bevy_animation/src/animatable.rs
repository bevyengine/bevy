//! Traits and type for interpolating between values.

use crate::util;
use bevy_color::{Laba, LinearRgba, Oklaba, Srgba, Xyza};
use bevy_math::*;
use bevy_reflect::Reflect;
use bevy_transform::prelude::Transform;
use bevy_utils::default;

/// An individual input for [`Animatable::blend`].
pub struct BlendInput<T> {
    /// The individual item's weight. This may not be bound to the range `[0.0, 1.0]`.
    pub weight: f32,
    /// The input value to be blended.
    pub value: T,
    /// Whether or not to additively blend this input into the final result.
    pub additive: bool,
}

/// An animatable value type.
pub trait Animatable: Reflect + Sized + Send + Sync + 'static {
    /// Interpolates between `a` and `b` with  a interpolation factor of `time`.
    ///
    /// The `time` parameter here may not be clamped to the range `[0.0, 1.0]`.
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self;

    /// Blends one or more values together.
    ///
    /// Implementors should return a default value when no inputs are provided here.
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self;
}

macro_rules! impl_float_animatable {
    ($ty: ty, $base: ty) => {
        impl Animatable for $ty {
            #[inline]
            fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
                let t = <$base>::from(t);
                (*a) * (1.0 - t) + (*b) * t
            }

            #[inline]
            fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
                let mut value = Default::default();
                for input in inputs {
                    if input.additive {
                        value += <$base>::from(input.weight) * input.value;
                    } else {
                        value = Self::interpolate(&value, &input.value, input.weight);
                    }
                }
                value
            }
        }
    };
}

macro_rules! impl_color_animatable {
    ($ty: ident) => {
        impl Animatable for $ty {
            #[inline]
            fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
                let value = *a * (1. - t) + *b * t;
                value
            }

            #[inline]
            fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
                let mut value = Default::default();
                for input in inputs {
                    if input.additive {
                        value += input.weight * input.value;
                    } else {
                        value = Self::interpolate(&value, &input.value, input.weight);
                    }
                }
                value
            }
        }
    };
}

impl_float_animatable!(f32, f32);
impl_float_animatable!(Vec2, f32);
impl_float_animatable!(Vec3A, f32);
impl_float_animatable!(Vec4, f32);

impl_float_animatable!(f64, f64);
impl_float_animatable!(DVec2, f64);
impl_float_animatable!(DVec3, f64);
impl_float_animatable!(DVec4, f64);

impl_color_animatable!(LinearRgba);
impl_color_animatable!(Laba);
impl_color_animatable!(Oklaba);
impl_color_animatable!(Srgba);
impl_color_animatable!(Xyza);

// Vec3 is special cased to use Vec3A internally for blending
impl Animatable for Vec3 {
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }

    #[inline]
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut value = Vec3A::ZERO;
        for input in inputs {
            if input.additive {
                value += input.weight * Vec3A::from(input.value);
            } else {
                value = Vec3A::interpolate(&value, &Vec3A::from(input.value), input.weight);
            }
        }
        Self::from(value)
    }
}

impl Animatable for bool {
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        util::step_unclamped(*a, *b, t)
    }

    #[inline]
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        inputs
            .max_by(|a, b| FloatOrd(a.weight).cmp(&FloatOrd(b.weight)))
            .map(|input| input.value)
            .unwrap_or(false)
    }
}

impl Animatable for Transform {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            translation: Vec3::interpolate(&a.translation, &b.translation, t),
            rotation: Quat::interpolate(&a.rotation, &b.rotation, t),
            scale: Vec3::interpolate(&a.scale, &b.scale, t),
        }
    }

    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut translation = Vec3A::ZERO;
        let mut scale = Vec3A::ZERO;
        let mut rotation = Quat::IDENTITY;

        for input in inputs {
            if input.additive {
                translation += input.weight * Vec3A::from(input.value.translation);
                scale += input.weight * Vec3A::from(input.value.scale);
                rotation = rotation.slerp(input.value.rotation, input.weight);
            } else {
                translation = Vec3A::interpolate(
                    &translation,
                    &Vec3A::from(input.value.translation),
                    input.weight,
                );
                scale = Vec3A::interpolate(&scale, &Vec3A::from(input.value.scale), input.weight);
                rotation = Quat::interpolate(&rotation, &input.value.rotation, input.weight);
            }
        }

        Self {
            translation: Vec3::from(translation),
            rotation,
            scale: Vec3::from(scale),
        }
    }
}

impl Animatable for Quat {
    /// Performs a slerp to smoothly interpolate between quaternions.
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        // We want to smoothly interpolate between the two quaternions by default,
        // rather than using a quicker but less correct linear interpolation.
        a.slerp(*b, t)
    }

    #[inline]
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut value = Self::IDENTITY;
        for input in inputs {
            value = Self::interpolate(&value, &input.value, input.weight);
        }
        value
    }
}

/// Basically `Transform`, but with every part optional. Note that this is the true
/// output of `Transform` animation, since individual parts may be unconstrained
/// if they lack an animation curve to control them.
#[derive(Default, Debug, Clone, Reflect)]
pub(crate) struct TransformParts {
    translation: Option<Vec3>,
    rotation: Option<Quat>,
    scale: Option<Vec3>,
}

impl TransformParts {
    pub(crate) fn apply_to_transform(self, transform: &mut Transform) {
        if let Some(translation) = self.translation {
            transform.translation = translation;
        }
        if let Some(rotation) = self.rotation {
            transform.rotation = rotation;
        }
        if let Some(scale) = self.scale {
            transform.scale = scale;
        }
    }

    #[inline]
    pub(crate) fn from_translation(translation: Vec3) -> Self {
        Self {
            translation: Some(translation),
            ..default()
        }
    }

    #[inline]
    pub(crate) fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation: Some(rotation),
            ..default()
        }
    }

    #[inline]
    pub(crate) fn from_scale(scale: Vec3) -> Self {
        Self {
            scale: Some(scale),
            ..default()
        }
    }

    #[inline]
    pub(crate) fn from_transform(transform: Transform) -> Self {
        Self {
            translation: Some(transform.translation),
            rotation: Some(transform.rotation),
            scale: Some(transform.scale),
        }
    }
}

fn interpolate_option<A: Animatable + Copy>(a: Option<&A>, b: Option<&A>, time: f32) -> Option<A> {
    match (a, b) {
        (None, None) => None,
        (Some(a), None) => Some(*a),
        (None, Some(b)) => Some(*b),
        (Some(a), Some(b)) => Some(A::interpolate(a, b, time)),
    }
}

impl Animatable for TransformParts {
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self {
        TransformParts {
            translation: interpolate_option(a.translation.as_ref(), b.translation.as_ref(), time),
            rotation: interpolate_option(a.rotation.as_ref(), b.rotation.as_ref(), time),
            scale: interpolate_option(a.scale.as_ref(), b.scale.as_ref(), time),
        }
    }

    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut accum = TransformParts::default();
        for BlendInput {
            weight,
            value,
            additive,
        } in inputs
        {
            if additive {
                let Self {
                    translation,
                    rotation,
                    scale,
                } = value;
                if let Some(translation) = translation {
                    let weighted_translation = translation * weight;
                    match accum.translation {
                        Some(ref mut v) => *v += weighted_translation,
                        None => accum.translation = Some(weighted_translation),
                    }
                }
                if let Some(rotation) = rotation {
                    let weighted_rotation = Quat::slerp(Quat::IDENTITY, rotation, weight);
                    match accum.rotation {
                        Some(ref mut r) => *r = weighted_rotation * *r,
                        None => accum.rotation = Some(weighted_rotation),
                    }
                }
                if let Some(scale) = scale {
                    let weighted_scale = scale * weight;
                    match accum.scale {
                        Some(ref mut s) => *s += weighted_scale,
                        None => accum.scale = Some(weighted_scale),
                    }
                }
            } else {
                accum = Self::interpolate(&accum, &value, weight);
            }
        }
        accum
    }
}

/// Evaluates a cubic Bézier curve at a value `t`, given two endpoints and the
/// derivatives at those endpoints.
///
/// The derivatives are linearly scaled by `duration`.
pub fn interpolate_with_cubic_bezier<T>(p0: &T, d0: &T, d3: &T, p3: &T, t: f32, duration: f32) -> T
where
    T: Animatable + Clone,
{
    // We're given two endpoints, along with the derivatives at those endpoints,
    // and have to evaluate the cubic Bézier curve at time t using only
    // (additive) blending and linear interpolation.
    //
    // Evaluating a Bézier curve via repeated linear interpolation when the
    // control points are known is straightforward via [de Casteljau
    // subdivision]. So the only remaining problem is to get the two off-curve
    // control points. The [derivative of the cubic Bézier curve] is:
    //
    //      B′(t) = 3(1 - t)²(P₁ - P₀) + 6(1 - t)t(P₂ - P₁) + 3t²(P₃ - P₂)
    //
    // Setting t = 0 and t = 1 and solving gives us:
    //
    //      P₁ = P₀ + B′(0) / 3
    //      P₂ = P₃ - B′(1) / 3
    //
    // These P₁ and P₂ formulas can be expressed as additive blends.
    //
    // So, to sum up, first we calculate the off-curve control points via
    // additive blending, and then we use repeated linear interpolation to
    // evaluate the curve.
    //
    // [de Casteljau subdivision]: https://en.wikipedia.org/wiki/De_Casteljau%27s_algorithm
    // [derivative of the cubic Bézier curve]: https://en.wikipedia.org/wiki/B%C3%A9zier_curve#Cubic_B%C3%A9zier_curves

    // Compute control points from derivatives.
    let p1 = T::blend(
        [
            BlendInput {
                weight: duration / 3.0,
                value: (*d0).clone(),
                additive: true,
            },
            BlendInput {
                weight: 1.0,
                value: (*p0).clone(),
                additive: true,
            },
        ]
        .into_iter(),
    );
    let p2 = T::blend(
        [
            BlendInput {
                weight: duration / -3.0,
                value: (*d3).clone(),
                additive: true,
            },
            BlendInput {
                weight: 1.0,
                value: (*p3).clone(),
                additive: true,
            },
        ]
        .into_iter(),
    );

    // Use de Casteljau subdivision to evaluate.
    let p0p1 = T::interpolate(p0, &p1, t);
    let p1p2 = T::interpolate(&p1, &p2, t);
    let p2p3 = T::interpolate(&p2, p3, t);
    let p0p1p2 = T::interpolate(&p0p1, &p1p2, t);
    let p1p2p3 = T::interpolate(&p1p2, &p2p3, t);
    T::interpolate(&p0p1p2, &p1p2p3, t)
}
