//! Animation transitions.
//!
//! Please note that this is an unstable temporary API. It may be replaced by a
//! state machine in the future.

use bevy_asset::Assets;
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_time::Time;
use core::time::Duration;

use crate::{
    graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex, AnimationNodeType},
    ActiveAnimation, AnimationPlayer,
};

/// Manages fade-out of animation blend factors, allowing for smooth transitions
/// between animations.
///
/// To use this component, place it on the same entity as the
/// [`AnimationPlayer`] and [`AnimationGraphHandle`]. It'll take
/// responsibility for adjusting the weight on the [`ActiveAnimation`] in order
/// to fade out animations smoothly.
///
/// When using an [`AnimationTransitions`] component, you should play all
/// animations through the [`AnimationTransitions::play`] method, rather than by
/// directly manipulating the [`AnimationPlayer`]. Playing animations through
/// the [`AnimationPlayer`] directly will cause the [`AnimationTransitions`]
/// component to get confused about which animation is the "main" animation, and
/// transitions will usually be incorrect as a result.
///
/// Transition away from an animation node has different behaviour depending on
/// the node it's transitioning from:
/// - Clip: once a transition away from a clip completes, the clip is stopped.
/// - Blend/Add: nodes are kept at weight 0 instead of being stopped, so playing
///   one directly through the [`AnimationPlayer`] leaves it silent until
///   [`ActiveAnimation::set_weight`] is called.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default, Clone)]
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
#[reflect(Clone)]
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
        if let Some(old_animation_index) = self.main_animation.replace(new_animation)
            && let Some(old_animation) = player.animation_mut(old_animation_index)
            && !old_animation.is_paused()
        {
            self.transitions.push(AnimationTransition {
                current_weight: old_animation.weight,
                weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(),
                animation: old_animation_index,
            });
        }

        // If already transitioning away from this animation, cancel the transition.
        // Otherwise the transition ending would incorrectly stop the new animation.
        self.transitions
            .retain(|transition| transition.animation != new_animation);

        player.start(new_animation)
    }

    /// Obtain the currently playing main animation.
    pub fn get_main_animation(&self) -> Option<AnimationNodeIndex> {
        self.main_animation
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

        for transition in &mut animation_transitions.transitions.iter_mut().rev() {
            // Decrease weight.
            transition.current_weight = (transition.current_weight
                - transition.weight_decline_per_sec * time.delta_secs())
            .max(0.0);

            // Update weight.
            let Some(ref mut animation) = player.animation_mut(transition.animation) else {
                continue;
            };
            animation.weight = transition.current_weight * remaining_weight;
            remaining_weight -= animation.weight;
        }

        if let Some(main_animation_index) = animation_transitions.main_animation
            && let Some(ref mut animation) = player.animation_mut(main_animation_index)
        {
            animation.weight = remaining_weight;
        }
    }
}

/// A system that removed transitions that have completed from the
/// [`AnimationTransitions`] object.
///
/// Expired clips are stopped, but expired Blend/Add nodes are kept at
/// weight 0 instead, since a node the [`AnimationPlayer`] has no entry for
/// weighs in at its full graph weight.
pub fn expire_completed_transitions(
    graphs: Res<Assets<AnimationGraph>>,
    mut query: Query<(
        &mut AnimationTransitions,
        &mut AnimationPlayer,
        Option<&AnimationGraphHandle>,
    )>,
) {
    for (mut animation_transitions, mut player, graph) in query.iter_mut() {
        let graph = graph.and_then(|graph| graphs.get(graph));
        animation_transitions.transitions.retain(|transition| {
            if transition.current_weight > 0.0 {
                return true;
            }
            let node_type = graph
                .and_then(|graph| graph.get(transition.animation))
                .map(|node| &node.node_type);
            // A stopped blend or additive node would weigh in fully again, so it stays
            // silent instead.
            if let Some(AnimationNodeType::Blend | AnimationNodeType::Add) = node_type {
                if let Some(animation) = player.animation_mut(transition.animation) {
                    animation.set_weight(0.0);
                }
            } else {
                player.stop(transition.animation);
            }
            false
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_asset::Handle;
    use bevy_ecs::{system::RunSystemOnce, world::World};

    #[test]
    fn expiring_keeps_blend_nodes_silent_and_stops_clips() {
        let mut graph = AnimationGraph::new();
        let blend = graph.add_blend(1.0, graph.root);
        let clip = graph.add_clip(Handle::default(), 1.0, graph.root);
        let mut graphs = Assets::<AnimationGraph>::default();
        let graph = graphs.add(graph);

        let mut player = AnimationPlayer::default();
        player.play(blend);
        player.play(clip);
        let expired = |animation| AnimationTransition {
            current_weight: 0.0,
            weight_decline_per_sec: 1.0,
            animation,
        };
        let transitions = AnimationTransitions {
            main_animation: None,
            transitions: vec![expired(blend), expired(clip)],
        };

        let mut world = World::new();
        world.insert_resource(graphs);
        let entity = world
            .spawn((player, transitions, AnimationGraphHandle(graph)))
            .id();
        world.run_system_once(expire_completed_transitions).unwrap();

        let player = world.get::<AnimationPlayer>(entity).unwrap();
        assert_eq!(
            player.animation(blend).map(ActiveAnimation::weight),
            Some(0.0)
        );
        assert!(player.animation(clip).is_none());
    }
}
