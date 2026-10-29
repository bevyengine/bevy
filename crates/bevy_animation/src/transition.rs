//! Crossfading between animations with [`AnimationTransitions`].
//!
//! In the future, this is intended to serve as the building block for
//! animation state machines.

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
use core::{f32, num::NonZeroUsize, time::Duration};

/// Manages crossfades between animation nodes of an [`AnimationPlayer`].
///
/// Transitions are organized into independent *flows*. Each flow has at most
/// one current node, and transitioning a flow crossfades from its current node
/// to a new one, without affecting the nodes of any other flow. This allows,
/// for example, transitioning between locomotion animations in one flow while
/// another flow transitions between hand poses:
///
/// - Flow 0: idle, walk, run, etc. Affects the whole body.
/// - Flow 1: open hand, closed hand, etc. Affects only the hand bones,
///   through an [animation mask].
///
/// Flows don't know which bones they affect: masking and additive blending are
/// configured on the [`AnimationGraph`] itself. Keeping track of which flow
/// controls which nodes is up to the user.
///
/// [`AnimationTransitions::new`] creates a single flow, which is enough when
/// only one animation needs to play at a time. Use
/// [`AnimationTransitions::new_with_flow`] for more.
///
/// When using [`AnimationTransitions`], start animations through it rather than
/// directly through the [`AnimationPlayer`]. The [`AnimationTransitions`]
/// component manages the weights of all the animations and will get
/// confused if the animations are started directly via the [`AnimationPlayer`].
///
/// [animation mask]: crate::graph::AnimationMask
/// [`AnimationGraph`]: crate::graph::AnimationGraph
#[derive(Component, Reflect, Deref, DerefMut)]
#[reflect(Component, Default)]
#[require(AnimationGraphHandle, AnimationPlayer)]
pub struct AnimationTransitions {
    #[deref]
    transitions: Vec<AnimationTransition>,
    /// The current node of each flow, or `None` if nothing was played on that
    /// flow yet.
    flows: Vec<Option<AnimationNodeIndex>>,
}

/// An animation node that is being faded out as part of a transition within a
/// flow.
#[derive(Debug, Reflect, Clone)]
pub struct AnimationTransition {
    /// The current weight. Starts at the weight the node had when the
    /// transition started, and goes to 0 during the fade-out.
    current_weight: f32,
    /// How much to decrease `current_weight` per second.
    weight_decline: f32,
    /// The node being faded out.
    node: AnimationNodeIndex,
    /// The flow this transition belongs to.
    flow: usize,
}

impl Default for AnimationTransitions {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationTransitions {
    /// Creates an [`AnimationTransitions`] component with a single flow.
    pub fn new() -> Self {
        Self::new_with_flow(NonZeroUsize::MIN)
    }

    /// Creates an [`AnimationTransitions`] component with `flow_amount` flows,
    /// i.e. the number of animations that can be transitioned independently.
    pub fn new_with_flow(flow_amount: impl Into<NonZeroUsize>) -> Self {
        Self {
            transitions: Vec::new(),
            flows: vec![None; flow_amount.into().get()],
        }
    }

    /// Returns the current node of the given flow, or `None` if nothing was
    /// played on it yet or if `flow_position` is out of bounds.
    pub fn current_node(&self, flow_position: usize) -> Option<AnimationNodeIndex> {
        self.flows.get(flow_position).cloned().flatten()
    }

    /// Crossfades flow `0` from its current node to `new_node` over `duration`.
    ///
    /// See [`AnimationTransitions::transition_flows`] if you want to transition a specific flow.
    pub fn transition<'p>(
        &mut self,
        player: &'p mut AnimationPlayer,
        new_node: AnimationNodeIndex,
        duration: Duration,
    ) -> &'p mut ActiveAnimation {
        self.transition_flows_unchecked(player, new_node, 0, duration)
    }

    /// Crossfades the given flow from its current node to `new_node` over
    /// `duration`, and starts playing `new_node` on the [`AnimationPlayer`].
    ///
    /// Other flows are not affected. If nothing was played on this flow yet,
    /// `new_node` starts playing at full weight.
    ///
    /// Returns `None` and does nothing if `flow_position` is out of bounds.
    /// See [`AnimationTransitions::transition_flows_unchecked`] for a variant
    /// that panics instead.
    pub fn transition_flows<'p>(
        &mut self,
        player: &'p mut AnimationPlayer,
        new_node: AnimationNodeIndex,
        flow_position: usize,
        duration: Duration,
    ) -> Option<&'p mut ActiveAnimation> {
        let current_node = self.flows.get_mut(flow_position)?;

        // Fade out the previous node of this flow, starting from its current weight
        if let Some(old_node) = current_node.replace(new_node)
            && old_node != new_node
            && let Some(old_animation) = player.animation_mut(old_node)
            && !old_animation.is_paused()
        {
            self.transitions.push(AnimationTransition {
                current_weight: old_animation.weight,
                weight_decline: duration.as_secs_f32().recip(),
                node: old_node,
                flow: flow_position,
            });
        }

        // If already transitioning away from this node, cancel the transition.
        // Otherwise the transition ending would incorrectly stop the new node.
        self.transitions
            .retain(|transition| transition.node != new_node);

        Some(player.start(new_node))
    }

    /// Like [`AnimationTransitions::transition_flows`], but panics instead of
    /// returning `None` if `flow_position` is out of bounds.
    ///
    /// # Panics
    ///
    /// Panics if `flow_position` is out of bounds.
    pub fn transition_flows_unchecked<'p>(
        &mut self,
        player: &'p mut AnimationPlayer,
        new_node: AnimationNodeIndex,
        flow_position: usize,
        duration: Duration,
    ) -> &'p mut ActiveAnimation {
        let flow_amount = self.flows.len();
        self.transition_flows(player, new_node, flow_position, duration)
            .unwrap_or_else(|| {
                panic!("Flow position {flow_position} is out of bounds (flow amount is {flow_amount})!")
            })
    }
}

/// A system that advances the crossfades of [`AnimationTransitions`] and
/// updates the weights of the animations they control.
pub fn handle_node_transition(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
    time: Res<Time>,
) {
    // Within each flow, we use a "greedy layer" system. The most recent
    // transition gets as much weight as it wants, and the remaining amount is
    // divided between the older ones, eventually culminating in the current
    // node of the flow receiving whatever's left. Each flow distributes its
    // own weight, so transitions in one flow don't affect another.
    for (mut animation_transitions, mut player) in query.iter_mut() {
        let animation_transitions = &mut *animation_transitions;
        for (flow, current_node) in animation_transitions.flows.iter().enumerate() {
            let mut remaining_weight = 1.0;

            for transition in animation_transitions
                .transitions
                .iter_mut()
                .rev()
                .filter(|transition| transition.flow == flow)
            {
                transition.current_weight = (transition.current_weight
                    - transition.weight_decline * time.delta_secs())
                .max(0.0);

                let Some(animation) = player.animation_mut(transition.node) else {
                    continue;
                };
                animation.weight = transition.current_weight * remaining_weight;
                remaining_weight -= animation.weight;
            }

            if let Some(current_node) = current_node
                && let Some(animation) = player.animation_mut(*current_node)
            {
                animation.weight = remaining_weight;
            }
        }
    }
}

/// A system that removes transitions that have completed from
/// [`AnimationTransitions`], and stops the nodes they faded out.
pub fn expire_completed_transitions(
    mut query: Query<(&mut AnimationTransitions, &mut AnimationPlayer)>,
) {
    for (mut animation_transitions, mut player) in query.iter_mut() {
        animation_transitions.retain(|transition| {
            let expire = transition.current_weight <= 0.0;
            if expire {
                player.stop(transition.node);
            }
            !expire
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{system::RunSystemOnce, world::World};

    fn setup(flow_amount: usize) -> World {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.spawn(AnimationTransitions::new_with_flow(
            NonZeroUsize::new(flow_amount).unwrap(),
        ));
        world
    }

    fn transition(world: &mut World, node: AnimationNodeIndex, flow: usize, duration: Duration) {
        let (mut transitions, mut player) = world
            .query::<(&mut AnimationTransitions, &mut AnimationPlayer)>()
            .single_mut(world)
            .unwrap();
        transitions.transition_flows_unchecked(&mut player, node, flow, duration);
    }

    fn tick(world: &mut World, delta: Duration) {
        world.resource_mut::<Time>().advance_by(delta);
        world.run_system_once(handle_node_transition).unwrap();
        world.run_system_once(expire_completed_transitions).unwrap();
    }

    #[track_caller]
    fn assert_weight(world: &mut World, node: AnimationNodeIndex, expected: Option<f32>) {
        let player = world.query::<&AnimationPlayer>().single(world).unwrap();
        let weight = player.animation(node).map(ActiveAnimation::weight);
        match (weight, expected) {
            (Some(weight), Some(expected)) => assert!(
                (weight - expected).abs() < 1e-5,
                "node {node:?}: expected weight {expected}, got {weight}"
            ),
            _ => assert_eq!(weight, expected, "node {node:?}"),
        }
    }

    #[test]
    fn transitioning_one_flow_does_not_affect_another() {
        let [a, b, c] = [1, 2, 3].map(AnimationNodeIndex::new);
        let mut world = setup(2);
        transition(&mut world, a, 0, Duration::ZERO);
        transition(&mut world, b, 1, Duration::ZERO);
        tick(&mut world, Duration::from_millis(100));
        assert_weight(&mut world, a, Some(1.0));
        assert_weight(&mut world, b, Some(1.0));

        // Crossfade flow 0 from a to c, while flow 1 keeps playing b.
        transition(&mut world, c, 0, Duration::from_secs(1));
        tick(&mut world, Duration::from_millis(250));
        assert_weight(&mut world, a, Some(0.75));
        assert_weight(&mut world, c, Some(0.25));
        assert_weight(&mut world, b, Some(1.0));

        tick(&mut world, Duration::from_secs(1));
        assert_weight(&mut world, a, None);
        assert_weight(&mut world, c, Some(1.0));
        assert_weight(&mut world, b, Some(1.0));
    }

    #[test]
    fn flows_transition_simultaneously_and_independently() {
        let [a, b, c, d] = [1, 2, 3, 4].map(AnimationNodeIndex::new);
        let mut world = setup(2);
        transition(&mut world, a, 0, Duration::ZERO);
        transition(&mut world, b, 1, Duration::ZERO);
        tick(&mut world, Duration::from_millis(100));

        // Crossfade both flows at the same time: a -> c and b -> d.
        transition(&mut world, c, 0, Duration::from_secs(1));
        transition(&mut world, d, 1, Duration::from_secs(1));
        tick(&mut world, Duration::from_millis(250));
        assert_weight(&mut world, a, Some(0.75));
        assert_weight(&mut world, c, Some(0.25));
        assert_weight(&mut world, b, Some(0.75));
        assert_weight(&mut world, d, Some(0.25));

        tick(&mut world, Duration::from_secs(1));
        assert_weight(&mut world, a, None);
        assert_weight(&mut world, b, None);
        assert_weight(&mut world, c, Some(1.0));
        assert_weight(&mut world, d, Some(1.0));
    }

    #[test]
    fn transitioning_out_of_bounds_flow_does_nothing() {
        let a = AnimationNodeIndex::new(1);
        let mut transitions = AnimationTransitions::new();
        let mut player = AnimationPlayer::default();

        assert!(transitions
            .transition_flows(&mut player, a, 1, Duration::ZERO)
            .is_none());
        assert!(transitions.is_empty());
        assert!(!player.is_playing_animation(a));
    }

    #[test]
    #[should_panic(expected = "Flow position 1 is out of bounds")]
    fn transitioning_out_of_bounds_flow_unchecked_panics() {
        let a = AnimationNodeIndex::new(1);
        let mut transitions = AnimationTransitions::new();
        let mut player = AnimationPlayer::default();

        transitions.transition_flows_unchecked(&mut player, a, 1, Duration::ZERO);
    }

    #[test]
    fn transitioning_back_to_a_fading_node_keeps_it_playing() {
        let [a, c] = [1, 3].map(AnimationNodeIndex::new);
        let mut world = setup(1);
        transition(&mut world, a, 0, Duration::ZERO);
        tick(&mut world, Duration::from_millis(100));

        // Start fading a out, then go back to it before the fade completes.
        transition(&mut world, c, 0, Duration::from_secs(1));
        tick(&mut world, Duration::from_millis(500));
        transition(&mut world, a, 0, Duration::from_secs(1));
        tick(&mut world, Duration::from_millis(250));
        tick(&mut world, Duration::from_secs(1));

        assert_weight(&mut world, a, Some(1.0));
        assert_weight(&mut world, c, None);
    }
}
