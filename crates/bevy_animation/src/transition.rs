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
use bevy_utils::Duration;

use crate::{graph::AnimationNodeIndex, ActiveAnimation, AnimationPlayer};

/// Manages fade-out of animation blend factors, allowing for smooth transitions
/// between animations.
///
/// To use this component, place it on the same entity as the
/// [`AnimationPlayer`] and [`Handle<AnimationGraph>`]. It'll take
/// responsibility for adjusting the weight on the [`ActiveAnimation`] in order
/// to fade out animations smoothly.
#[derive(Component, Default, Deref, DerefMut, Reflect)]
pub struct AnimationTransitions(pub Vec<AnimationTransition>);

/// An animation that is being faded out as part of a transition
#[derive(Debug, Reflect)]
pub struct AnimationTransition {
    /// The current weight. Starts at 1.0 and goes to 0.0 during the fade-out.
    current_weight: f32,
    /// How much to decrease `current_weight` per second
    weight_decline_per_sec: f32,
    /// The animation that is being faded out
    animation: AnimationNodeIndex,
}

impl AnimationTransitions {
    /// Creates a new [`AnimationTransitions`] component, ready to be added to
    /// an entity with an [`AnimationPlayer`].
    pub fn new() -> AnimationTransitions {
        AnimationTransitions::default()
    }

    /// Plays a new animation on the given [`AnimationPlayer`], fading out any
    /// existing animations that were already playing over the
    /// [`transition_duration`].
    pub fn play<'p>(
        &mut self,
        player: &'p mut AnimationPlayer,
        animation: AnimationNodeIndex,
        transition_duration: Duration,
    ) -> &'p mut ActiveAnimation {
        for (&old_animation_index, old_animation) in player.playing_animations() {
            if !old_animation.is_paused() {
                self.push(AnimationTransition {
                    current_weight: old_animation.weight,
                    weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(),
                    animation: old_animation_index,
                });
            }
        }

        player.start(animation)
    }
}

/// A system that alters the weight of currently-playing transitions based on
/// the current time and decline amount.
pub fn advance_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
    time: Res<Time>,
) {
    for (mut transitions, mut player) in query.iter_mut() {
        for transition in &mut transitions.0 {
            // Decrease weight.
            transition.current_weight = (transition.current_weight
                - transition.weight_decline_per_sec * time.delta_seconds())
            .max(0.0);
            if let Some(ref mut animation) = player.animation_mut(transition.animation) {
                animation.weight = transition.current_weight;
            }
        }
    }
}

/// A system that removed transitions that have completed from the
/// [`AnimationTransitions`] object.
pub fn expire_completed_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
) {
    for (mut transitions, mut player) in query.iter_mut() {
        transitions.0.retain(|transition| {
            let expire = transition.current_weight <= 0.0;
            if expire {
                player.stop(transition.animation);
            }
            !expire
        });
    }
}
