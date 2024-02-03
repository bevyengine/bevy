//! The animation graph.

#![allow(missing_docs)]

use std::ops::{Index, IndexMut, Mul};

use bevy_asset::{Assets, Handle};
use bevy_core::Name;
use bevy_derive::Deref;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Has,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_hierarchy::{Children, Parent};
use bevy_reflect::Reflect;
use bevy_render::mesh::morph::MorphWeights;
use bevy_time::Time;
use bevy_transform::components::Transform;
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    visit::{IntoNodeReferences, NodeRef},
    Direction,
};

use crate::{AnimationClip, AnimationPlayer, PlayingAnimation, RepeatAnimation};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationGraph {
    graph: StableDiGraph<AnimationNode, ()>,
    root: AnimationNodeIndex,
}

#[derive(Clone, Copy, Deref, Reflect)]
pub struct AnimationWeight(f32);

#[derive(Clone, Reflect)]
pub struct AnimationNode {
    animation: Option<PlayingAnimation>,
    pub paused: bool,
    pub weight: AnimationWeight,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deref, Reflect)]
pub struct AnimationNodeIndex(NodeIndex<u32>);

struct AnimationContext<
    'gr,
    't,
    'ac,
    'p,
    'ppq,
    'ppa,
    'ppb,
    'ppc,
    'cq,
    'ca,
    'cb,
    'cc,
    'nq,
    'na,
    'nb,
    'nc,
    'tq,
    'ta,
    'tb,
    'tc,
    'mwq,
    'mwa,
    'mwb,
    'mwc,
> {
    animation_graph: &'gr mut AnimationGraph,
    time: &'t Time,
    root_entity: Entity,
    animation_clips: &'ac Assets<AnimationClip>,
    parent: Option<&'p Parent>,
    parents: &'ppq Query<
        'ppa,
        'ppb,
        (
            Has<AnimationPlayer>,
            Has<AnimationGraph>,
            Option<&'ppc Parent>,
        ),
    >,
    children: &'cq Query<'ca, 'cb, &'cc Children>,

    // TODO: These should be abstracted into a list of `Animatable` properties.
    names: &'nq Query<'na, 'nb, &'nc Name>,
    transforms: &'tq Query<'ta, 'tb, &'tc mut Transform>,
    morph_weights: &'mwq Query<'mwa, 'mwb, &'mwc mut MorphWeights>,
}

impl<
        'gr,
        't,
        'ac,
        'p,
        'ppq,
        'ppa,
        'ppb,
        'ppc,
        'cq,
        'ca,
        'cb,
        'cc,
        'nq,
        'na,
        'nb,
        'nc,
        'tq,
        'ta,
        'tb,
        'tc,
        'mwq,
        'mwa,
        'mwb,
        'mwc,
    >
    AnimationContext<
        'gr,
        't,
        'ac,
        'p,
        'ppq,
        'ppa,
        'ppb,
        'ppc,
        'cq,
        'ca,
        'cb,
        'cc,
        'nq,
        'na,
        'nb,
        'nc,
        'tq,
        'ta,
        'tb,
        'tc,
        'mwq,
        'mwa,
        'mwb,
        'mwc,
    >
{
    fn evaluate(&mut self) {
        self.evaluate_node(self.animation_graph.root, AnimationWeight(1.0))
    }

    fn evaluate_node(&mut self, node_index: AnimationNodeIndex, weight: AnimationWeight) {
        let kids: Vec<_> = self
            .animation_graph
            .graph
            .neighbors_directed(*node_index, Direction::Outgoing)
            .collect();
        for &kid in &kids {
            // FIXME: Is `weight * kid_edge.weight()` right?
            self.evaluate_node(kid.into(), weight * self.animation_graph.graph[kid].weight);
        }

        let node = &mut self.animation_graph.graph[*node_index];
        if let Some(ref mut playing_animation) = node.animation {
            crate::apply_animation(
                *weight,
                playing_animation,
                node.paused,
                self.root_entity,
                self.time,
                self.animation_clips,
                self.names,
                self.transforms,
                self.morph_weights,
                self.parent,
                self.parents,
                self.children,
            )
        }
    }
}

impl From<NodeIndex<u32>> for AnimationNodeIndex {
    fn from(value: NodeIndex<u32>) -> Self {
        Self(value)
    }
}

impl Into<NodeIndex<u32>> for AnimationNodeIndex {
    fn into(self) -> NodeIndex<u32> {
        *self
    }
}

impl From<f32> for AnimationWeight {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl Mul<AnimationWeight> for AnimationWeight {
    type Output = AnimationWeight;

    fn mul(self, rhs: AnimationWeight) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

/// A system that updates all playing animations in animation graphs.
pub fn evaluate_animation_graphs(
    time: Res<Time>,
    animation_clips: Res<Assets<AnimationClip>>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&mut Transform>,
    morph_weights: Query<&mut MorphWeights>,
    parents: Query<(Has<AnimationPlayer>, Has<AnimationGraph>, Option<&Parent>)>,
    mut animation_graphs: Query<(Entity, Option<&Parent>, &mut AnimationGraph)>,
) {
    for (root_entity, parent, mut animation_graph) in animation_graphs.iter_mut() {
        // TODO: Update transitions.
        AnimationContext {
            animation_graph: &mut animation_graph,
            time: &time,
            root_entity,
            animation_clips: &animation_clips,
            parent,
            parents: &parents,
            children: &children,
            names: &names,
            transforms: &transforms,
            morph_weights: &morph_weights,
        }
        .evaluate()
    }
}

impl AnimationGraph {
    pub fn new() -> AnimationGraph {
        Self::default()
    }

    pub fn from_clip(
        animation_clip: Handle<AnimationClip>,
    ) -> (AnimationGraph, AnimationNodeIndex) {
        let mut graph = AnimationGraph::new();
        let node = graph.add_clip_node_from(graph.root, animation_clip);
        (graph, node)
    }

    pub fn add_blend_node(&mut self) -> AnimationNodeIndex {
        self.graph.add_node(AnimationNode::new_blend()).into()
    }

    pub fn add_clip_node(&mut self, animation_clip: Handle<AnimationClip>) -> AnimationNodeIndex {
        self.graph
            .add_node(AnimationNode::new_clip(PlayingAnimation {
                animation_clip,
                ..Default::default()
            }))
            .into()
    }

    pub fn add_blend_node_from(&mut self, from: AnimationNodeIndex) -> AnimationNodeIndex {
        let node = self.add_blend_node();
        self.add_edge(from, node);
        node
    }

    pub fn add_clip_node_from(
        &mut self,
        from: AnimationNodeIndex,
        animation_clip: Handle<AnimationClip>,
    ) -> AnimationNodeIndex {
        let node = self.add_clip_node(animation_clip);
        self.add_edge(from, node);
        node
    }

    pub fn add_edge(&mut self, from: AnimationNodeIndex, to: AnimationNodeIndex) -> &mut Self {
        self.graph.add_edge(from.into(), to.into(), ());
        self
    }

    pub fn get_node(&self, index: AnimationNodeIndex) -> Option<&AnimationNode> {
        self.graph.node_weight(*index)
    }

    pub fn get_node_mut(&mut self, index: AnimationNodeIndex) -> Option<&mut AnimationNode> {
        self.graph.node_weight_mut(*index)
    }

    pub fn root_node(&self) -> AnimationNodeIndex {
        self.root
    }

    pub fn nodes(&self) -> impl Iterator<Item = AnimationNodeIndex> + '_ {
        self.graph
            .node_references()
            .map(|reference| AnimationNodeIndex(reference.id()))
    }
}

impl Index<AnimationNodeIndex> for AnimationGraph {
    type Output = AnimationNode;

    fn index(&self, index: AnimationNodeIndex) -> &Self::Output {
        self.get_node(index)
            .expect("Animation node not found in this graph")
    }
}

impl IndexMut<AnimationNodeIndex> for AnimationGraph {
    fn index_mut(&mut self, index: AnimationNodeIndex) -> &mut Self::Output {
        self.get_node_mut(index)
            .expect("Animation node not found in this graph")
    }
}

impl AnimationNode {
    // Note that these don't start paused by default, to avoid confusion with the root node.
    fn new_blend() -> AnimationNode {
        AnimationNode {
            animation: None,
            paused: false,
            weight: AnimationWeight(1.0),
        }
    }

    fn new_clip(animation: PlayingAnimation) -> AnimationNode {
        AnimationNode {
            animation: Some(animation),
            paused: true,
            weight: AnimationWeight(1.0),
        }
    }

    #[doc(alias = "resume")]
    pub fn play(&mut self) -> &mut Self {
        self.set_paused(false)
    }

    pub fn pause(&mut self) -> &mut Self {
        self.set_paused(true)
    }

    pub fn set_paused(&mut self, paused: bool) -> &mut Self {
        self.paused = paused;
        self
    }

    #[doc(alias = "is_paused")]
    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn weight(&self) -> AnimationWeight {
        self.weight
    }

    pub fn set_weight<W>(&mut self, new_weight: W) -> &mut Self
    where
        W: Into<AnimationWeight>,
    {
        self.weight = new_weight.into();
        self
    }

    pub fn repeat_mode(&self) -> RepeatAnimation {
        self.animation
            .as_ref()
            .expect("Only animation clips can repeat")
            .repeat
    }

    #[doc(alias = "set_repeat")]
    pub fn set_repeat_mode(&mut self, repeat_mode: RepeatAnimation) -> &mut Self {
        self.animation
            .as_mut()
            .expect("Only animation clips can repeat")
            .repeat = repeat_mode;
        self
    }

    pub fn repeat_forever(&mut self) -> &mut Self {
        self.set_repeat_mode(RepeatAnimation::Forever)
    }

    pub fn repeat_n(&mut self, count: u32) -> &mut Self {
        self.set_repeat_mode(RepeatAnimation::Count(count))
    }

    pub fn no_repeat(&mut self) -> &mut Self {
        self.set_repeat_mode(RepeatAnimation::Never)
    }

    pub fn speed(&self) -> f32 {
        self.animation
            .as_ref()
            .expect("Only animation clips have a speed")
            .speed
    }

    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.animation
            .as_mut()
            .expect("Only animation clips have a speed")
            .speed = speed;
        self
    }

    pub fn elapsed(&self) -> f32 {
        self.animation
            .as_ref()
            .expect("Only animation clips have elapsed time")
            .elapsed
    }

    pub fn seek_time(&self) -> f32 {
        self.animation
            .as_ref()
            .expect("Only animation clips have a seek time")
            .seek_time
    }

    #[doc(alias = "seek_to")]
    pub fn set_seek_time(&mut self, new_seek_time: f32) -> &mut Self {
        self.animation
            .as_mut()
            .expect("Only animation clips have a seek time")
            .seek_time = new_seek_time;
        self
    }

    pub fn completions(&self) -> u32 {
        self.animation
            .as_ref()
            .expect("Only animation clips record the number of completions")
            .completions
    }

    #[doc(alias = "replay")]
    pub fn restart(&mut self) -> &mut Self {
        let animation = self
            .animation
            .as_mut()
            .expect("Only animation clips can be restarted");
        animation.completions = 0;
        animation.elapsed = 0.0;
        animation.seek_time = 0.0;
        self
    }
}

impl Default for AnimationGraph {
    fn default() -> Self {
        let mut graph = StableDiGraph::new();
        let root = graph.add_node(AnimationNode::new_blend());
        AnimationGraph {
            graph,
            root: root.into(),
        }
    }
}
