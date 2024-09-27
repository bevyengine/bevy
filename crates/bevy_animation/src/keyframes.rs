//! Keyframes of animation clips.

use core::{
    any::{Any, TypeId},
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, world::Mut};
use bevy_math::{Quat, Vec3};
use bevy_reflect::{FromReflect, Reflect, Reflectable, TypePath};
use bevy_render::mesh::morph::MorphWeights;
use bevy_transform::prelude::Transform;

use crate::{
    animatable,
    graph::AnimationNodeIndex,
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
/// This is the generic type of [`Keyframes`] that can animate any
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

/// A [`KeyframeEvaluator`] for [`AnimatableProperty`] instances.
///
/// You shouldn't ordinarily need to instantiate one of these manually. Bevy
/// will automatically do so when you use an [`AnimatablePropertyKeyframes`]
/// instance.
#[derive(Reflect)]
pub struct AnimatablePropertyKeyframeEvaluator<P>(
    SimpleKeyframeEvaluator<AnimatablePropertyKeyframes<P>, P::Property>,
)
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

impl<P> Clone for AnimatablePropertyKeyframeEvaluator<P>
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

impl<P> Debug for AnimatablePropertyKeyframeEvaluator<P>
where
    P: AnimatableProperty,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AnimatablePropertyKeyframeEvaluator")
            .field(&self.0)
            .finish()
    }
}

/// A low-level trait for use in [`crate::VariableCurve`] that allows a custom
/// [`KeyframeEvaluator`] to be constructed.
///
/// You should usually prefer to use [`AnimatablePropertyKeyframes`] instead of
/// implementing this trait manually. Only implement this trait if you need a
/// custom [`KeyframeEvaluator`]. See the [`KeyframeEvaluator`] documentation
/// for more information on when this might be necessary.
pub trait Keyframes: Reflect + Debug + Send + Sync {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn Keyframes>;

    /// Returns this value upcast to [`Any`].
    fn as_any(&self) -> &dyn Any;

    /// Returns a newly-instantiated [`KeyframeEvaluator`] for use with these
    /// keyframes.
    fn create_keyframe_evaluator(&self) -> Box<dyn KeyframeEvaluator>;
}

/// A low-level trait for use in [`crate::VariableCurve`] that provides fine
/// control over how animations are evaluated.
///
/// You can implement this trait when the generic
/// [`AnimatablePropertyKeyframeEvaluator`] isn't sufficiently-expressive for
/// your needs. For example, [`MorphWeights`] implements this trait instead of
/// using [`AnimatablePropertyKeyframeEvaluator`] because it needs to animate
/// arbitrarily many weights at once, which can't be done with [`Animatable`] as
/// that works on fixed-size values only.
///
/// If you implement this trait, you should also implement [`Keyframes`], as
/// that trait allows creating instances of this one.
///
/// Implementations of [`KeyframeEvaluator`] should maintain a *stack* of
/// (value, weight, node index) triples, as well as a *blend register*, which is
/// either a (value, weight) pair or empty. *Value* here refers to an instance
/// of the value being animated: for example, [`Vec3`] in the case of
/// translation keyframes.  The stack stores intermediate values generated while
/// evaluating the [`AnimationGraph`], while the blend register stores the
/// result of a blend operation.
pub trait KeyframeEvaluator: Reflect {
    /// Blends the top element of the stack with the blend register.
    ///
    /// The semantics of this method are as follows:
    ///
    /// 1. Pop the top element of the stack. Call its value vₘ and its weight
    ///    wₘ. If the stack was empty, return success.
    ///
    /// 2. If the blend register is empty, set the blend register value to vₘ
    ///    and the blend register weight to wₘ; then, return success.
    ///
    /// 3. If the blend register is nonempty, call its current value vₙ and its
    ///    current weight wₙ. Then, set the value of the blend register to
    ///    `interpolate(vₙ, vₘ, wₘ / (wₘ + wₙ))`, and set the weight of the blend
    ///    register to wₘ + wₙ.
    ///
    /// 4. Return success.
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError>;

    /// Pushes the current value of the blend register onto the stack.
    ///
    /// If the blend register is empty, this method does nothing successfully.
    /// Otherwise, this method pushes the current value of the blend register
    /// onto the stack, alongside the weight and graph node supplied to this
    /// function. The weight present in the blend register is discarded; only
    /// the weight parameter to this function is pushed onto the stack. The
    /// blend register is emptied after this process.
    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError>;

    /// Pops the top value off the stack and writes it into the appropriate component.
    ///
    /// If the stack is empty, this method does nothing successfully. Otherwise,
    /// it pops the top value off the stack, fetches the associated component
    /// from either the `transform` or `entity` values as appropriate, and
    /// updates the appropriate property with the value popped from the stack.
    /// The weight and node index associated with the popped stack element are
    /// discarded. After doing this, the stack is emptied.
    ///
    /// The property on the component must be overwritten with the value from
    /// the stack, not blended with it.
    fn commit<'a>(
        &mut self,
        transform: Option<Mut<'a, Transform>>,
        entity: AnimationEntityMut<'a>,
    ) -> Result<(), AnimationEvaluationError>;

    /// Pushes the value from the first keyframe (keyframe 0) onto the stack
    /// alongside the given weight and graph node.
    fn apply_single_keyframe(
        &mut self,
        keyframes: &dyn Keyframes,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError>;

    /// Samples the value in between two keyframes according to the given
    /// interpolation mode.
    ///
    /// This method first computes the interpolated value in between keyframe
    /// `step_start` and keyframe `step_start + 1` according to the
    /// `interpolation` mode and the `time` value. For example, an interpolation
    /// mode of `Interpolation::Linear` and a time value of 0.5 computes a value
    /// halfway between the keyframe `step_start` and the following one. Then,
    /// this method pushes the resulting value onto the stack, alongside the
    /// given `weight` and `graph_node`.
    ///
    /// Be sure not to confuse the `time` and `weight` values. The former
    /// determines the amount by which two *keyframes* are blended together,
    /// while `weight` ultimately determines how much the *stack values* will be
    /// blended together (see the definition of [`KeyframeEvaluator::blend`]).
    #[allow(clippy::too_many_arguments)]
    fn apply_tweened_keyframes(
        &mut self,
        keyframes: &dyn Keyframes,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
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

/// A [`KeyframeEvaluator`] for use with [`TranslationKeyframes`].
///
/// You shouldn't need to instantiate this manually; Bevy will automatically do
/// so.
#[derive(Clone, Default, Reflect, Debug)]
pub struct TranslationKeyframeEvaluator(SimpleKeyframeEvaluator<TranslationKeyframes, Vec3>);

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

/// A [`KeyframeEvaluator`] for use with [`ScaleKeyframes`].
///
/// You shouldn't need to instantiate this manually; Bevy will automatically do
/// so.
#[derive(Clone, Default, Reflect, Debug)]
pub struct ScaleKeyframeEvaluator(SimpleKeyframeEvaluator<ScaleKeyframes, Vec3>);

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

/// A [`KeyframeEvaluator`] for use with [`RotationKeyframes`].
///
/// You shouldn't need to instantiate this manually; Bevy will automatically do
/// so.
#[derive(Clone, Default, Reflect, Debug)]
pub struct RotationKeyframeEvaluator(SimpleKeyframeEvaluator<RotationKeyframes, Quat>);

/// Keyframes for animating [`MorphWeights`].
#[derive(Clone, Debug, Reflect)]
pub struct MorphWeightsKeyframes {
    /// The total number of morph weights.
    pub morph_target_count: u32,

    /// The morph weights.
    ///
    /// The length of this vector should be the total number of morph weights
    /// times the number of keyframes.
    pub weights: Vec<f32>,
}

/// A [`KeyframeEvaluator`] for use with [`MorphWeightsKeyframes`].
///
/// You shouldn't need to instantiate this manually; Bevy will automatically do
/// so.
#[derive(Clone, Debug, Reflect)]
pub struct MorphWeightsKeyframeEvaluator {
    /// The values of the stack, in which each element is a list of morph target
    /// weights.
    ///
    /// The stack elements are concatenated and tightly packed together.
    ///
    /// The number of elements in this stack will always be a multiple of
    /// [`Self::morph_target_count`].
    stack_morph_target_weights: Vec<f32>,

    /// The blend weights and graph node indices for each element of the stack.
    ///
    /// This should have as many elements as there are stack nodes. In other
    /// words, `Self::stack_morph_target_weights.len() *
    /// Self::morph_target_counts as usize ==
    /// Self::stack_blend_weights_and_graph_nodes`.
    stack_blend_weights_and_graph_nodes: Vec<(f32, AnimationNodeIndex)>,

    /// The morph target weights in the blend register, if any.
    ///
    /// This field should be ignored if [`Self::blend_register_blend_weight`] is
    /// `None`. If non-empty, it will always have [`Self::morph_target_count`]
    /// elements in it.
    blend_register_morph_target_weights: Vec<f32>,

    /// The weight in the blend register.
    ///
    /// This will be `None` if the blend register is empty. In that case,
    /// [`Self::blend_register_blend_weight`] will be empty.
    blend_register_blend_weight: Option<f32>,

    /// The number of morph targets that are to be animated.
    morph_target_count: u32,
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_keyframe_evaluator(&self) -> Box<dyn KeyframeEvaluator> {
        Box::new(TranslationKeyframeEvaluator::default())
    }
}

impl KeyframeEvaluator for TranslationKeyframeEvaluator {
    fn blend<'a>(
        &mut self,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.0.blend(graph_node)
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.0.push_blend(weight, graph_node)
    }

    fn commit<'a>(
        &mut self,
        mut transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
    ) -> Result<(), AnimationEvaluationError> {
        transform
            .as_mut()
            .ok_or_else(
                || AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>()),
            )?
            .translation = self
            .0
            .stack
            .pop()
            .ok_or_else(inconsistent::<TranslationKeyframes>)?
            .value;
        self.0.stack.clear();
        Ok(())
    }

    fn apply_single_keyframe(
        &mut self,
        keyframes: &dyn Keyframes,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let value = *Keyframes::as_any(keyframes)
            .downcast_ref::<TranslationKeyframes>()
            .unwrap()
            .0
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &mut self,
        keyframes: &dyn Keyframes,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let keyframes = Keyframes::as_any(keyframes)
            .downcast_ref::<TranslationKeyframes>()
            .unwrap();
        let value = animatable::interpolate_keyframes(
            &keyframes.0[..],
            interpolation,
            step_start,
            time,
            duration,
        )?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_keyframe_evaluator(&self) -> Box<dyn KeyframeEvaluator> {
        Box::new(ScaleKeyframeEvaluator::default())
    }
}

impl KeyframeEvaluator for ScaleKeyframeEvaluator {
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.0.blend(graph_node)
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.0.push_blend(weight, graph_node)
    }

    fn commit<'a>(
        &mut self,
        mut transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
    ) -> Result<(), AnimationEvaluationError> {
        transform
            .as_mut()
            .ok_or_else(
                || AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>()),
            )?
            .scale = self
            .0
            .stack
            .pop()
            .ok_or_else(inconsistent::<ScaleKeyframes>)?
            .value;
        self.0.stack.clear();
        Ok(())
    }

    fn apply_single_keyframe(
        &mut self,
        keyframes: &dyn Keyframes,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let value = *Keyframes::as_any(keyframes)
            .downcast_ref::<ScaleKeyframes>()
            .unwrap()
            .0
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &mut self,
        keyframes: &dyn Keyframes,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let keyframes = Keyframes::as_any(keyframes)
            .downcast_ref::<ScaleKeyframes>()
            .unwrap();
        let value = animatable::interpolate_keyframes(
            &keyframes.0[..],
            interpolation,
            step_start,
            time,
            duration,
        )?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_keyframe_evaluator(&self) -> Box<dyn KeyframeEvaluator> {
        Box::new(RotationKeyframeEvaluator::default())
    }
}

impl KeyframeEvaluator for RotationKeyframeEvaluator {
    fn commit<'a>(
        &mut self,
        mut transform: Option<Mut<'a, Transform>>,
        _: AnimationEntityMut<'a>,
    ) -> Result<(), AnimationEvaluationError> {
        transform
            .as_mut()
            .ok_or_else(
                || AnimationEvaluationError::ComponentNotPresent(TypeId::of::<Transform>()),
            )?
            .rotation = self
            .0
            .stack
            .pop()
            .ok_or_else(inconsistent::<RotationKeyframes>)?
            .value;
        self.0.stack.clear();
        Ok(())
    }

    fn apply_single_keyframe(
        &mut self,
        keyframes: &dyn Keyframes,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let value = Keyframes::as_any(keyframes)
            .downcast_ref::<RotationKeyframes>()
            .unwrap()
            .0
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value: *value,
            weight,
            graph_node,
        });
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &mut self,
        keyframes: &dyn Keyframes,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let keyframes = Keyframes::as_any(keyframes)
            .downcast_ref::<RotationKeyframes>()
            .unwrap();
        let value = animatable::interpolate_keyframes(
            &keyframes.0[..],
            interpolation,
            step_start,
            time,
            duration,
        )?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
    }

    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.0.blend(graph_node)
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.0.push_blend(weight, graph_node)
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_keyframe_evaluator(&self) -> Box<dyn KeyframeEvaluator> {
        Box::new(AnimatablePropertyKeyframeEvaluator::<P>(
            SimpleKeyframeEvaluator::default(),
        ))
    }
}

impl<P> KeyframeEvaluator for AnimatablePropertyKeyframeEvaluator<P>
where
    P: AnimatableProperty,
{
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.0.blend(graph_node)
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.0.push_blend(weight, graph_node)
    }

    fn commit<'a>(
        &mut self,
        _: Option<Mut<'a, Transform>>,
        mut entity: AnimationEntityMut<'a>,
    ) -> Result<(), AnimationEvaluationError> {
        *P::get_mut(&mut *entity.get_mut::<P::Component>().ok_or_else(|| {
            AnimationEvaluationError::ComponentNotPresent(TypeId::of::<P::Component>())
        })?)
        .ok_or_else(|| {
            AnimationEvaluationError::PropertyNotPresent(TypeId::of::<P::Property>())
        })? = self.0.stack.pop().ok_or_else(inconsistent::<P>)?.value;
        self.0.stack.clear();
        Ok(())
    }

    fn apply_single_keyframe(
        &mut self,
        keyframes: &dyn Keyframes,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let value = (*Keyframes::as_any(keyframes)
            .downcast_ref::<AnimatablePropertyKeyframes<P>>()
            .unwrap()
            .0
            .first()
            .ok_or(AnimationEvaluationError::KeyframeNotPresent(0))?)
        .clone();
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &mut self,
        keyframes: &dyn Keyframes,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let keyframes = (*Keyframes::as_any(keyframes)
            .downcast_ref::<AnimatablePropertyKeyframes<P>>()
            .unwrap())
        .clone();
        let value = animatable::interpolate_keyframes(
            &keyframes.0[..],
            interpolation,
            step_start,
            time,
            duration,
        )?;
        self.0.stack.push(SimpleKeyframeEvaluatorStackElement {
            value,
            weight,
            graph_node,
        });
        Ok(())
    }
}

#[derive(Clone, Reflect, Debug)]
struct SimpleKeyframeEvaluator<K, P>
where
    K: Keyframes,
    P: Animatable,
{
    stack: Vec<SimpleKeyframeEvaluatorStackElement<P>>,
    blend_register: Option<(P, f32)>,
    #[reflect(ignore)]
    phantom: PhantomData<K>,
}

#[derive(Clone, Reflect, Debug)]
struct SimpleKeyframeEvaluatorStackElement<P>
where
    P: Animatable,
{
    value: P,
    weight: f32,
    graph_node: AnimationNodeIndex,
}

impl<K, P> Default for SimpleKeyframeEvaluator<K, P>
where
    K: Keyframes,
    P: Animatable,
{
    fn default() -> Self {
        SimpleKeyframeEvaluator {
            stack: vec![],
            blend_register: None,
            phantom: PhantomData,
        }
    }
}

impl<K, P> SimpleKeyframeEvaluator<K, P>
where
    K: Keyframes,
    P: Animatable + Debug,
{
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        let Some(top) = self.stack.last() else {
            return Ok(());
        };
        if top.graph_node != graph_node {
            return Ok(());
        }

        let SimpleKeyframeEvaluatorStackElement {
            value: value_to_blend,
            weight: weight_to_blend,
            graph_node: _,
        } = self.stack.pop().unwrap();

        match self.blend_register {
            None => self.blend_register = Some((value_to_blend, weight_to_blend)),
            Some((ref mut current_value, ref mut current_weight)) => {
                *current_weight += weight_to_blend;
                *current_value = P::interpolate(
                    current_value,
                    &value_to_blend,
                    weight_to_blend / *current_weight,
                );
            }
        }

        Ok(())
    }

    fn push_blend(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        if let Some((value, _)) = self.blend_register.take() {
            self.stack.push(SimpleKeyframeEvaluatorStackElement {
                value,
                weight,
                graph_node,
            });
        }
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_keyframe_evaluator(&self) -> Box<dyn KeyframeEvaluator> {
        Box::new(MorphWeightsKeyframeEvaluator {
            stack_morph_target_weights: vec![],
            stack_blend_weights_and_graph_nodes: vec![],
            blend_register_morph_target_weights: vec![],
            blend_register_blend_weight: None,
            morph_target_count: self.morph_target_count,
        })
    }
}

impl KeyframeEvaluator for MorphWeightsKeyframeEvaluator {
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        let Some(&(_, top_graph_node)) = self.stack_blend_weights_and_graph_nodes.last() else {
            return Ok(());
        };
        if top_graph_node != graph_node {
            return Ok(());
        }

        let (weight_to_blend, _) = self.stack_blend_weights_and_graph_nodes.pop().unwrap();
        let stack_iter = self
            .stack_morph_target_weights
            .drain((self.stack_morph_target_weights.len() - self.morph_target_count as usize)..);

        match self.blend_register_blend_weight {
            None => {
                self.blend_register_blend_weight = Some(weight_to_blend);
                self.blend_register_morph_target_weights.clear();
                self.blend_register_morph_target_weights.extend(stack_iter);
            }

            Some(ref mut current_weight) => {
                *current_weight += weight_to_blend;
                for (dest, src) in self
                    .blend_register_morph_target_weights
                    .iter_mut()
                    .zip(stack_iter)
                {
                    *dest = f32::interpolate(dest, &src, weight_to_blend / *current_weight);
                }
            }
        }

        Ok(())
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        if self.blend_register_blend_weight.take().is_some() {
            self.stack_morph_target_weights
                .append(&mut self.blend_register_morph_target_weights);
            self.stack_blend_weights_and_graph_nodes
                .push((weight, graph_node));
        }
        Ok(())
    }

    fn commit<'a>(
        &mut self,
        _: Option<Mut<'a, Transform>>,
        mut entity: AnimationEntityMut<'a>,
    ) -> Result<(), AnimationEvaluationError> {
        for (dest, src) in entity
            .get_mut::<MorphWeights>()
            .ok_or_else(|| {
                AnimationEvaluationError::ComponentNotPresent(TypeId::of::<MorphWeights>())
            })?
            .weights_mut()
            .iter_mut()
            .zip(
                self.stack_morph_target_weights
                    [(self.stack_morph_target_weights.len() - self.morph_target_count as usize)..]
                    .iter(),
            )
        {
            *dest = *src;
        }
        self.stack_morph_target_weights.clear();
        self.stack_blend_weights_and_graph_nodes.clear();
        Ok(())
    }

    fn apply_single_keyframe(
        &mut self,
        keyframes: &dyn Keyframes,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let morph_weights_keyframes = Keyframes::as_any(keyframes)
            .downcast_ref::<MorphWeightsKeyframes>()
            .unwrap();
        if morph_weights_keyframes.weights.len()
            < (morph_weights_keyframes.morph_target_count as usize)
        {
            return Err(AnimationEvaluationError::KeyframeNotPresent(0));
        }
        self.stack_morph_target_weights.extend(
            morph_weights_keyframes.weights
                [0..(morph_weights_keyframes.morph_target_count as usize)]
                .iter()
                .cloned(),
        );
        self.stack_blend_weights_and_graph_nodes
            .push((weight, graph_node));
        Ok(())
    }

    fn apply_tweened_keyframes<'a>(
        &mut self,
        keyframes: &dyn Keyframes,
        interpolation: Interpolation,
        step_start: usize,
        time: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
        duration: f32,
    ) -> Result<(), AnimationEvaluationError> {
        let morph_weights_keyframes = Keyframes::as_any(keyframes)
            .downcast_ref::<MorphWeightsKeyframes>()
            .unwrap();

        // TODO: Go 4 weights at a time to make better use of SIMD.
        self.stack_morph_target_weights
            .reserve(self.morph_target_count as usize);
        for morph_target_index in 0..self.morph_target_count {
            self.stack_morph_target_weights
                .push(animatable::interpolate_keyframes(
                    &GetMorphWeightKeyframe {
                        keyframes: morph_weights_keyframes,
                        morph_target_index: morph_target_index as usize,
                    },
                    interpolation,
                    step_start,
                    time,
                    duration,
                )?);
        }

        self.stack_blend_weights_and_graph_nodes
            .push((weight, graph_node));

        Ok(())
    }
}

impl GetKeyframe for GetMorphWeightKeyframe<'_> {
    type Output = f32;

    fn get_keyframe(&self, keyframe_index: usize) -> Option<&Self::Output> {
        self.keyframes.weights.as_slice().get(
            keyframe_index * self.keyframes.morph_target_count as usize + self.morph_target_index,
        )
    }
}

fn inconsistent<P>() -> AnimationEvaluationError
where
    P: 'static,
{
    AnimationEvaluationError::InconsistentKeyframeImplementation(TypeId::of::<P>())
}
