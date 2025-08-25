use crate::{
    animatable::Animatable,
    animation_curves::{AnimationCurve, AnimationCurveEvaluator, EvaluatorId},
    graph::AnimationNodeIndex,
    AnimationEntityMut, AnimationEvaluationError,
};
use bevy_math::curve::{iterable::IterableCurve, Interval};
use bevy_mesh::morph::MorphWeights;
use bevy_reflect::{FromReflect, Reflect, Reflectable};
use core::{any::TypeId, fmt::Debug};

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
