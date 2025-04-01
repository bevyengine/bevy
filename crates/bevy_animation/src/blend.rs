//! This module contains traits for additive blending, for use in animation.

use bevy_color::{
    ColorToComponents, Gray, Hsla, Hwba, Laba, Lcha, LinearRgba, Oklaba, Oklcha, Srgba, Xyza,
};
use bevy_math::*;
use bevy_transform::components::Transform;

/// A type with a natural method of addative blending.
///
/// Any type with a well-behaved method of composition which also implements the [`Interpolate`] trait
/// can support addative blending. Specifically, the type must:
///
/// 1. Have a natural method of composition. For vectors this is addition, for quaternions or rotations
///    composition is usually expressed as multiplication. We'll use `comp(a, b)` to denote this operation.
///
/// 2. The operation must be associative. That is, `comp(comp(a, b), c)` must be equivilent to
///    `comp(a, comp(b, c))`. The result does not need to be data-identical, but it should be equivilent
///    under some reasonable notion of equivilence.
///
/// 2. Have an `IDENTITY` value such that composition with the identity is equivilent in some way to the
///    original value.
///
/// 3. The value of `T::blend(a, b, w)` should be equivilent to `comp(a, interp(IDENTITY, b, w))`. This implies
///    that `T::blend(a, b, 0)` is equivilent to `a` and `T::blend(a, b, 1)` is equivilent to `comp(a, b)`.
///
/// Some of you will have noticed that these rules encodes the axioms ofr a Monoid; In fact this trait
/// represents something similar to a Lie Group.
pub trait Blend: Interpolate {
    /// The default value of the blend, which has no effect when blended with other values.
    const IDENTITY: Self;

    /// Addatively blends another value on top of this one.
    fn blend(self, other: Self, blend_weight: f32) -> Self;
}

/// The state for an ongoing blend operation. On types implementing `Blend`, you can start a new
/// blend operation using [`Blendable::blend_additive`] or [`Blendable::blend_interp`].
pub struct Blender<T>
where
    T: Blend,
{
    /// The value being blended.
    value: T,
    /// The cumulative weight of the blend operation so far. Every new blend operation
    /// increases this weight.
    weight: f32,
}

impl<T> Blender<T>
where
    T: Blend,
{
    /// Addatively blends the value into the blender.
    pub fn blend_additive(self, value: T, weight: f32) -> Self {
        Blender {
            value: T::blend(self.value, value, weight),
            weight: self.weight + weight,
        }
    }

    /// Interpolatively blends the value into the blender.
    pub fn blend_interp(self, value: T, weight: f32) -> Self {
        let cumulative_weight = self.weight + weight;
        Blender {
            value: T::interp(&self.value, &value, weight / cumulative_weight),
            weight: cumulative_weight,
        }
    }

    /// Finishes the blend and returns the blended value.
    pub fn finish(self) -> T {
        self.value
    }
}

/// This is an extension trait to `Blend` with methods to easily blend and interpolate
/// between multiple values.
pub trait Blendable: Blend {
    /// Addatively blends the value into a new blender.
    fn blend_additive(self, weight: f32) -> Blender<Self> {
        Blender {
            value: Self::blend(Self::IDENTITY, self, weight),
            weight,
        }
    }

    /// Interpolateively blends the value into a new blender.
    fn blend_interp(self, weight: f32) -> Blender<Self> {
        Blender {
            value: self,
            weight,
        }
    }
}

impl<T> Blendable for T where T: Blend {}

macro_rules! impl_blendable_vectorspace {
    ($ty: ident) => {
        impl Blend for $ty {
            const IDENTITY: Self = <$ty as VectorSpace>::ZERO;

            #[inline]
            fn blend(self, other: Self, blend_weight: f32) -> Self {
                self + other * blend_weight
            }
        }
    };
}

impl_blendable_vectorspace!(f32);
impl_blendable_vectorspace!(Vec2);
impl_blendable_vectorspace!(Vec3);
impl_blendable_vectorspace!(Vec3A);
impl_blendable_vectorspace!(Vec4);

macro_rules! impl_blendable_color {
    ($ty: ident) => {
        impl Blend for $ty {
            const IDENTITY: Self = $ty::BLACK;

            #[inline]
            fn blend(self, other: Self, blend_weight: f32) -> Self {
                $ty::from_vec4(self.to_vec4() + other.to_vec4() * blend_weight)
            }
        }
    };
}

impl_blendable_color!(Srgba);
impl_blendable_color!(LinearRgba);
impl_blendable_color!(Hsla);
impl_blendable_color!(Hwba);
impl_blendable_color!(Laba);
impl_blendable_color!(Lcha);
impl_blendable_color!(Oklaba);
impl_blendable_color!(Oklcha);
impl_blendable_color!(Xyza);

macro_rules! impl_blendable_group_action {
    ($ty: ident) => {
        impl Blend for $ty {
            const IDENTITY: Self = $ty::IDENTITY;

            #[inline]
            fn blend(self, other: Self, blend_weight: f32) -> Self {
                $ty::interp(&Self::IDENTITY, &other, blend_weight) * self
            }
        }
    };
}

impl_blendable_group_action!(Rot2);
impl_blendable_group_action!(Quat);

impl Blend for Transform {
    const IDENTITY: Self = Transform::IDENTITY;

    fn blend(self, other: Self, blend_weight: f32) -> Self {
        Transform {
            translation: Vec3::blend(self.translation, other.translation, blend_weight),
            scale: Vec3::blend(self.scale, other.scale, blend_weight),
            rotation: Quat::blend(self.rotation, other.rotation, blend_weight),
        }
    }
}

impl Blend for Isometry2d {
    const IDENTITY: Self = Isometry2d::IDENTITY;

    fn blend(self, other: Self, blend_weight: f32) -> Self {
        Isometry2d {
            rotation: Rot2::blend(self.rotation, other.rotation, blend_weight),
            translation: Vec2::blend(self.translation, other.translation, blend_weight),
        }
    }
}

impl Blend for Isometry3d {
    const IDENTITY: Self = Isometry3d::IDENTITY;

    fn blend(self, other: Self, blend_weight: f32) -> Self {
        Isometry3d {
            rotation: Quat::blend(self.rotation, other.rotation, blend_weight),
            translation: Vec3A::blend(self.translation, other.translation, blend_weight),
        }
    }
}
