//! The animation graph.

use std::ops::{Index, IndexMut};

use bevy_asset::{Asset, Handle};
use bevy_reflect::Reflect;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Dfs, Visitable};
use petgraph::Graph;

use crate::AnimationClip;

#[derive(Asset, Reflect, Debug)]
pub struct AnimationGraph {
    #[reflect(ignore)]
    pub(crate) graph: AnimationDiGraph,
    #[reflect(ignore)]
    pub root: NodeIndex,
    // Cached topological ordering.
    //#[reflect(ignore)]
    //schedule: Vec<AnimationNodeIndex>,
}

pub type AnimationDiGraph = DiGraph<AnimationGraphNode, (), u32>;

pub type AnimationNodeIndex = NodeIndex<u32>;

#[derive(Reflect, Debug)]
pub struct AnimationGraphNode {
    pub clip: Option<Handle<AnimationClip>>,
    pub weight: f32,
}

impl AnimationGraph {
    pub fn new() -> Self {
        let mut graph = DiGraph::default();
        let root = graph.add_node(AnimationGraphNode::default());
        Self { graph, root }
    }

    pub fn add_clip(
        &mut self,
        clip: Handle<AnimationClip>,
        weight: f32,
        parent: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            clip: Some(clip),
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    pub fn add_blend(&mut self, weight: f32, parent: AnimationNodeIndex) -> AnimationNodeIndex {
        let node_index = self
            .graph
            .add_node(AnimationGraphNode { clip: None, weight });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, ());
    }

    pub fn get(&self, animation: AnimationNodeIndex) -> Option<&AnimationGraphNode> {
        self.graph.node_weight(animation)
    }

    pub fn get_mut(&mut self, animation: AnimationNodeIndex) -> Option<&mut AnimationGraphNode> {
        self.graph.node_weight_mut(animation)
    }

    pub fn dfs(
        &self,
    ) -> Dfs<AnimationNodeIndex, <Graph<AnimationGraphNode, ()> as Visitable>::Map> {
        Dfs::new(&self.graph, self.root)
    }
}

impl Index<AnimationNodeIndex> for AnimationGraph {
    type Output = AnimationGraphNode;

    fn index(&self, index: AnimationNodeIndex) -> &Self::Output {
        &self.graph[index]
    }
}

impl IndexMut<AnimationNodeIndex> for AnimationGraph {
    fn index_mut(&mut self, index: AnimationNodeIndex) -> &mut Self::Output {
        &mut self.graph[index]
    }
}

impl Default for AnimationGraphNode {
    fn default() -> Self {
        Self {
            clip: None,
            weight: 1.0,
        }
    }
}

impl Default for AnimationGraph {
    fn default() -> Self {
        Self::new()
    }
}
