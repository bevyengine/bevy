//! The [`AnimationCurve`] trait and adaptors that allow curves to implement it.
//!
//! # Overview
//!
//! The flow of curves into the animation system generally begins with something that
//! implements the [`Curve`] trait. Let's imagine, for example, that we have some
//! `Curve<Vec3>` that we want to use to animate something. That could be defined in
//! a number of different ways, but let's imagine that we've defined it [using a function]:
//!
//!     # use bevy_math::curve::{Curve, Interval, FunctionCurve};
//!     # use bevy_math::vec3;
//!     let wobble_curve = FunctionCurve::new(
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
//! For instance, let's imagine that we want to use the `Vec3` output
//! from our curve to animate the [translation component of a `Transform`]. For this, there is
//! the adaptor [`AnimatableCurve`], which wraps any [`Curve`] and [`AnimatableProperty`] and turns it into an
//! [`AnimationCurve`] that will use the given curve to animate the entity's property:
//!
//!     # use bevy_math::curve::{Curve, Interval, FunctionCurve};
//!     # use bevy_math::vec3;
//!     # use bevy_transform::components::Transform;
//!     # use bevy_animation::{animated_field, animation_curves::*};
//!     # let wobble_curve = FunctionCurve::new(
//!     #     Interval::UNIT,
//!     #     |t| vec3(t.cos(), 0.0, 0.0)
//!     # );
//!     let wobble_animation = AnimatableCurve::new(animated_field!(Transform::translation), wobble_curve);
//!
//! And finally, this [`AnimationCurve`] needs to be added to an [`AnimationClip`] in order to
//! actually animate something. This is what that looks like:
//!
//!     # use bevy_math::curve::{Curve, Interval, FunctionCurve};
//!     # use bevy_animation::{AnimationClip, AnimationTargetId, animated_field, animation_curves::*};
//!     # use bevy_transform::components::Transform;
//!     # use bevy_ecs::name::Name;
//!     # use bevy_math::vec3;
//!     # let wobble_curve = FunctionCurve::new(
//!     #     Interval::UNIT,
//!     #     |t| { vec3(t.cos(), 0.0, 0.0) },
//!     # );
//!     # let wobble_animation = AnimatableCurve::new(animated_field!(Transform::translation), wobble_curve);
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
//! ## Animated Fields
//!
//! The [`animated_field`] macro (which returns an [`AnimatedField`]), in combination with [`AnimatableCurve`]
//! is the easiest way to make an animation curve (see the example above).
//!
//! This will select a field on a component and pass it to a [`Curve`] with a type that matches the field.
//!
//! ## Animatable Properties
//!
//! Animation of arbitrary aspects of entities can be accomplished using [`AnimatableProperty`] in
//! conjunction with [`AnimatableCurve`]. See the documentation [there] for details.
//!
//! ## Custom [`AnimationCurve`] and [`AnimationCurveEvaluator`]
//!
//! This is the lowest-level option with the most control, but it is also the most complicated.
//!
//! [using a function]: bevy_math::curve::FunctionCurve
//! [translation component of a `Transform`]: bevy_transform::prelude::Transform::translation
//! [`AnimationClip`]: crate::AnimationClip
//! [there]: AnimatableProperty
//! [`animated_field`]: crate::animated_field

use core::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use crate::{
    graph::AnimationNodeIndex,
    prelude::{Animatable, BlendInput},
    AnimationEntityMut, AnimationEvaluationError,
};
use bevy_ecs::component::{Component, Mutable};
use bevy_math::curve::{
    cores::{UnevenCore, UnevenCoreError},
    iterable::IterableCurve,
    Curve, Interval,
};
use bevy_mesh::morph::MorphWeights;
use bevy_platform::hash::Hashed;
use bevy_reflect::{FromReflect, Reflect, Reflectable, TypeInfo, Typed};
use downcast_rs::{impl_downcast, Downcast};

/// A trait for exposing a value in an entity so that it can be animated.
///
/// `AnimatableProperty` allows any value contained in an entity to be animated
/// as long as it can be obtained by mutable reference. This makes it more
/// flexible than [`animated_field`].
///
/// [`animated_field`]: crate::animated_field
///
/// Here, `AnimatableProperty` is used to animate a value inside an `Option`,
/// returning an error if the option is `None`.
///
///     # use bevy_animation::{prelude::AnimatableProperty, AnimationEntityMut, AnimationEvaluationError, animation_curves::EvaluatorId};
///     # use bevy_ecs::component::Component;
///     # use std::any::TypeId;
///     #[derive(Component)]
///     struct ExampleComponent {
///         power_level: Option<f32>
///     }
///
///     #[derive(Clone)]
///     struct PowerLevelProperty;
///
///     impl AnimatableProperty for PowerLevelProperty {
///         type Property = f32;
///         fn get_mut<'a>(
///             &self,
///             entity: &'a mut AnimationEntityMut
///         ) -> Result<&'a mut Self::Property, AnimationEvaluationError> {
///             let component = entity
///                 .get_mut::<ExampleComponent>()
///                 .ok_or(AnimationEvaluationError::ComponentNotPresent(
///                   TypeId::of::<ExampleComponent>()
///                 ))?
///                 .into_inner();
///             component.power_level.as_mut().ok_or(AnimationEvaluationError::PropertyNotPresent(
///                 TypeId::of::<Option<f32>>()
///             ))
///         }
///
///         fn evaluator_id(&self) -> EvaluatorId {
///             EvaluatorId::Type(TypeId::of::<Self>())
///         }
///     }
///
///
/// You can then create an [`AnimatableCurve`] to animate this property like so:
///
///     # use bevy_animation::{VariableCurve, AnimationEntityMut, AnimationEvaluationError, animation_curves::EvaluatorId};
///     # use bevy_animation::prelude::{AnimatableProperty, AnimatableKeyframeCurve, AnimatableCurve};
///     # use bevy_ecs::{name::Name, component::Component};
///     # use std::any::TypeId;
///     # #[derive(Component)]
///     # struct ExampleComponent { power_level: Option<f32> }
///     # #[derive(Clone)]
///     # struct PowerLevelProperty;
///     # impl AnimatableProperty for PowerLevelProperty {
///     #     type Property = f32;
///     #     fn get_mut<'a>(
///     #         &self,
///     #         entity: &'a mut AnimationEntityMut
///     #     ) -> Result<&'a mut Self::Property, AnimationEvaluationError> {
///     #         let component = entity
///     #             .get_mut::<ExampleComponent>()
///     #             .ok_or(AnimationEvaluationError::ComponentNotPresent(
///     #               TypeId::of::<ExampleComponent>()
///     #             ))?
///     #             .into_inner();
///     #         component.power_level.as_mut().ok_or(AnimationEvaluationError::PropertyNotPresent(
///     #             TypeId::of::<Option<f32>>()
///     #         ))
///     #     }
///     #     fn evaluator_id(&self) -> EvaluatorId {
///     #         EvaluatorId::Type(TypeId::of::<Self>())
///     #     }
///     # }
///     AnimatableCurve::new(
///         PowerLevelProperty,
///         AnimatableKeyframeCurve::new([
///             (0.0, 0.0),
///             (1.0, 9001.0),
///         ]).expect("Failed to create power level curve")
///     );
pub trait AnimatableProperty: Send + Sync + 'static {
    /// The animated property type.
    type Property: Animatable;

    /// Retrieves the property from the given `entity`.
    fn get_mut<'a>(
        &self,
        entity: &'a mut AnimationEntityMut,
    ) -> Result<&'a mut Self::Property, AnimationEvaluationError>;

    /// The [`EvaluatorId`] used to look up the [`AnimationCurveEvaluator`] for this [`AnimatableProperty`].
    /// For a given animated property, this ID should always be the same to allow things like animation blending to occur.
    fn evaluator_id(&self) -> EvaluatorId<'_>;
}

/// A [`Component`] field that can be animated, defined by a function that reads the component and returns
/// the accessed field / property.
///
/// The best way to create an instance of this type is via the [`animated_field`] macro.
///
/// `C` is the component being animated, `A` is the type of the [`Animatable`] field on the component, and `F` is an accessor
/// function that accepts a reference to `C` and retrieves the field `A`.
///
/// [`animated_field`]: crate::animated_field
#[derive(Clone)]
pub struct AnimatedField<C, A, F: Fn(&mut C) -> &mut A> {
    func: F,
    /// A pre-hashed (component-type-id, reflected-field-index) pair, uniquely identifying a component field
    evaluator_id: Hashed<(TypeId, usize)>,
    marker: PhantomData<(C, A)>,
}

impl<C, A, F> AnimatableProperty for AnimatedField<C, A, F>
where
    C: Component<Mutability = Mutable>,
    A: Animatable + Clone + Sync + Debug,
    F: Fn(&mut C) -> &mut A + Send + Sync + 'static,
{
    type Property = A;
    fn get_mut<'a>(
        &self,
        entity: &'a mut AnimationEntityMut,
    ) -> Result<&'a mut A, AnimationEvaluationError> {
        let c = entity
            .get_mut::<C>()
            .ok_or_else(|| AnimationEvaluationError::ComponentNotPresent(TypeId::of::<C>()))?;
        Ok((self.func)(c.into_inner()))
    }

    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::ComponentField(&self.evaluator_id)
    }
}

impl<C: Typed, P, F: Fn(&mut C) -> &mut P + 'static> AnimatedField<C, P, F> {
    /// Creates a new instance of [`AnimatedField`]. This operates under the assumption that
    /// `C` is a reflect-able struct, and that `field_name` is a valid field on that struct.
    ///
    /// # Panics
    /// If the type of `C` is not a struct or if the `field_name` does not exist.
    pub fn new_unchecked(field_name: &str, func: F) -> Self {
        let field_index;
        if let TypeInfo::Struct(struct_info) = C::type_info() {
            field_index = struct_info
                .index_of(field_name)
                .expect("Field name should exist");
        } else if let TypeInfo::TupleStruct(struct_info) = C::type_info() {
            field_index = field_name
                .parse()
                .expect("Field name should be a valid tuple index");
            if field_index >= struct_info.field_len() {
                panic!("Field name should be a valid tuple index");
            }
        } else {
            panic!("Only structs are supported in `AnimatedField::new_unchecked`")
        }

        Self {
            func,
            evaluator_id: Hashed::new((TypeId::of::<C>(), field_index)),
            marker: PhantomData,
        }
    }
}

/// This trait collects the additional requirements on top of [`Curve<T>`] needed for a
/// curve to be used as an [`AnimationCurve`].
pub trait AnimationCompatibleCurve<T>: Curve<T> + Debug + Clone + Reflectable {}

impl<T, C> AnimationCompatibleCurve<T> for C where C: Curve<T> + Debug + Clone + Reflectable {}

/// This type allows the conversion of a [curve] valued in the [property type] of an
/// [`AnimatableProperty`] into an [`AnimationCurve`] which animates that property.
///
/// [curve]: Curve
/// [property type]: AnimatableProperty::Property
#[derive(Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct AnimatableCurve<P, C> {
    /// The property selector, which defines what component to access and how to access
    /// a property on that component.
    pub property: P,

    /// The inner [curve] whose values are used to animate the property.
    ///
    /// [curve]: Curve
    pub curve: C,
}

/// An [`AnimatableCurveEvaluator`] for [`AnimatableProperty`] instances.
///
/// You shouldn't ordinarily need to instantiate one of these manually. Bevy
/// will automatically do so when you use an [`AnimatableCurve`] instance.
#[derive(Reflect)]
pub struct AnimatableCurveEvaluator<A: Animatable> {
    evaluator: BasicAnimationCurveEvaluator<A>,
    property: Box<dyn AnimatableProperty<Property = A>>,
}

impl<P, C> AnimatableCurve<P, C>
where
    P: AnimatableProperty,
    C: AnimationCompatibleCurve<P::Property>,
{
    /// Create an [`AnimatableCurve`] (and thus an [`AnimationCurve`]) from a curve
    /// valued in an [animatable property].
    ///
    /// [animatable property]: AnimatableProperty::Property
    pub fn new(property: P, curve: C) -> Self {
        Self { property, curve }
    }
}

impl<P, C> Clone for AnimatableCurve<P, C>
where
    C: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            curve: self.curve.clone(),
            property: self.property.clone(),
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

impl<P: Send + Sync + 'static, C> AnimationCurve for AnimatableCurve<P, C>
where
    P: AnimatableProperty + Clone,
    C: AnimationCompatibleCurve<P::Property> + Clone,
{
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }

    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    fn evaluator_id(&self) -> EvaluatorId<'_> {
        self.property.evaluator_id()
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(AnimatableCurveEvaluator::<P::Property> {
            evaluator: BasicAnimationCurveEvaluator::default(),
            property: Box::new(self.property.clone()),
        })
    }

    fn apply(
        &self,
        curve_evaluator: &mut dyn AnimationCurveEvaluator,
        t: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let curve_evaluator = curve_evaluator
            .downcast_mut::<AnimatableCurveEvaluator<P::Property>>()
            .unwrap();
        let value = self.curve.sample_clamped(t);
        curve_evaluator
            .evaluator
            .stack
            .push(BasicAnimationCurveEvaluatorStackElement {
                value,
                weight,
                graph_node,
            });
        Ok(())
    }
}

impl<A: Animatable> AnimationCurveEvaluator for AnimatableCurveEvaluator<A> {
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.evaluator.combine(graph_node, /*additive=*/ false)
    }

    fn add(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.evaluator.combine(graph_node, /*additive=*/ true)
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.evaluator.push_blend_register(weight, graph_node)
    }

    fn commit(&mut self, mut entity: AnimationEntityMut) -> Result<(), AnimationEvaluationError> {
        let property = self.property.get_mut(&mut entity)?;
        *property = self
            .evaluator
            .stack
            .pop()
            .ok_or_else(inconsistent::<AnimatableCurveEvaluator<A>>)?
            .value;
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

#[derive(Reflect)]
struct WeightsCurveEvaluator {
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
    /// [`Self::blend_register_morph_target_weights`] will be empty.
    blend_register_blend_weight: Option<f32>,

    /// The number of morph targets that are to be animated.
    morph_target_count: Option<u32>,
}

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

    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::Type(TypeId::of::<WeightsCurveEvaluator>())
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(WeightsCurveEvaluator {
            stack_morph_target_weights: vec![],
            stack_blend_weights_and_graph_nodes: vec![],
            blend_register_morph_target_weights: vec![],
            blend_register_blend_weight: None,
            morph_target_count: None,
        })
    }

    fn apply(
        &self,
        curve_evaluator: &mut dyn AnimationCurveEvaluator,
        t: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let curve_evaluator = curve_evaluator
            .downcast_mut::<WeightsCurveEvaluator>()
            .unwrap();

        let prev_morph_target_weights_len = curve_evaluator.stack_morph_target_weights.len();
        curve_evaluator
            .stack_morph_target_weights
            .extend(self.0.sample_iter_clamped(t));
        curve_evaluator.morph_target_count = Some(
            (curve_evaluator.stack_morph_target_weights.len() - prev_morph_target_weights_len)
                as u32,
        );

        curve_evaluator
            .stack_blend_weights_and_graph_nodes
            .push((weight, graph_node));
        Ok(())
    }
}

impl WeightsCurveEvaluator {
    fn combine(
        &mut self,
        graph_node: AnimationNodeIndex,
        additive: bool,
    ) -> Result<(), AnimationEvaluationError> {
        let Some(&(_, top_graph_node)) = self.stack_blend_weights_and_graph_nodes.last() else {
            return Ok(());
        };
        if top_graph_node != graph_node {
            return Ok(());
        }

        let (weight_to_blend, _) = self.stack_blend_weights_and_graph_nodes.pop().unwrap();
        let stack_iter = self.stack_morph_target_weights.drain(
            (self.stack_morph_target_weights.len() - self.morph_target_count.unwrap() as usize)..,
        );

        match self.blend_register_blend_weight {
            None => {
                self.blend_register_blend_weight = Some(weight_to_blend);
                self.blend_register_morph_target_weights.clear();

                // In the additive case, the values pushed onto the blend register need
                // to be scaled by the weight.
                if additive {
                    self.blend_register_morph_target_weights
                        .extend(stack_iter.map(|m| m * weight_to_blend));
                } else {
                    self.blend_register_morph_target_weights.extend(stack_iter);
                }
            }

            Some(ref mut current_weight) => {
                *current_weight += weight_to_blend;
                for (dest, src) in self
                    .blend_register_morph_target_weights
                    .iter_mut()
                    .zip(stack_iter)
                {
                    if additive {
                        *dest += src * weight_to_blend;
                    } else {
                        *dest = f32::interpolate(dest, &src, weight_to_blend / *current_weight);
                    }
                }
            }
        }

        Ok(())
    }
}

impl AnimationCurveEvaluator for WeightsCurveEvaluator {
    fn blend(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.combine(graph_node, /*additive=*/ false)
    }

    fn add(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.combine(graph_node, /*additive=*/ true)
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

    fn commit(&mut self, mut entity: AnimationEntityMut) -> Result<(), AnimationEvaluationError> {
        if self.stack_morph_target_weights.is_empty() {
            return Ok(());
        }

        // Compute the index of the first morph target in the last element of
        // the stack.
        let index_of_first_morph_target =
            self.stack_morph_target_weights.len() - self.morph_target_count.unwrap() as usize;

        for (dest, src) in entity
            .get_mut::<MorphWeights>()
            .ok_or_else(|| {
                AnimationEvaluationError::ComponentNotPresent(TypeId::of::<MorphWeights>())
            })?
            .weights_mut()
            .iter_mut()
            .zip(self.stack_morph_target_weights[index_of_first_morph_target..].iter())
        {
            *dest = *src;
        }
        self.stack_morph_target_weights.clear();
        self.stack_blend_weights_and_graph_nodes.clear();
        Ok(())
    }
}

#[derive(Reflect)]
struct BasicAnimationCurveEvaluator<A>
where
    A: Animatable,
{
    stack: Vec<BasicAnimationCurveEvaluatorStackElement<A>>,
    blend_register: Option<(A, f32)>,
}

#[derive(Reflect)]
struct BasicAnimationCurveEvaluatorStackElement<A>
where
    A: Animatable,
{
    value: A,
    weight: f32,
    graph_node: AnimationNodeIndex,
}

impl<A> Default for BasicAnimationCurveEvaluator<A>
where
    A: Animatable,
{
    fn default() -> Self {
        BasicAnimationCurveEvaluator {
            stack: vec![],
            blend_register: None,
        }
    }
}

impl<A> BasicAnimationCurveEvaluator<A>
where
    A: Animatable,
{
    fn combine(
        &mut self,
        graph_node: AnimationNodeIndex,
        additive: bool,
    ) -> Result<(), AnimationEvaluationError> {
        let Some(top) = self.stack.last() else {
            return Ok(());
        };
        if top.graph_node != graph_node {
            return Ok(());
        }

        let BasicAnimationCurveEvaluatorStackElement {
            value: value_to_blend,
            weight: weight_to_blend,
            graph_node: _,
        } = self.stack.pop().unwrap();

        match self.blend_register.take() {
            None => {
                self.initialize_blend_register(value_to_blend, weight_to_blend, additive);
            }
            Some((mut current_value, mut current_weight)) => {
                current_weight += weight_to_blend;

                if additive {
                    current_value = A::blend(
                        [
                            BlendInput {
                                weight: 1.0,
                                value: current_value,
                                additive: true,
                            },
                            BlendInput {
                                weight: weight_to_blend,
                                value: value_to_blend,
                                additive: true,
                            },
                        ]
                        .into_iter(),
                    );
                } else {
                    current_value = A::interpolate(
                        &current_value,
                        &value_to_blend,
                        weight_to_blend / current_weight,
                    );
                }

                self.blend_register = Some((current_value, current_weight));
            }
        }

        Ok(())
    }

    fn initialize_blend_register(&mut self, value: A, weight: f32, additive: bool) {
        if additive {
            let scaled_value = A::blend(
                [BlendInput {
                    weight,
                    value,
                    additive: true,
                }]
                .into_iter(),
            );
            self.blend_register = Some((scaled_value, weight));
        } else {
            self.blend_register = Some((value, weight));
        }
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        if let Some((value, _)) = self.blend_register.take() {
            self.stack.push(BasicAnimationCurveEvaluatorStackElement {
                value,
                weight,
                graph_node,
            });
        }
        Ok(())
    }
}

/// A low-level trait that provides control over how curves are actually applied
/// to entities by the animation system.
///
/// Typically, this will not need to be implemented manually, since it is
/// automatically implemented by [`AnimatableCurve`] and other curves used by
/// the animation system (e.g. those that animate parts of transforms or morph
/// weights). However, this can be implemented manually when `AnimatableCurve`
/// is not sufficiently expressive.
///
/// In many respects, this behaves like a type-erased form of [`Curve`], where
/// the output type of the curve is remembered only in the components that are
/// mutated in the implementation of [`apply`].
///
/// [`apply`]: AnimationCurve::apply
pub trait AnimationCurve: Debug + Send + Sync + 'static {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn AnimationCurve>;

    /// The range of times for which this animation is defined.
    fn domain(&self) -> Interval;

    /// Returns the type ID of the [`AnimationCurveEvaluator`].
    ///
    /// This must match the type returned by [`Self::create_evaluator`]. It must
    /// be a single type that doesn't depend on the type of the curve.
    fn evaluator_id(&self) -> EvaluatorId<'_>;

    /// Returns a newly-instantiated [`AnimationCurveEvaluator`] for use with
    /// this curve.
    ///
    /// All curve types must return the same type of
    /// [`AnimationCurveEvaluator`]. The returned value must match the type
    /// returned by [`Self::evaluator_id`].
    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator>;

    /// Samples the curve at the given time `t`, and pushes the sampled value
    /// onto the evaluation stack of the `curve_evaluator`.
    ///
    /// The `curve_evaluator` parameter points to the value returned by
    /// [`Self::create_evaluator`], upcast to an `&mut dyn
    /// AnimationCurveEvaluator`. Typically, implementations of [`Self::apply`]
    /// will want to downcast the `curve_evaluator` parameter to the concrete
    /// type [`Self::evaluator_id`] in order to push values of the appropriate
    /// type onto its evaluation stack.
    ///
    /// Be sure not to confuse the `t` and `weight` values. The former
    /// determines the position at which the *curve* is sampled, while `weight`
    /// ultimately determines how much the *stack values* will be blended
    /// together (see the definition of [`AnimationCurveEvaluator::blend`]).
    fn apply(
        &self,
        curve_evaluator: &mut dyn AnimationCurveEvaluator,
        t: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError>;
}

/// The [`EvaluatorId`] is used to look up the [`AnimationCurveEvaluator`] for an [`AnimatableProperty`].
/// For a given animated property, this ID should always be the same to allow things like animation blending to occur.
#[derive(Clone)]
pub enum EvaluatorId<'a> {
    /// Corresponds to a specific field on a specific component type.
    /// The `TypeId` should correspond to the component type, and the `usize`
    /// should correspond to the Reflect-ed field index of the field.
    //
    // IMPLEMENTATION NOTE: The Hashed<(TypeId, usize) is intentionally cheap to clone, as it will be cloned per frame by the evaluator
    // Switching the field index `usize` for something like a field name `String` would probably be too expensive to justify
    ComponentField(&'a Hashed<(TypeId, usize)>),
    /// Corresponds to a custom property of a given type. This should be the [`TypeId`]
    /// of the custom [`AnimatableProperty`].
    Type(TypeId),
}

/// A low-level trait for use in [`crate::VariableCurve`] that provides fine
/// control over how animations are evaluated.
///
/// You can implement this trait when the generic [`AnimatableCurveEvaluator`]
/// isn't sufficiently-expressive for your needs. For example, [`MorphWeights`]
/// implements this trait instead of using [`AnimatableCurveEvaluator`] because
/// it needs to animate arbitrarily many weights at once, which can't be done
/// with [`Animatable`] as that works on fixed-size values only.
///
/// If you implement this trait, you should also implement [`AnimationCurve`] on
/// your curve type, as that trait allows creating instances of this one.
///
/// Implementations of [`AnimatableCurveEvaluator`] should maintain a *stack* of
/// (value, weight, node index) triples, as well as a *blend register*, which is
/// either a (value, weight) pair or empty. *Value* here refers to an instance
/// of the value being animated: for example, [`Vec3`] in the case of
/// translation keyframes.  The stack stores intermediate values generated while
/// evaluating the [`crate::graph::AnimationGraph`], while the blend register
/// stores the result of a blend operation.
///
/// [`Vec3`]: bevy_math::Vec3
pub trait AnimationCurveEvaluator: Downcast + Send + Sync + 'static {
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

    /// Additively blends the top element of the stack with the blend register.
    ///
    /// The semantics of this method are as follows:
    ///
    /// 1. Pop the top element of the stack. Call its value vₘ and its weight
    ///    wₘ. If the stack was empty, return success.
    ///
    /// 2. If the blend register is empty, set the blend register value to vₘ
    ///    and the blend register weight to wₘ; then, return success.
    ///
    /// 3. If the blend register is nonempty, call its current value vₙ.
    ///    Then, set the value of the blend register to vₙ + vₘwₘ.
    ///
    /// 4. Return success.
    fn add(&mut self, graph_node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError>;

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

    /// Pops the top value off the stack and writes it into the appropriate
    /// component.
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
    fn commit(&mut self, entity: AnimationEntityMut) -> Result<(), AnimationEvaluationError>;
}

impl_downcast!(AnimationCurveEvaluator);

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
    fn sample_clamped(&self, t: f32) -> T {
        // `UnevenCore::sample_with` is implicitly clamped.
        self.core.sample_with(t, <T as Animatable>::interpolate)
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
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

fn inconsistent<P>() -> AnimationEvaluationError
where
    P: 'static + ?Sized,
{
    AnimationEvaluationError::InconsistentEvaluatorImplementation(TypeId::of::<P>())
}

/// Returns an [`AnimatedField`] with a given `$component` and `$field`.
///
/// This can be used in the following way:
///
/// ```
/// # use bevy_animation::{animation_curves::AnimatedField, animated_field};
/// # use bevy_color::Srgba;
/// # use bevy_ecs::component::Component;
/// # use bevy_math::Vec3;
/// # use bevy_reflect::Reflect;
/// #[derive(Component, Reflect)]
/// struct Transform {
///     translation: Vec3,
/// }
///
/// let field = animated_field!(Transform::translation);
///
/// #[derive(Component, Reflect)]
/// struct Color(Srgba);
///
/// let tuple_field = animated_field!(Color::0);
/// ```
#[macro_export]
macro_rules! animated_field {
    ($component:ident::$field:tt) => {
        AnimatedField::new_unchecked(stringify!($field), |component: &mut $component| {
            &mut component.$field
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animated_field_tuple_struct_simple_uses() {
        #[derive(Clone, Debug, Component, Reflect)]
        struct A(f32);
        let _ = AnimatedField::new_unchecked("0", |a: &mut A| &mut a.0);

        #[derive(Clone, Debug, Component, Reflect)]
        struct B(f32, f64, f32);
        let _ = AnimatedField::new_unchecked("0", |b: &mut B| &mut b.0);
        let _ = AnimatedField::new_unchecked("1", |b: &mut B| &mut b.1);
        let _ = AnimatedField::new_unchecked("2", |b: &mut B| &mut b.2);
    }
}
