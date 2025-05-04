//! Animation Transitioning logic goes here!
//! This struct should in the later run be responsible for handling multi-state Animation Graph nodes.

use crate::{
    graph::{AnimationGraphHandle, AnimationNodeIndex},
    ActiveAnimation, AnimationPlayer,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_time::Time;
use core::{f32, time::Duration};

/// Component responsible for managing transitions between multiple nodes or states.
///
/// It supports multiple independent "flows", where each flow represents a distinct active
/// animation or state machine. A flow tracks the transition between two states over time.
///
/// In the simplest case, `flow_amount` should be set to `1`, indicating a single flow.
/// However, if multiple state machines or simultaneous animations are needed, `flow_amount`
/// should be increased accordingly.
///
/// Is worth mentioning, that when using AnimationTransitions, you should avoid messing around with player!
/// As he will do the heavy lifting for you! All you need to worry about is transitioning your flows!
/// If you do play an additional animation directly via player  this WILL BREAK!
///
/// It is also the user's responsibility to track which flow they are currently operating on
/// when triggering transitions.
/// Ex: Flow 0 - Plays idle,walks and so on. Affect whole body
/// Flow 1 - Plays close hands - Affects only hand bones.
#[derive(Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Default)]
#[require(AnimationGraphHandle, AnimationPlayer)]
pub struct AnimationTransitions {
    #[deref]
    transitions: Vec<AnimationTransition>,
    /// Flows represent sequences of animation states.
    /// For example, in cases such as masked or additive animation scenarios, a user can easily define transitions between previous and new states.
    /// This concept is similar to "main" animations, but instead of one sole ActiveAnimation, we might have multiple active animation nodes being controlled!
    flows: Vec<Option<AnimationNodeIndex>>,
}

/// An animation node that is being faded out as part of a transition, note this controls the animations being played!
#[derive(Debug, Reflect, Clone)]
pub struct AnimationTransition {
    /// How much weight we will decrease according to the given user value
    duration: Duration,
    /// Node to transition from
    old_node: AnimationNodeIndex,
    /// Node to transition into
    new_node: AnimationNodeIndex,
    /// Acts similarly to a local variable, tracks how far into the transition are we, should start from 1. and go to 0
    weight: f32,
}
impl AnimationTransitions {
    /// Define your flow amount and initializes your component!
    pub fn new(flow_amount: usize) -> Self {
        Self {
            flows: vec![None; flow_amount],
            // Default transitions are instantaniously cleared
            transitions: Vec::new(),
        }
    }

    /// Transitions the specified flow from its current node to a new node over a given duration.
    ///
    /// This method manages transitions within a specific flow, allowing multiple independent
    /// state machines or animation layers to transition separately. If the flow has no previous node,
    /// it will treat the `new_node` as both the old and new node during the transition.
    ///
    /// The logic here goes as follows: Check if you have a history,
    /// if not old_node eq new_node.
    /// Queues a transition, make old_node eq new_node, after queuing transition
    /// Put in active animation new_node
    pub fn transition_flows<'p>(
        &mut self,
        player: &'p mut AnimationPlayer,
        new_node: AnimationNodeIndex,
        flow_position: usize,
        duration: Duration,
    ) -> &'p mut ActiveAnimation {
        // Check if flow exists
        if let Some(old_node) = self.flows.get_mut(flow_position) {
            // If is first time playing, just say old node equals new node
            let previous_node = old_node.unwrap_or(new_node);
            self.transitions.push(AnimationTransition {
                duration,
                old_node: previous_node,
                new_node,
                weight: 1.0,
            });
            *old_node = Some(new_node);

            // Starts new animation, note we wont clear AnimationPlayer active animation hashmap here!
            player.start(new_node)
        } else {
            panic!("Flow position {flow_position} is out of bounds!");
        }
    }
}

/// System responsible for handling [`AnimationTransitions`] transitioning nodes among each other. According to the pacing defined by user.
pub fn handle_node_transition(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
    time: Res<Time>,
) {
    for (mut animation_transitions, mut animation_player) in query.iter_mut() {
        let mut remaining_weight = 1.0;
        for transition in animation_transitions.iter_mut() {
            // How much to transition per tick!
            transition.weight = (transition.weight
                - 1. / transition.duration.as_secs_f32() * time.delta_secs())
            .max(0.0);

            // Handles edge case when duration is zero
            if transition.duration == Duration::ZERO {
                transition.weight = 0.0;
            }

            // Handles edge case first flow
            if transition.old_node.eq(&transition.new_node) {
                if let Some(old_node) = animation_player.animation_mut(transition.old_node) {
                    remaining_weight -= transition.weight * remaining_weight;
                    old_node.weight = remaining_weight;
                }
                continue;
            }

            if let Some(old_node) = animation_player.animation_mut(transition.old_node) {
                old_node.weight = transition.weight * remaining_weight;
                remaining_weight -= old_node.weight;
            }

            if let Some(new_node) = animation_player.animation_mut(transition.new_node) {
                new_node.weight = remaining_weight;
            }
        }
    }
}

/// A system that removes transitions that have completed from the
/// [`AnimationTransitions`] object.
pub fn expire_completed_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
) {
    for (mut animation_transitions, mut animation_player) in query.iter_mut() {
        animation_transitions.retain(|transition| {
            let should_expire = transition.weight <= 0.0;
            let first_init = transition.new_node.eq(&transition.old_node);
            //We need to handle the edge case of first initialization, we shouldnt stop in that case!
            if should_expire && !first_init {
                animation_player.stop(transition.old_node);
                false
            } else if should_expire && first_init {
                false
            } else {
                true
            }
        });
    }
}
