//! Animation transitions.
//!
//! Please note that this is an unstable temporary API. It may be replaced by a
//! state machine in the future.

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
/// [`AnimationPlayer`] and [`bevy_asset::Handle<AnimationGraph>`]. It'll take
/// responsibility for adjusting the weight on the [`ActiveAnimation`] in order
/// to fade out animations smoothly.
///
/// When using an [`AnimationTransitions`] component, you should play all
/// animations through the [`AnimationTransitions::play`] method, rather than by
/// directly manipulating the [`AnimationPlayer`]. Playing animations through
/// the [`AnimationPlayer`] directly will cause the [`AnimationTransitions`]
/// component to get confused about which animation is the "main" animation, and
/// transitions will usually be incorrect as a result.
#[derive(Component, Default, Reflect)]
pub struct AnimationTransitions {
    main_animation: Option<AnimationNodeIndex>,
    transitions: Vec<AnimationTransition>,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for AnimationTransitions {
    fn clone(&self) -> Self {
        Self {
            main_animation: self.main_animation,
            transitions: self.transitions.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.main_animation = source.main_animation;
        self.transitions.clone_from(&source.transitions);
    }
}

/// An animation that is being faded out as part of a transition
#[derive(Debug, Clone, Copy, Reflect)]
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
    /// `transition_duration`.
    ///
    /// Pass [`Duration::ZERO`] to instantly switch to a new animation, avoiding
    /// any transition.
    pub fn play<'p>(
        &mut self,
        player: &'p mut AnimationPlayer,
        new_animation: AnimationNodeIndex,
        transition_duration: Duration,
    ) -> &'p mut ActiveAnimation {
        if let Some(old_animation_index) = self.main_animation.replace(new_animation) {
            if let Some(old_animation) = player.animation_mut(old_animation_index) {
                if !old_animation.is_paused() {
                    self.transitions.push(AnimationTransition {
                        current_weight: old_animation.weight,
                        weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(),
                        animation: old_animation_index,
                    });
                }
            }
        }

        self.main_animation = Some(new_animation);
        player.start(new_animation)
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
    let mut remaining_weight = 1.0;
    for (mut animation_transitions, mut player) in query.iter_mut() {
        for transition in &mut animation_transitions.transitions.iter_mut().rev() {
            // Decrease weight.
            transition.current_weight = (transition.current_weight
                - transition.weight_decline_per_sec * time.delta_seconds())
            .max(0.0);

            // Update weight.
            let Some(ref mut animation) = player.animation_mut(transition.animation) else {
                continue;
            };
            animation.weight = transition.current_weight * remaining_weight;
            remaining_weight -= animation.weight;
        }

        if let Some(main_animation_index) = animation_transitions.main_animation {
            if let Some(ref mut animation) = player.animation_mut(main_animation_index) {
                animation.weight = remaining_weight;
            }
        }
    }
}

/// A system that removed transitions that have completed from the
/// [`AnimationTransitions`] object.
pub fn expire_completed_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
) {
    for (mut animation_transitions, mut player) in query.iter_mut() {
        animation_transitions.transitions.retain(|transition| {
            let expire = transition.current_weight <= 0.0;
            if expire {
                player.stop(transition.animation);
            }
            !expire
        });
    }
}
