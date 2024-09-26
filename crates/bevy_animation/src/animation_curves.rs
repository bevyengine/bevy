//! The [`AnimationCurve`] trait and adaptors that allow curves to implement it.
//!
//! # Overview
//!
//! The flow of curves into the animation system generally begins with something that
//! implements the [`Curve`] trait. Let's imagine, for example, that we have some
//! `Curve<Vec3>` that we want to use to animate something. That could be defined in
//! a number of different ways, but let's imagine that we've defined it [using a function]:
//!
//!     # use bevy_math::curve::{Curve, Interval, function_curve};
//!     # use bevy_math::vec3;
//!     let wobble_curve = function_curve(
//!         Interval::UNIT,
//!         |t| { vec3(t.cos(), 0.0, 0.0) },
//!     );
//!
//! Okay, so we have a curve, but the animation system also needs to know, in some way,
//! how the values from this curve should actually be used. That is, it needs to know what
//! to animate! That's what [`AnimationCurve`] is for. In particular, what we need to do
//! is take our curve and turn it into an `AnimationCurve` which will be usable by the
//! animation system.
//!
//! For instance, let's imagine that we want to imagine that we want to use the `Vec3` output
//! from our curve to animate the [translation component of a `Transform`]. For this, there is
//! the adaptor [`TranslationCurve`], which wraps any `Curve<Vec3>` and turns it into an
//! [`AnimationCurve`] that will use the given curve to animate the entity's translation:
//!
//!     # use bevy_math::curve::{Curve, Interval, function_curve};
//!     # use bevy_math::vec3;
//!     # use bevy_animation::animation_curves::*;
//!     # let wobble_curve = function_curve(
//!     #     Interval::UNIT,
//!     #     |t| vec3(t.cos(), 0.0, 0.0)
//!     # );
//!     let wobble_animation = TranslationCurve(wobble_curve);
//!
//! And finally, this `AnimationCurve` needs to be added to an [`AnimationClip`] in order to
//! actually animate something. This is what that looks like:
//!
//!     # use bevy_math::curve::{Curve, Interval, function_curve};
//!     # use bevy_animation::{AnimationClip, AnimationTargetId, animation_curves::*};
//!     # use bevy_core::Name;
//!     # use bevy_math::vec3;
//!     # let wobble_curve = function_curve(
//!     #     Interval::UNIT,
//!     #     |t| { vec3(t.cos(), 0.0, 0.0) },
//!     # );
//!     # let wobble_animation = TranslationCurve(wobble_curve);
//!     # let animation_target_id = AnimationTargetId::from(&Name::new("Test"));
//!     let mut animation_clip = AnimationClip::default();
//!     animation_clip.add_curve_to_target(
//!         animation_target_id,
//!         wobble_animation,
//!     );
//!
//! # Making animation curves
//!
//! The overview showed one example, but in general there are a few different ways of going from
//! a [`Curve`], which produces time-related data of some kind, to an [`AnimationCurve`], which
//! knows how to apply that data to an entity.
//!
//! ## `Transform`
//!
//! [`Transform`] is special and has its own adaptors:
//!     - [`TranslationCurve`], which uses `Vec3` output to animate [`Transform::translation`]
//!     - [`RotationCurve`], which uses `Quat` output to animate [`Transform::rotation`]
//!     - [`ScaleCurve`], which uses `Vec3` output to animate [`Transform::scale`]
//!     - [`TransformCurve`], which uses `Transform` output to animate the entire `Transform`
//!
//! ## Animatable properties
//!
//! Animation of arbitrary components can be accomplished using [`AnimatableProperty`] in
//! conjunction with [`AnimatableCurve`]. See the documentation [there] for details.
//!
//! [using a function]: bevy_math::curve::function_curve
//! [translation component of a `Transform`]: bevy_transform::prelude::Transform::translation
//! [`AnimationClip`]: crate::AnimationClip
//! [there]: AnimatableProperty

use std::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    world::{EntityMutExcept, Mut},
};
use bevy_math::{
    curve::{
        cores::{UnevenCore, UnevenCoreError},
        iterable::IterableCurve,
        Curve, Interval,
    },
    FloatExt, Quat, Vec3,
};
use bevy_reflect::{FromReflect, Reflect, Reflectable, TypePath};
use bevy_render::mesh::morph::MorphWeights;
use bevy_transform::prelude::Transform;

use crate::{
    graph::AnimationGraph, prelude::Animatable, AnimationEvaluationError, AnimationPlayer,
};

/// A value on a component that Bevy can animate.
///
/// You can implement this trait on a unit struct in order to support animating
/// custom components other than transforms and morph weights. Use that type in
/// conjunction with [`AnimatableCurve`] (and perhaps [`AnimatableKeyframeCurve`]
/// to define the animation itself). For example, in order to animate font size of a
/// text section from 24 pt. to 80 pt., you might use:
///
///     # use bevy_animation::prelude::AnimatableProperty;
///     # use bevy_reflect::Reflect;
///     # use bevy_text::Text;
///     #[derive(Reflect)]
///     struct FontSizeProperty;
///
///     impl AnimatableProperty for FontSizeProperty {
///         type Component = Text;
///         type Property = f32;
///         fn get_mut(component: &mut Self::Component) -> Option<&mut Self::Property> {
///             Some(&mut component.sections.get_mut(0)?.style.font_size)
///         }
///     }
///
/// You can then create an [`AnimationClip`] to animate this property like so:
///
///     # use bevy_animation::{AnimationClip, AnimationTargetId, VariableCurve};
///     # use bevy_animation::prelude::{AnimatableProperty, AnimatableKeyframeCurve, AnimatableCurve};
///     # use bevy_core::Name;
///     # use bevy_reflect::Reflect;
///     # use bevy_text::Text;
///     # let animation_target_id = AnimationTargetId::from(&Name::new("Test"));
///     # #[derive(Reflect)]
///     # struct FontSizeProperty;
///     # impl AnimatableProperty for FontSizeProperty {
///     #     type Component = Text;
///     #     type Property = f32;
///     #     fn get_mut(component: &mut Self::Component) -> Option<&mut Self::Property> {
///     #         Some(&mut component.sections.get_mut(0)?.style.font_size)
///     #     }
///     # }
///     let mut animation_clip = AnimationClip::default();
///     animation_clip.add_curve_to_target(
///         animation_target_id,
///         AnimatableKeyframeCurve::new(
///             [
///                 (0.0, 24.0),
///                 (1.0, 80.0),
///             ]
///         )
///         .map(AnimatableCurve::<FontSizeProperty, _>::from_curve)
///         .expect("Failed to create font size curve")
///     );
///
/// Here, the use of `AnimatableKeyframeCurve` creates a curve out of the given keyframe time-value
/// pairs, using the `Animatable` implementation of `f32` to interpolate between then. The
/// invocation of [`AnimatableCurve::from_curve`] with `FontSizeProperty` indicates that the `f32`
/// output from that curve is to be used to animate the font size of a `Text` component (as
/// configured above).
///
/// [`AnimationClip`]: crate::AnimationClip
pub trait AnimatableProperty: Reflect + TypePath {
    /// The type of the component that the property lives on.
    type Component: Component;

    /// The type of the property to be animated.
    type Property: Animatable + FromReflect + Reflectable + Clone + Sync + Debug;

    /// Given a reference to the component, returns a reference to the property.
    ///
    /// If the property couldn't be found, returns `None`.
    fn get_mut(component: &mut Self::Component) -> Option<&mut Self::Property>;
}

/// This trait collects the additional requirements on top of [`Curve<T>`] needed for a
/// curve to be used as an [`AnimationCurve`].
pub trait InnerAnimationCurve<T>: Curve<T> + Debug + Clone + Reflectable {}

impl<T, C> InnerAnimationCurve<T> for C where C: Curve<T> + Debug + Clone + Reflectable {}

/// This type allows the conversion of a [curve] valued in the [property type] of an
/// [`AnimatableProperty`] into an [`AnimationCurve`] which animates that property.
///
/// [curve]: Curve
/// [property type]: AnimatableProperty::Property
#[derive(Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct AnimatableCurve<P, C> {
    curve: C,
    #[reflect(ignore)]
    _phantom: PhantomData<P>,
}

impl<P, C> AnimatableCurve<P, C>
where
    P: AnimatableProperty,
    C: InnerAnimationCurve<P::Property>,
{
    /// Create an [`AnimatableCurve`] (and thus an [`AnimationCurve`]) from a curve
    /// valued in an [animatable property].
    ///
    /// [animatable property]: AnimatableProperty::Property
    pub fn from_curve(curve: C) -> Self {
        Self {
            curve,
            _phantom: PhantomData,
        }
    }
}

impl<P, C> Clone for AnimatableCurve<P, C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            curve: self.curve.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<P, C> Debug for AnimatableCurve<P, C>
where
    C: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatableCurve")
            .field("curve", &self.curve)
            .finish()
    }
}

impl<P, C> AnimationCurve for AnimatableCurve<P, C>
where
    P: AnimatableProperty,
    C: InnerAnimationCurve<P::Property>,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    fn apply<'a>(
        &self,
        t: f32,
        _transform: Option<Mut<'a, Transform>>,
        mut entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = entity.get_mut::<P::Component>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<P::Component>())
        })?;
        let property = P::get_mut(&mut component)
            .ok_or_else(|| AnimationEvaluationError::PropertyNotPresent(TypeId::of::<P>()))?;
        let value = self.curve.sample_clamped(t);
        *property = <P::Property>::interpolate(property, &value, weight);
        Ok(())
    }
}

/// This type allows a [curve] valued in `Vec3` to become an [`AnimationCurve`] that animates
/// the translation component of a transform.
///
/// [curve]: Curve
#[derive(Debug, Clone, Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct TranslationCurve<C>(pub C);

impl<C> AnimationCurve for TranslationCurve<C>
where
    C: InnerAnimationCurve<Vec3>,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn apply<'a>(
        &self,
        t: f32,
        transform: Option<Mut<'a, Transform>>,
        _entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let new_value = self.0.sample_clamped(t);
        component.translation =
            <Vec3 as Animatable>::interpolate(&component.translation, &new_value, weight);
        Ok(())
    }
}

/// This type allows a [curve] valued in `Quat` to become an [`AnimationCurve`] that animates
/// the rotation component of a transform.
///
/// [curve]: Curve
#[derive(Debug, Clone, Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct RotationCurve<C>(pub C);

impl<C> AnimationCurve for RotationCurve<C>
where
    C: InnerAnimationCurve<Quat>,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn apply<'a>(
        &self,
        t: f32,
        transform: Option<Mut<'a, Transform>>,
        _entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let new_value = self.0.sample_clamped(t);
        component.rotation =
            <Quat as Animatable>::interpolate(&component.rotation, &new_value, weight);
        Ok(())
    }
}

/// This type allows a [curve] valued in `Vec3` to become an [`AnimationCurve`] that animates
/// the scale component of a transform.
///
/// [curve]: Curve
#[derive(Debug, Clone, Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct ScaleCurve<C>(pub C);

impl<C> AnimationCurve for ScaleCurve<C>
where
    C: InnerAnimationCurve<Vec3>,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn apply<'a>(
        &self,
        t: f32,
        transform: Option<Mut<'a, Transform>>,
        _entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let new_value = self.0.sample_clamped(t);
        component.scale = <Vec3 as Animatable>::interpolate(&component.scale, &new_value, weight);
        Ok(())
    }
}

/// This type allows a [curve] valued in `Transform` to become an [`AnimationCurve`] that animates
/// a transform.
///
/// This exists primarily as a convenience to animate entities using the entire transform at once
/// instead of splitting it into pieces and animating each part (translation, rotation, scale).
///
/// [curve]: Curve
#[derive(Debug, Clone, Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct TransformCurve<C>(pub C);

impl<C> AnimationCurve for TransformCurve<C>
where
    C: InnerAnimationCurve<Transform>,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn apply<'a>(
        &self,
        t: f32,
        transform: Option<Mut<'a, Transform>>,
        _entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let new_value = self.0.sample_clamped(t);
        *component = <Transform as Animatable>::interpolate(&component, &new_value, weight);
        Ok(())
    }
}

/// This type allows an [`IterableCurve`] valued in `f32` to be used as an [`AnimationCurve`]
/// that animates [morph weights].
///
/// [morph weights]: MorphWeights
#[derive(Debug, Clone, Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct WeightsCurve<C>(pub C);

impl<C> AnimationCurve for WeightsCurve<C>
where
    C: IterableCurve<f32> + Debug + Clone + Reflectable,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn apply<'a>(
        &self,
        t: f32,
        _transform: Option<Mut<'a, Transform>>,
        mut entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut dest = entity.get_mut::<MorphWeights>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<MorphWeights>())
        })?;
        lerp_morph_weights(dest.weights_mut(), self.0.sample_iter_clamped(t), weight);
        Ok(())
    }
}

/// Update `morph_weights` based on weights in `incoming_weights` with a linear interpolation
/// on `lerp_weight`.
fn lerp_morph_weights(
    morph_weights: &mut [f32],
    incoming_weights: impl Iterator<Item = f32>,
    lerp_weight: f32,
) {
    let zipped = morph_weights.iter_mut().zip(incoming_weights);
    for (morph_weight, incoming_weights) in zipped {
        *morph_weight = morph_weight.lerp(incoming_weights, lerp_weight);
    }
}

/// A low-level trait that provides control over how curves are actually applied to entities
/// by the animation system.
///
/// Typically, this will not need to be implemented manually, since it is automatically
/// implemented by [`AnimatableCurve`] and other curves used by the animation system
/// (e.g. those that animate parts of transforms or morph weights). However, this can be
/// implemented manually when `AnimatableCurve` is not sufficiently expressive.
///
/// In many respects, this behaves like a type-erased form of [`Curve`], where the output
/// type of the curve is remembered only in the components that are mutated in the
/// implementation of [`apply`].
///
/// [`apply`]: AnimationCurve::apply
pub trait AnimationCurve: Reflect + Debug + Send + Sync {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn AnimationCurve>;

    /// The range of times for which this animation is defined.
    fn domain(&self) -> Interval;

    /// Write the value of sampling this curve at time `t` into `transform` or `entity`,
    /// as appropriate, interpolating between the existing value and the sampled value
    /// using the given `weight`.
    fn apply<'a>(
        &self,
        t: f32,
        transform: Option<Mut<'a, Transform>>,
        entity: EntityMutExcept<'a, (Transform, AnimationPlayer, Handle<AnimationGraph>)>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError>;
}

/// A [curve] defined by keyframes with values in an [animatable] type.
///
/// The keyframes are interpolated using the type's [`Animatable::interpolate`] implementation.
///
/// [curve]: Curve
/// [animatable]: Animatable
#[derive(Debug, Clone, Reflect)]
pub struct AnimatableKeyframeCurve<T> {
    core: UnevenCore<T>,
}

impl<T> Curve<T> for AnimatableKeyframeCurve<T>
where
    T: Animatable + Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.core.sample_with(t, <T as Animatable>::interpolate)
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        // Sampling by keyframes is automatically clamped to the keyframe bounds.
        self.sample_unchecked(t)
    }
}

impl<T> AnimatableKeyframeCurve<T>
where
    T: Animatable,
{
    /// Create a new [`AnimatableKeyframeCurve`] from the given `keyframes`. The values of this
    /// curve are interpolated from the keyframes using the output type's implementation of
    /// [`Animatable::interpolate`].
    ///
    /// There must be at least two samples in order for this method to succeed.
    pub fn new(keyframes: impl IntoIterator<Item = (f32, T)>) -> Result<Self, UnevenCoreError> {
        Ok(Self {
            core: UnevenCore::new(keyframes)?,
        })
    }
}
