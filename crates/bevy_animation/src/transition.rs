use bevy_asset::{Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_time::Time;
use core::time::Duration;

use crate::graph::{AnimationGraph, AnimationNodeIndex};

/// Component responsible for managing transitions between N given nodes or states.
/// It can handle multiple transitions and supports scalable flow-based animation management.
#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Default)]
pub struct AnimationTransitions {
    #[deref]
    transitions: Vec<AnimationTransition>,
    /// Flows represent sequences of animation states.
    /// For example, in cases such as masked or additive animation scenarios, a user can easily define transitions between previous and new states.
    /// This concept is similar to "main" animations but is designed to scale across multiple animation layers or parts.
    flows: Vec<Option<AnimationNodeIndex>>,
}

/// An animation that is being faded out as part of a transition
#[derive(Debug, Reflect, Default, Clone)]
pub struct AnimationTransition {
    /// How much weight we will decrease according to the given user [`Duration`]
    weight_decline_per_sec: f32,
    /// Node to transition from
    old_node: AnimationNodeIndex,
    /// Node to transition into
    new_node: AnimationNodeIndex,
    /// Handle pointer to required component [`AnimationGraph`], needed to grab nodes current weights
    graph: Handle<AnimationGraph>,
    /// Acts similarly to a local variable, tracks how far into the transition are we, starts from 1. should go to 0
    weight: f32,
}
impl AnimationTransitions {
    /// Potato
    pub fn new(flow_amount: u8) -> Self {
        Self {
            flows: vec![None; flow_amount as usize],
            transitions: vec![AnimationTransition::default(); flow_amount as usize],
        }
    }

    /// Transition between one graph node to another according to the given duration
    pub fn transition_nodes(
        &mut self,
        graph: Handle<AnimationGraph>,
        old_node: AnimationNodeIndex,
        new_node: AnimationNodeIndex,
        transition: Duration,
    ) {
        self.push(AnimationTransition {
            weight_decline_per_sec: 1.0 / transition.as_secs_f32(),
            old_node,
            new_node,
            graph,
            weight: 1.0,
        });
    }

    pub fn transition_flows(
        &mut self,
        player: &mut AnimationGraph,
        new_animation: AnimationNodeIndex,
        transition_duration: Duration,
    ) {

        // if let Some(old_animation_index) = self.main_animation.replace(new_animation) {
        //     if let Some(old_animation) = player.animation_mut(old_animation_index) {
        //         if !old_animation.is_paused() {
        //             self.transitions.push(AnimationTransition {
        //                 current_weight: old_animation.weight,
        //                 weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(),
        //                 animation: old_animation_index,
        //             });
        //         }
        //     }
        // }

        // // If already transitioning away from this animation, cancel the transition.
        // // Otherwise the transition ending would incorrectly stop the new animation.
        // self.transitions
        //     .retain(|transition| transition.animation != new_animation);

        // player.start(new_animation)
    }
}

/// System responsible for handling [`AnimationTransitions`] transitioning nodes among each other. According to the pacing defined by user.
pub fn handle_node_transition(
    mut query: Query<&mut AnimationTransitions>,
    mut assets_graph: ResMut<Assets<AnimationGraph>>,
    time: Res<Time>,
) {
    for mut animation_transitions in query.iter_mut() {
        for transition in animation_transitions.iter_mut() {
            let animation_graph = assets_graph.get_mut(&transition.graph).unwrap();

            if let Some(old_node) = animation_graph.get_mut(transition.old_node) {
                if transition.weight.eq(&1.0) {
                    old_node.weight = transition.weight;
                }
                old_node.weight -= transition.weight_decline_per_sec * time.delta_secs().max(0.0);
            }
            if let Some(new_node) = animation_graph.get_mut(transition.new_node) {
                if transition.weight.eq(&1.0) {
                    new_node.weight = 0.0;
                }
                new_node.weight += transition.weight_decline_per_sec * time.delta_secs().min(1.0);
            }

            transition.weight -= transition.weight_decline_per_sec * time.delta_secs().max(0.0);
        }
    }
}

/// A system that removes transitions that have completed from the
/// [`AnimationTransitions`] object.
pub fn expire_completed_transitions(mut query: Query<&mut AnimationTransitions>) {
    for mut animation_transitions in query.iter_mut() {
        animation_transitions.retain(|transition| transition.weight <= 0.0);
    }
}
