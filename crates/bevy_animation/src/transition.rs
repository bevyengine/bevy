//! The transition manager.

use bevy_ecs::{
    component::Component,
    system::{Query, Res},
};
use bevy_math::FloatExt;
use bevy_reflect::Reflect;
use bevy_time::Time;
use bevy_utils::Duration;

use crate::{
    graph::{AnimationNodeIndex, AnimationWeight},
    AnimationGraph,
};

#[derive(Component, Default, Reflect)]
pub struct AnimationTransitions {
    pub transitions: Vec<AnimationTransition>,
}

#[derive(Clone, Reflect)]
pub struct AnimationTransition {
    node_index: AnimationNodeIndex,
    from_weight: AnimationWeight,
    to_weight: AnimationWeight,
    start_time: Duration,
    duration: Duration,
}

impl AnimationTransitions {
    pub fn new() -> AnimationTransitions {
        AnimationTransitions::default()
    }

    pub fn add<WFrom, WTo>(
        &mut self,
        node_index: AnimationNodeIndex,
        from_weight: WFrom,
        to_weight: WTo,
        start_time: Duration,
        duration: Duration,
    ) where
        WFrom: Into<AnimationWeight>,
        WTo: Into<AnimationWeight>,
    {
        self.transitions.push(AnimationTransition {
            node_index,
            from_weight: from_weight.into(),
            to_weight: to_weight.into(),
            start_time,
            duration,
        })
    }

    pub fn transition<WFrom, WTo>(
        &mut self,
        time: &Time,
        node_index: AnimationNodeIndex,
        from_weight: WFrom,
        to_weight: WTo,
        duration: Duration,
    ) where
        WFrom: Into<AnimationWeight>,
        WTo: Into<AnimationWeight>,
    {
        self.add(node_index, from_weight, to_weight, time.elapsed(), duration)
    }

    pub fn transition_from_current<WTo>(
        &mut self,
        time: &Time,
        animation_graph: &AnimationGraph,
        node_index: AnimationNodeIndex,
        to_weight: WTo,
        duration: Duration,
    ) where
        WTo: Into<AnimationWeight>,
    {
        self.transition(
            time,
            node_index,
            animation_graph[node_index].weight,
            to_weight,
            duration,
        )
    }
}

impl AnimationTransition {
    // Returns true if the transition needs to continue or false if it should be stopped.
    fn update(&mut self, time: &Time, animation_graph: &mut AnimationGraph) -> bool {
        let Some(mut node) = animation_graph.get_node_mut(self.node_index) else {
            return false;
        };
        let t = ((time.elapsed() - self.start_time).as_secs_f32() / self.duration.as_secs_f32())
            .clamp(0.0, 1.0);
        node.set_weight(self.from_weight.lerp(*self.to_weight, t));
        t < 1.0
    }
}

pub fn update_animation_transitions(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTransitions, &mut AnimationGraph)>,
) {
    for (mut animation_transitions, mut animation_graph) in query.iter_mut() {
        animation_transitions
            .transitions
            .retain_mut(|animation_transition| {
                animation_transition.update(&time, &mut animation_graph)
            });
    }
}
