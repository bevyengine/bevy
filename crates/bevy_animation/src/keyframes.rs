//! Keyframes of animation clips.

use core::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, world::Mut};
use bevy_math::{Quat, Vec3};
use bevy_reflect::{FromReflect, Reflect, Reflectable, TypePath};
use bevy_render::mesh::morph::MorphWeights;
use bevy_transform::prelude::Transform;

use crate::{
    animatable,
    prelude::{Animatable, GetKeyframe},
    AnimationEntityMut, AnimationEvaluationError, Interpolation,
};

/// A value on a component that Bevy can animate.
///
/// You can implement this trait on a unit struct in order to support animating
/// custom components other than transforms and morph weights. Use that type in
/// conjunction with [`AnimatablePropertyKeyframes`]. For example, in order to
/// animate font size of a text section from 24 pt. to 80 pt., you might use:
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
/// You can then create a [`crate::AnimationClip`] to animate this property like so:
///
///     # use bevy_animation::{AnimationClip, AnimationTargetId, Interpolation, VariableCurve};
///     # use bevy_animation::prelude::{AnimatableProperty, AnimatablePropertyKeyframes};
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
///         VariableCurve::linear::<AnimatablePropertyKeyframes<FontSizeProperty>>(
///             [0.0, 1.0],
///             [24.0, 80.0],
///         ),
///     );
pub trait AnimatableProperty: Reflect + TypePath + 'static {
    /// The type of the component that the property lives on.
    type Component: Component;

    /// The type of the property to be animated.
    type Property: Animatable + FromReflect + Reflectable + Clone + Sync + Debug + 'static;

    /// Given a reference to the component, returns a reference to the property.
    ///
    /// If the property couldn't be found, returns `None`.
    fn get_mut(component: &mut Self::Component) -> Option<&mut Self::Property>;
}

/// Keyframes in a [`crate::VariableCurve`] that animate an
/// [`AnimatableProperty`].
///
/// This is the a generic type of [`Keyframes`] that can animate any
/// [`AnimatableProperty`]. See the documentation for [`AnimatableProperty`] for
/// more information as to how to use this type.
///
/// If you're animating scale, rotation, or translation of a [`Transform`],
/// [`ScaleKeyframes`], [`RotationKeyframes`], and [`TranslationKeyframes`] are
/// faster, and simpler, alternatives to this type.
#[derive(Reflect, Deref, DerefMut)]
pub struct AnimatablePropertyKeyframes<P>(pub Vec<P::Property>)
where
    P: AnimatableProperty;

impl<P> Clone for AnimatablePropertyKeyframes<P>
where
    P: AnimatableProperty,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<P> Debug for AnimatablePropertyKeyframes<P>
where
    P: AnimatableProperty,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AnimatablePropertyKeyframes")
            .field(&self.0)
            .finish()
    }
}

/// A low-level trait for use in [`crate::VariableCurve`] that provides fine
/// control over how animations are evaluated.
///
/// You can implement this trait when the generic
/// [`AnimatablePropertyKeyframes`] isn't sufficiently-expressive for your
/// needs. For example, [`MorphWeights`] implements this trait instead of using
/// [`AnimatablePropertyKeyframes`] because it needs to animate arbitrarily many
/// weights at once, which can't be done with [`Animatable`] as that works on
/// fixed-size values only.
pub trait Keyframes: Reflect + Debug + Send + Sync {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn Keyframes>;

    /// Interpolates between the existing value and the value of the first
    /// keyframe, and writes the value into `transform` and/or `entity` as
    /// appropriate.
    ///
    /// Arguments:
    ///
    /// * `transform`: The transform of the entity, if present.
    ///
    /// * `entity`: Allows access to the rest of the components of the entity.
    ///
    /// * `weight`: The blend weight between the existing component value (0.0)
    ///   and the one computed from the keyframes (1.0).
    fn apply_single_keyframe<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        entity: AnimationEntityMut<'a>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError>;

    /// Interpolates between the existing value and the value of the two nearest
    /// keyframes, and writes the value into `transform` and/or `entity` as
    /// appropriate.
    ///
    /// Arguments:
    ///
    /// * `transform`: The transform of the entity, if present.
    ///
    /// * `entity`: Allows access to the rest of the components of the entity.
    ///
    /// * `interpolation`: The type of interpolation to use.
    ///
    /// * `step_start`: The index of the first keyframe.
    ///
    /// * `time`: The blend weight between the first keyframe (0.0) and the next
    ///   keyframe (1.0).
    ///
    /// * `weight`: The blend weight between the existing component value (0.0)
    ///   and the one computed from the keyframes (1.0).
    ///
    /// If `interpolation` is `Interpolation::Linear`, then pseudocode for this
    /// function could be `property = lerp(property, lerp(keyframes[step_start],
    /// keyframes[step_start + 1], time), weight)`.
    #[allow(clippy::too_many_arguments)]
    fn apply_tweened_keyframes<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        entity: AnimationEntityMut<'a>,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError>;
}

/// Keyframes for animating [`Transform::translation`].
///
/// An example of a [`crate::AnimationClip`] that animates translation:
///
///     # use bevy_animation::{AnimationClip, AnimationTargetId, Interpolation};
///     # use bevy_animation::{VariableCurve, prelude::TranslationKeyframes};
///     # use bevy_core::Name;
///     # use bevy_math::Vec3;
///     # let animation_target_id = AnimationTargetId::from(&Name::new("Test"));
///     let mut animation_clip = AnimationClip::default();
///     animation_clip.add_curve_to_target(
///         animation_target_id,
///         VariableCurve::linear::<TranslationKeyframes>(
///             [0.0, 1.0],
///             [Vec3::ZERO, Vec3::ONE],
///         ),
///     );
#[derive(Clone, Reflect, Debug, Deref, DerefMut)]
pub struct TranslationKeyframes(pub Vec<Vec3>);

/// Keyframes for animating [`Transform::scale`].
///
/// An example of a [`crate::AnimationClip`] that animates translation:
///
///     # use bevy_animation::{AnimationClip, AnimationTargetId, Interpolation};
///     # use bevy_animation::{VariableCurve, prelude::ScaleKeyframes};
///     # use bevy_core::Name;
///     # use bevy_math::Vec3;
///     # let animation_target_id = AnimationTargetId::from(&Name::new("Test"));
///     let mut animation_clip = AnimationClip::default();
///     animation_clip.add_curve_to_target(
///         animation_target_id,
///         VariableCurve::linear::<ScaleKeyframes>(
///             [0.0, 1.0],
///             [Vec3::ONE, Vec3::splat(2.0)],
///         ),
///     );
#[derive(Clone, Reflect, Debug, Deref, DerefMut)]
pub struct ScaleKeyframes(pub Vec<Vec3>);

/// Keyframes for animating [`Transform::rotation`].
///
/// An example of a [`crate::AnimationClip`] that animates translation:
///
///     # use bevy_animation::{AnimationClip, AnimationTargetId, Interpolation};
///     # use bevy_animation::{VariableCurve, prelude::RotationKeyframes};
///     # use bevy_core::Name;
///     # use bevy_math::Quat;
///     # use std::f32::consts::FRAC_PI_2;
///     # let animation_target_id = AnimationTargetId::from(&Name::new("Test"));
///     let mut animation_clip = AnimationClip::default();
///     animation_clip.add_curve_to_target(
///         animation_target_id,
///         VariableCurve::linear::<RotationKeyframes>(
///             [0.0, 1.0],
///             [Quat::from_rotation_x(FRAC_PI_2), Quat::from_rotation_y(FRAC_PI_2)],
///         ),
///     );
#[derive(Clone, Reflect, Debug, Deref, DerefMut)]
pub struct RotationKeyframes(pub Vec<Quat>);

/// Keyframes for animating [`MorphWeights`].
#[derive(Clone, Debug, Reflect)]
pub struct MorphWeightsKeyframes {
    /// The total number of morph weights.
    pub morph_target_count: usize,

    /// The morph weights.
    ///
    /// The length of this vector should be the total number of morph weights
    /// times the number of keyframes.
    pub weights: Vec<f32>,
}

impl<T> From<T> for TranslationKeyframes
where
    T: Into<Vec<Vec3>>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Keyframes for TranslationKeyframes {
    fn clone_value(&self) -> Box<dyn Keyframes> {
        Box::new(self.clone())
    }

    fn apply_single_keyframe<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let value = self
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        component.translation = Animatable::interpolate(&component.translation, value, weight);
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        animatable::interpolate_keyframes(
            &mut component.translation,
            &(*self)[..],
            interpolation,
            step_start,
            time,
            weight,
            duration,
        )
    }
}

impl<T> From<T> for ScaleKeyframes
where
    T: Into<Vec<Vec3>>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Keyframes for ScaleKeyframes {
    fn clone_value(&self) -> Box<dyn Keyframes> {
        Box::new(self.clone())
    }

    fn apply_single_keyframe<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let value = self
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        component.scale = Animatable::interpolate(&component.scale, value, weight);
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        animatable::interpolate_keyframes(
            &mut component.scale,
            &(*self)[..],
            interpolation,
            step_start,
            time,
            weight,
            duration,
        )
    }
}

impl<T> From<T> for RotationKeyframes
where
    T: Into<Vec<Quat>>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Keyframes for RotationKeyframes {
    fn clone_value(&self) -> Box<dyn Keyframes> {
        Box::new(self.clone())
    }

    fn apply_single_keyframe<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        let value = self
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        component.rotation = Animatable::interpolate(&component.rotation, value, weight);
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &self,
        transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = transform.ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>())
        })?;
        animatable::interpolate_keyframes(
            &mut component.rotation,
            &(*self)[..],
            interpolation,
            step_start,
            time,
            weight,
            duration,
        )
    }
}

impl<P, T> From<T> for AnimatablePropertyKeyframes<P>
where
    P: AnimatableProperty,
    T: Into<Vec<P::Property>>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl<P> Keyframes for AnimatablePropertyKeyframes<P>
where
    P: AnimatableProperty,
{
    fn clone_value(&self) -> Box<dyn Keyframes> {
        Box::new((*self).clone())
    }

    fn apply_single_keyframe<'a>(
        &self,
        _: Option<Mut<'a, Transform>>,
        mut entity: AnimationEntityMut<'a>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = entity.get_mut::<P::Component>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<P::Component>())
        })?;
        let property = P::get_mut(&mut component)
            .ok_or_else(|| AnimationEvaluationError::PropertyNotPresent(TypeId::of::<P>()))?;
        let value = self
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        <P::Property>::interpolate(property, value, weight);
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &self,
        _: Option<Mut<'a, Transform>>,
        mut entity: AnimationEntityMut<'a>,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut component = entity.get_mut::<P::Component>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<P::Component>())
        })?;
        let property = P::get_mut(&mut component)
            .ok_or_else(|| AnimationEvaluationError::PropertyNotPresent(TypeId::of::<P>()))?;
        animatable::interpolate_keyframes(
            property,
            self,
            interpolation,
            step_start,
            time,
            weight,
            duration,
        )?;
        Ok(())
    }
}

impl<A> GetKeyframe for [A]
where
    A: Animatable,
{
    type Output = A;

    fn get_keyframe(&self, index: usize) -> Option<&Self::Output> {
        self.get(index)
    }
}

impl<P> GetKeyframe for AnimatablePropertyKeyframes<P>
where
    P: AnimatableProperty,
{
    type Output = P::Property;

    fn get_keyframe(&self, index: usize) -> Option<&Self::Output> {
        self.get(index)
    }
}

/// Information needed to look up morph weight values in the flattened morph
/// weight keyframes vector.
struct GetMorphWeightKeyframe<'k> {
    /// The morph weights keyframe structure that we're animating.
    keyframes: &'k MorphWeightsKeyframes,
    /// The index of the morph target in that structure.
    morph_target_index: usize,
}

impl Keyframes for MorphWeightsKeyframes {
    fn clone_value(&self) -> Box<dyn Keyframes> {
        Box::new(self.clone())
    }

    fn apply_single_keyframe<'a>(
        &self,
        _: Option<Mut<'a, Transform>>,
        mut entity: AnimationEntityMut<'a>,
        weight: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut dest = entity.get_mut::<MorphWeights>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<MorphWeights>())
        })?;

        // TODO: Go 4 weights at a time to make better use of SIMD.
        for (morph_target_index, morph_weight) in dest.weights_mut().iter_mut().enumerate() {
            *morph_weight =
                f32::interpolate(morph_weight, &self.weights[morph_target_index], weight);
        }

        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &self,
        _: Option<Mut<'a, Transform>>,
        mut entity: AnimationEntityMut<'a>,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let mut dest = entity.get_mut::<MorphWeights>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<MorphWeights>())
        })?;

        // TODO: Go 4 weights at a time to make better use of SIMD.
        for (morph_target_index, morph_weight) in dest.weights_mut().iter_mut().enumerate() {
            animatable::interpolate_keyframes(
                morph_weight,
                &GetMorphWeightKeyframe {
                    keyframes: self,
                    morph_target_index,
                },
                interpolation,
                step_start,
                time,
                weight,
                duration,
            )?;
        }

        Ok(())
    }
}

impl GetKeyframe for GetMorphWeightKeyframe<'_> {
    type Output = f32;

    fn get_keyframe(&self, keyframe_index: usize) -> Option<&Self::Output> {
        self.keyframes
            .weights
            .as_slice()
            .get(keyframe_index * self.keyframes.morph_target_count + self.morph_target_index)
    }
}
