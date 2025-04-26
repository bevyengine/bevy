//! Animation transitions.
//!
//! Please note that this is an unstable temporary API. It may be replaced by a
//! state machine in the future.

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_time::Time;
use core::time::Duration;

use crate::{graph::{AnimationGraph, AnimationGraphNode, AnimationNodeIndex},  AnimationPlayer};

/// Manages fade-out of animation blend factors, allowing for smooth transitions
/// between animations.
///
/// To use this component, place it on the same entity as the
/// [`AnimationPlayer`] and [`AnimationGraphHandle`](crate::AnimationGraphHandle). It'll take
/// responsibility for adjusting the weight on the [`ActiveAnimation`] in order
/// to fade out animations smoothly.
#[derive(Component, Default, Reflect,Deref,DerefMut,Clone)]
#[reflect(Component, Default, Clone)]
pub struct AnimationTransitions (Vec<AnimationTransition>);


/// An animation that is being faded out as part of a transition
#[derive(Debug, Clone,  Reflect)]
#[reflect(Clone)]
pub struct AnimationTransition {
    /// The current weight. Starts at 1.0 and goes to 0.0 during the fade-out.
    current_weight: f32,
    /// How much to decrease `current_weight` per second
    weight_decline_per_sec: f32,
    /// The animation that is beind fade out
    old_node:AnimationGraphNode,
    /// The animation that is gaining weight
    new_node: AnimationGraphNode,
}

impl AnimationTransitions {
    /// Plays a new animation on the given [`AnimationPlayer`], fading out any
    /// existing animations that were already playing over the
    /// `transition_duration`.
    ///
    /// Pass [`Duration::ZERO`] to instantly switch to a new animation, avoiding
    /// any transition.
    pub fn transition(
        &mut self,
        graph: & mut AnimationGraph,
        old_animation: AnimationNodeIndex,
        new_animation: AnimationNodeIndex,
        transition_duration: Duration,
    ) {
        let old_node = graph.get(old_animation).unwrap().clone();
        let new_node = graph.get(new_animation).unwrap().clone();

        self.push(AnimationTransition { current_weight: old_node.weight, weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(), old_node,new_node });


    }
}

/// A system that alters the weight of currently-playing transitions based on
/// the current time and decline amount.
pub fn advance_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
    time: Res<Time>,
) {
    // We use a "greedy layer" system here. The top layer (most recent
    // transition) gets as much as weight as it wants, and the remaining amount
    // is divided between all the other layers, eventually culminating in the
    // currently-playing animation receiving whatever's left. This results in a
    // nicely normalized weight.
    for (mut animation_transitions, mut player) in query.iter_mut() {
        let mut remaining_weight = 1.0;

        for transition in &mut animation_transitions.iter_mut().rev() {
            // Decrease weight.
            transition.current_weight = (transition.current_weight
                - transition.weight_decline_per_sec * time.delta_secs())
            .max(0.0);

            transition.old_node.weight = transition.current_weight * remaining_weight;
            remaining_weight -= transition.old_node.weight;
        }
    }
}

/// A system that removed transitions that have completed from the
/// [`AnimationTransitions`] object.
pub fn expire_completed_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
) {
    for (mut animation_transitions, _player) in query.iter_mut() {
        animation_transitions.retain(|transition| {
            transition.current_weight > 0.0
        });
    }
}
