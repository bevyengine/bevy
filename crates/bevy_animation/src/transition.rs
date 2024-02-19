//! Animation transitions.
//!
//! Please note that this is an unstable temporary API. It may be replaced by a
//! state machine in the future.

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    system::{Query, Res},
};
use bevy_reflect::Reflect;
use bevy_time::Time;
use bevy_utils::{Duration, HashMap};

use crate::{graph::AnimationNodeIndex, AnimationPlayer};

#[derive(Component, Deref, DerefMut, Reflect)]
pub struct AnimationTransitions(pub HashMap<AnimationNodeIndex, AnimationTransition>);

/// An animation that is being faded out as part of a transition
#[derive(Reflect)]
pub struct AnimationTransition {
    /// The current weight. Starts at 1.0 and goes to 0.0 during the fade-out.
    current_weight: f32,
    /// How much to decrease `current_weight` per second
    weight_decline_per_sec: f32,
    /// The animation that is being faded out
    animation: AnimationNodeIndex,
}

impl AnimationTransitions {
    pub fn start_with_transition(
        &mut self,
        player: &mut AnimationPlayer,
        animation: AnimationNodeIndex,
        transition_duration: Duration,
    ) -> &mut Self {
        player.play(animation);

        self.insert(
            animation,
            AnimationTransition {
                current_weight: 1.0,
                weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(),
                animation,
            },
        );
        self
    }
}

// Advances transitions.
pub fn advance_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
    time: Res<Time>,
) {
    for (mut transitions, player) in query.iter_mut() {
        for transition in transitions.values_mut() {
            // Decrease weight.
            transition.current_weight =
                (transition.weight_decline_per_sec * time.delta_seconds()).max(0.0);
        }
    }
}

// Expires completed transitions.
pub fn expire_completed_transitions(mut query: Query<&mut AnimationTransitions>, time: Res<Time>) {
    for mut transitions in query.iter_mut() {
        transitions
            .0
            .retain(|_, transition| transition.current_weight <= 0.0);
    }
}
