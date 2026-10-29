//! The blend graph, which allows animations to be blended together.

use core::{
    fmt::Write,
    iter,
    ops::{Index, IndexMut, Range},
};
use std::io;

use bevy_asset::{
    io::Reader, Asset, AssetEvent, AssetId, AssetLoader, AssetPath, Assets, Handle, LoadContext,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    message::MessageReader,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Local, Res, ResMut},
    template::FromTemplate,
};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bitvec::vec::BitVec;
use derive_more::derive::From;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    Direction,
};
use ron::de::SpannedError;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

use crate::{AnimationClip, AnimationTargetId};

/// A graph structure that describes how animation clips are to be blended
/// together.
///
/// Applications frequently want to be able to play multiple animations at once
/// and to fine-tune the influence that animations have on a skinned mesh. Bevy
/// uses a *blend graph* to store this information. Blend graphs are a
/// directed acyclic graph (DAG) that describes how animations are to be
/// weighted and combined together. Every frame, Bevy evaluates the graph from
/// the root and blends the animations together in a bottom-up fashion to
/// produce the final pose.
///
/// There are three types of nodes: *blend nodes*, *add nodes*, and *clip
/// nodes*, all of which can have an associated weight. Blend nodes and add
/// nodes have no associated animation clip and combine the animations of their
/// children according to those children's weights. Clip nodes specify an
/// animation clip to play. When a graph is created, it starts with only a
/// single blend node, the root node.
///
/// For example, consider the following graph:
///
/// ```text
/// ┌────────────┐
/// │            │
/// │    Idle    ├─────────────────────┐
/// │            │                     │
/// └────────────┘                     │
///                                    │
/// ┌────────────┐                     │  ┌────────────┐
/// │            │                     │  │            │
/// │    Run     ├──┐                  ├──┤    Root    │
/// │            │  │  ┌────────────┐  │  │            │
/// └────────────┘  │  │   Blend    │  │  └────────────┘
///                 ├──┤            ├──┘
/// ┌────────────┐  │  │    0.5     │
/// │            │  │  └────────────┘
/// │    Walk    ├──┘
/// │            │
/// └────────────┘
/// ```
///
/// In this case, assuming that Idle, Run, and Walk are all playing with weight
/// 1.0, the Run and Walk animations will be equally blended together, then
/// their weights will be halved and finally blended with the Idle animation.
/// Thus the weight of Run and Walk are effectively half of the weight of Idle.
///
/// Nodes can optionally have a *mask*, a bitfield that restricts the set of
/// animation targets that the node and its descendants affect. Each bit in the
/// mask corresponds to a *mask group*, which is a set of animation targets
/// (bones). An animation target can belong to any number of mask groups within
/// the context of a blend graph.
///
/// When the appropriate bit is set in a node's mask, neither the node nor its
/// descendants will animate any animation targets belonging to that mask group.
/// That is, setting a mask bit to 1 *disables* the animation targets in that
/// group. If an animation target belongs to multiple mask groups, masking any
/// one of the mask groups that it belongs to will mask that animation target.
/// (Thus an animation target will only be animated if *all* of its mask groups
/// are unmasked.)
///
/// A common use of masks is to allow characters to hold objects. For this, the
/// typical workflow is to assign each character's hand to a mask group. Then,
/// when the character picks up an object, the application masks out the hand
/// that the object is held in for the character's animation set, then positions
/// the hand's digits as necessary to grasp the object. The character's
/// animations will continue to play but will not affect the hand, which will
/// continue to be depicted as holding the object.
///
/// Blend graphs are assets and can be serialized to and loaded from [RON]
/// files. Canonically, such files have an `.blendgraph.ron` extension.
///
/// The blend graph implements [RFC 51]. See that document for more
/// information.
///
/// [RON]: https://github.com/ron-rs/ron
///
/// [RFC 51]: https://github.com/bevyengine/rfcs/blob/main/rfcs/51-animation-composition.md
#[derive(Asset, Reflect, Clone, Debug)]
#[reflect(Debug, Clone)]
pub struct BlendGraph {
    /// The `petgraph` data structure that defines the blend graph.
    pub graph: BlendDiGraph,

    /// The index of the root node in the blend graph.
    pub root: NodeIndex,

    /// The mask groups that each animation target (bone) belongs to.
    ///
    /// Each value in this map is a bitfield, in which 0 in bit position N
    /// indicates that the animation target doesn't belong to mask group N, and
    /// a 1 in position N indicates that the animation target does belong to
    /// mask group N.
    ///
    /// Animation targets not in this collection are treated as though they
    /// don't belong to any mask groups.
    pub mask_groups: HashMap<AnimationTargetId, AnimationMask>,
}

/// A [`Handle`] to the [`BlendGraph`] to be used by the [`AnimationPlayer`](crate::AnimationPlayer) on the same entity.
#[derive(
    Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From, FromTemplate,
)]
#[reflect(Component, Default, Clone)]
pub struct BlendGraphHandle(pub Handle<BlendGraph>);

impl From<BlendGraphHandle> for AssetId<BlendGraph> {
    fn from(handle: BlendGraphHandle) -> Self {
        handle.id()
    }
}

impl From<&BlendGraphHandle> for AssetId<BlendGraph> {
    fn from(handle: &BlendGraphHandle) -> Self {
        handle.id()
    }
}

/// A type alias for the `petgraph` data structure that defines the animation
/// graph.
pub type BlendDiGraph = DiGraph<BlendGraphNode, (), u32>;

/// The index of either an animation or blend node in the blend graph.
///
/// These indices are the way that [animation players] identify each animation.
///
/// [animation players]: crate::AnimationPlayer
pub type BlendNodeIndex = NodeIndex<u32>;

/// An individual node within a blend graph.
///
/// The [`BlendGraphNode::node_type`] field specifies the type of node: one
/// of a *clip node*, a *blend node*, or an *add node*. Clip nodes, the leaves
/// of the graph, contain animation clips to play. Blend and add nodes describe
/// how to combine their children to produce a final animation.
#[derive(Clone, Reflect, Debug)]
#[reflect(Clone)]
pub struct BlendGraphNode {
    /// Blend node data specific to the type of node (clip, blend, or add).
    ///
    /// In the case of clip nodes, this contains the actual animation clip
    /// associated with the node.
    pub node_type: BlendNodeType,

    /// A bitfield specifying the mask groups that this node and its descendants
    /// will not affect.
    ///
    /// A 0 in bit N indicates that this node and its descendants *can* animate
    /// animation targets in mask group N, while a 1 in bit N indicates that
    /// this node and its descendants *cannot* animate mask group N.
    pub mask: AnimationMask,

    /// The weight of this node, which signifies its contribution in blending.
    ///
    /// Note that this does not propagate down the graph hierarchy; rather,
    /// each [Blend] and [Add] node uses the weights of its children to determine
    /// the total animation that is accumulated at that node. The parent node's
    /// weight is used only to determine the contribution of that total animation
    /// in *further* blending.
    ///
    /// In other words, it is as if the blend node is replaced by a single clip
    /// node consisting of the blended animation with the weight specified at the
    /// blend node.
    ///
    /// For animation clips, this weight is also multiplied by the [active animation weight]
    /// before being applied.
    ///
    /// [Blend]: BlendNodeType::Blend
    /// [Add]: BlendNodeType::Add
    /// [active animation weight]: crate::ActiveAnimation::weight
    pub weight: f32,
}

/// Blend node data specific to the type of node (clip, blend, or add).
///
/// In the case of clip nodes, this contains the actual animation clip
/// associated with the node.
#[derive(Clone, Default, Reflect, Debug)]
#[reflect(Clone)]
pub enum BlendNodeType {
    /// A *clip node*, which plays an animation clip.
    ///
    /// These are always the leaves of the graph.
    Clip(Handle<AnimationClip>),

    /// A *blend node*, which blends its children according to their weights.
    ///
    /// The weights of all the children of this node are normalized to 1.0.
    #[default]
    Blend,

    /// An *additive blend node*, which combines the animations of its children
    /// additively.
    ///
    /// The weights of all the children of this node are *not* normalized to
    /// 1.0. Rather, each child is multiplied by its respective weight and
    /// added in sequence.
    ///
    /// Add nodes are primarily useful for superimposing an animation for a
    /// portion of a rig on top of the main animation. For example, an add node
    /// could superimpose a weapon attack animation for a character's limb on
    /// top of a running animation to produce an animation of a character
    /// attacking while running.
    Add,
}

/// An [`AssetLoader`] that can load [`BlendGraph`]s as assets.
///
/// The canonical extension for [`BlendGraph`]s is `.blendgraph.ron`. Plain
/// `.blendgraph` is supported as well.
#[derive(Default, TypePath)]
pub struct BlendGraphAssetLoader;

/// Errors that can occur when serializing blend graphs to RON.
#[derive(Error, Debug)]
pub enum BlendGraphSaveError {
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// An error occurred in RON serialization.
    #[error(transparent)]
    Ron(#[from] ron::Error),
    /// An error occurred converting the graph to its serialization form.
    #[error(transparent)]
    ConvertToSerialized(#[from] NonPathHandleError),
}

/// Errors that can occur when deserializing blend graphs from RON.
#[derive(Error, Debug)]
pub enum BlendGraphLoadError {
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// An error occurred in RON deserialization.
    #[error(transparent)]
    Ron(#[from] ron::Error),
    /// An error occurred in RON deserialization, and the location of the error
    /// is supplied.
    #[error(transparent)]
    SpannedRon(#[from] SpannedError),
    /// The deserialized graph contained legacy data that we no longer support.
    #[error(
        "The deserialized BlendGraph contained an AnimationClip referenced by an AssetId, \
    which is no longer supported. Consider manually deserializing the SerializedBlendGraph \
    type and determine how to migrate any SerializedAnimationClip::AssetId animation clips"
    )]
    GraphContainsLegacyAssetId,
}

/// Acceleration structures for blend graphs that allows Bevy to evaluate
/// them quickly.
///
/// These are kept up to date as [`BlendGraph`] instances are added,
/// modified, and removed.
#[derive(Default, Reflect, Resource)]
pub struct ThreadedBlendGraphs {
    /// The mapping from each blend graph to its threaded blend graph
    /// acceleration structure.
    ///
    /// Note that, because graphs can load before their clips do, when a clip
    /// loads, we have to invalidate portions of this table.
    pub(crate) threaded_graphs: HashMap<AssetId<BlendGraph>, ThreadedBlendGraph>,

    /// A mapping from the ID of each animation clip to the IDs of the graphs
    /// that reference that clip.
    clip_to_graphs: HashMap<AssetId<AnimationClip>, HashSet<AssetId<BlendGraph>>>,
}

/// An acceleration structure for a blend graph that allows Bevy to
/// evaluate it quickly.
///
/// This is kept up to date as the associated [`BlendGraph`] instance is
/// added, modified, or removed.
#[derive(Default, Reflect)]
pub struct ThreadedBlendGraph {
    /// A cached postorder traversal of the graph.
    ///
    /// The node indices here are stored in postorder. Siblings are stored in
    /// descending order. This is because the
    /// [`AnimationCurveEvaluator`](`crate::animation_curves::AnimationCurveEvaluator`) uses a stack for
    /// evaluation. Consider this graph:
    ///
    /// ```text
    ///             ┌─────┐
    ///             │     │
    ///             │  1  │
    ///             │     │
    ///             └──┬──┘
    ///                │
    ///        ┌───────┼───────┐
    ///        │       │       │
    ///        ▼       ▼       ▼
    ///     ┌─────┐ ┌─────┐ ┌─────┐
    ///     │     │ │     │ │     │
    ///     │  2  │ │  3  │ │  4  │
    ///     │     │ │     │ │     │
    ///     └──┬──┘ └─────┘ └─────┘
    ///        │
    ///    ┌───┴───┐
    ///    │       │
    ///    ▼       ▼
    /// ┌─────┐ ┌─────┐
    /// │     │ │     │
    /// │  5  │ │  6  │
    /// │     │ │     │
    /// └─────┘ └─────┘
    /// ```
    ///
    /// The postorder traversal in this case will be (4, 3, 6, 5, 2, 1).
    ///
    /// The fact that the children of each node are sorted in reverse ensures
    /// that, at each level, the order of blending proceeds in ascending order
    /// by node index, as we guarantee. To illustrate this, consider the way
    /// the graph above is evaluated. (Interpolation is represented with the ⊕
    /// symbol.)
    ///
    /// | Step | Node | Operation  | Stack (after operation) | Blend Register |
    /// | ---- | ---- | ---------- | ----------------------- | -------------- |
    /// | 1    | 4    | Push       | 4                       |                |
    /// | 2    | 3    | Push       | 4 3                     |                |
    /// | 3    | 6    | Push       | 4 3 6                   |                |
    /// | 4    | 5    | Push       | 4 3 6 5                 |                |
    /// | 5    | 2    | Blend 5    | 4 3 6                   | 5              |
    /// | 6    | 2    | Blend 6    | 4 3                     | 5 ⊕ 6          |
    /// | 7    | 2    | Push Blend | 4 3 2                   |                |
    /// | 8    | 1    | Blend 2    | 4 3                     | 2              |
    /// | 9    | 1    | Blend 3    | 4                       | 2 ⊕ 3          |
    /// | 10   | 1    | Blend 4    |                         | 2 ⊕ 3 ⊕ 4      |
    /// | 11   | 1    | Push Blend | 1                       |                |
    /// | 12   |      | Commit     |                         |                |
    pub threaded_graph: Vec<BlendNodeIndex>,

    /// A mapping from each parent node index to the range of its children
    /// within [`Self::sorted_edges`].
    ///
    /// This allows for quick lookup of the children of each node, sorted in
    /// ascending order of node index, without having to sort the result of the
    /// `petgraph` traversal functions every frame.
    pub sorted_edge_list_ranges: Vec<Range<u32>>,

    /// A list of the children of each node, sorted in ascending order.
    pub sorted_edges: Vec<BlendNodeIndex>,

    /// A mapping from node index to a bitfield specifying the mask groups that
    /// this node masks *out* (i.e. doesn't animate).
    ///
    /// A 1 in bit position N indicates that this node doesn't animate any
    /// targets of mask group N.
    pub computed_masks: Vec<u64>,

    /// All animation clips that this graph contains clip nodes for.
    pub animation_clips: HashSet<AssetId<AnimationClip>>,

    /// A mapping from each animation target to the *threaded subgraph* for that
    /// target.
    ///
    /// See [`ThreadedBlendSubgraph`] for more information.
    pub animation_target_to_threaded_subgraph: HashMap<AnimationTargetId, ThreadedBlendSubgraph>,
}

/// A subgraph of a [`ThreadedBlendGraph`] that contains only the nodes
/// necessary to animate a single target.
///
/// It's common for a single graph to contain animations for many unrelated
/// targets: for example, a character might have a single blend graph that
/// blends both facial animations and locomotion animations. Now, internally,
/// Bevy evaluates animations for all targets individually and in parallel. If
/// Bevy evaluated the entire graph separately for each target, this would be
/// inefficient: for instance, Bevy would be evaluating and blending the
/// locomotion animations for the facial bones, even though locomotion
/// animations don't typically affect facial bones.
///
/// To remedy this problem, Bevy creates individual *subgraphs* for each target.
/// These subgraphs contain only the clip nodes that affect a target, as well as
/// any add or blend nodes that transitively blend those clip nodes. The end
/// result is a graph tailored to an animation target that, when evaluated for a
/// target, produces the exact same result as if the entire graph had been
/// evaluated for that target, but contains only the nodes that were relevant to
/// produce that result.
///
/// The fields in this structure have identical meanings to the corresponding
/// fields in the [`ThreadedBlendGraph`] structure.
#[derive(Default, Reflect)]
pub struct ThreadedBlendSubgraph {
    /// A cached postorder traversal of the graph, containing only the nodes
    /// that are needed to animate this target.
    ///
    /// See [`ThreadedBlendGraph::threaded_graph`] for more information.
    pub threaded_graph: Vec<BlendNodeIndex>,

    /// A mapping from the index of each element of [`Self::threaded_graph`] to
    /// the index of the first such edge within [`Self::sorted_edges`].
    ///
    /// In other words, this array is parallel to the [`Self::threaded_graph`]
    /// array.
    ///
    /// See [`ThreadedBlendGraph::sorted_edge_list_ranges`] for more
    /// information.
    pub sorted_edge_list_offsets: Vec<u32>,

    /// A list of the children of each node, sorted in ascending order.
    pub sorted_edges: Vec<BlendNodeIndex>,
}

/// A version of [`BlendGraph`] suitable for serializing as an asset.
///
/// Blend nodes can refer to external animation clips, and the [`AssetId`]
/// is typically not sufficient to identify the clips, since the
/// [`bevy_asset::AssetServer`] assigns IDs in unpredictable ways. That fact
/// motivates this type, which replaces the `Handle<AnimationClip>` with an
/// asset path.  Loading a blend graph via the [`bevy_asset::AssetServer`]
/// actually loads a serialized instance of this type, as does serializing an
/// [`BlendGraph`] through `serde`.
#[derive(Serialize, Deserialize)]
pub struct SerializedBlendGraph {
    /// Corresponds to the `graph` field on [`BlendGraph`].
    pub graph: DiGraph<SerializedBlendGraphNode, (), u32>,
    /// Corresponds to the `root` field on [`BlendGraph`].
    pub root: NodeIndex,
    /// Corresponds to the `mask_groups` field on [`BlendGraph`].
    pub mask_groups: HashMap<AnimationTargetId, AnimationMask>,
}

/// A version of [`BlendGraphNode`] suitable for serializing as an asset.
///
/// See the comments in [`SerializedBlendGraph`] for more information.
#[derive(Serialize, Deserialize)]
pub struct SerializedBlendGraphNode {
    /// Corresponds to the `node_type` field on [`BlendGraphNode`].
    pub node_type: SerializedBlendNodeType,
    /// Corresponds to the `mask` field on [`BlendGraphNode`].
    pub mask: AnimationMask,
    /// Corresponds to the `weight` field on [`BlendGraphNode`].
    pub weight: f32,
}

/// A version of [`BlendNodeType`] suitable for serializing as part of a
/// [`SerializedBlendGraphNode`] asset.
#[derive(Serialize, Deserialize)]
pub enum SerializedBlendNodeType {
    /// Corresponds to [`BlendNodeType::Clip`].
    Clip(AssetPath<'static>),
    /// Corresponds to [`BlendNodeType::Blend`].
    Blend,
    /// Corresponds to [`BlendNodeType::Add`].
    Add,
}

/// The type of an animation mask bitfield.
///
/// Bit N corresponds to mask group N.
///
/// Because this is a 64-bit value, there is currently a limitation of 64 mask
/// groups per blend graph.
pub type AnimationMask = u64;

impl BlendGraph {
    /// Creates a new blend graph with a root node and no other nodes.
    pub fn new() -> Self {
        let mut graph = DiGraph::default();
        let root = graph.add_node(BlendGraphNode::default());
        Self {
            graph,
            root,
            mask_groups: HashMap::default(),
        }
    }

    /// A convenience function for creating an [`BlendGraph`] from a single
    /// [`AnimationClip`].
    ///
    /// The clip will be a direct child of the root with weight 1.0. Both the
    /// graph and the index of the added node are returned as a tuple.
    pub fn from_clip(clip: Handle<AnimationClip>) -> (Self, BlendNodeIndex) {
        let mut graph = Self::new();
        let node_index = graph.add_clip(clip, 1.0, graph.root);
        (graph, node_index)
    }

    /// A convenience method to create an [`BlendGraph`]s with an iterator
    /// of clips.
    ///
    /// All of the animation clips will be direct children of the root with
    /// weight 1.0.
    ///
    /// Returns the graph and indices of the new nodes.
    pub fn from_clips<'a, I>(clips: I) -> (Self, Vec<BlendNodeIndex>)
    where
        I: IntoIterator<Item = Handle<AnimationClip>>,
        <I as IntoIterator>::IntoIter: 'a,
    {
        let mut graph = Self::new();
        let indices = graph.add_clips(clips, 1.0, graph.root).collect();
        (graph, indices)
    }

    /// Adds an [`AnimationClip`] to the blend graph with the given weight
    /// and returns its index.
    ///
    /// The animation clip will be the child of the given parent. The resulting
    /// node will have no mask.
    pub fn add_clip(
        &mut self,
        clip: Handle<AnimationClip>,
        weight: f32,
        parent: BlendNodeIndex,
    ) -> BlendNodeIndex {
        let node_index = self.graph.add_node(BlendGraphNode {
            node_type: BlendNodeType::Clip(clip),
            mask: 0,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds an [`AnimationClip`] to the blend graph with the given weight
    /// and mask, and returns its index.
    ///
    /// The animation clip will be the child of the given parent.
    pub fn add_clip_with_mask(
        &mut self,
        clip: Handle<AnimationClip>,
        mask: AnimationMask,
        weight: f32,
        parent: BlendNodeIndex,
    ) -> BlendNodeIndex {
        let node_index = self.graph.add_node(BlendGraphNode {
            node_type: BlendNodeType::Clip(clip),
            mask,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// A convenience method to add multiple [`AnimationClip`]s to the animation
    /// graph.
    ///
    /// All of the animation clips will have the same weight and will be
    /// parented to the same node.
    ///
    /// Returns the indices of the new nodes.
    pub fn add_clips<'a, I>(
        &'a mut self,
        clips: I,
        weight: f32,
        parent: BlendNodeIndex,
    ) -> impl Iterator<Item = BlendNodeIndex> + 'a
    where
        I: IntoIterator<Item = Handle<AnimationClip>>,
        <I as IntoIterator>::IntoIter: 'a,
    {
        clips
            .into_iter()
            .map(move |clip| self.add_clip(clip, weight, parent))
    }

    /// Adds a blend node to the blend graph with the given weight and
    /// returns its index.
    ///
    /// The blend node will be placed under the supplied `parent` node. During
    /// animation evaluation, the descendants of this blend node will have their
    /// weights multiplied by the weight of the blend. The blend node will have
    /// no mask.
    pub fn add_blend(&mut self, weight: f32, parent: BlendNodeIndex) -> BlendNodeIndex {
        let node_index = self.graph.add_node(BlendGraphNode {
            node_type: BlendNodeType::Blend,
            mask: 0,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds a blend node to the blend graph with the given weight and
    /// returns its index.
    ///
    /// The blend node will be placed under the supplied `parent` node. During
    /// animation evaluation, the descendants of this blend node will have their
    /// weights multiplied by the weight of the blend. Neither this node nor its
    /// descendants will affect animation targets that belong to mask groups not
    /// in the given `mask`.
    pub fn add_blend_with_mask(
        &mut self,
        mask: AnimationMask,
        weight: f32,
        parent: BlendNodeIndex,
    ) -> BlendNodeIndex {
        let node_index = self.graph.add_node(BlendGraphNode {
            node_type: BlendNodeType::Blend,
            mask,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds a blend node to the blend graph with the given weight and
    /// returns its index.
    ///
    /// The blend node will be placed under the supplied `parent` node. During
    /// animation evaluation, the descendants of this blend node will have their
    /// weights multiplied by the weight of the blend. The blend node will have
    /// no mask.
    pub fn add_additive_blend(&mut self, weight: f32, parent: BlendNodeIndex) -> BlendNodeIndex {
        let node_index = self.graph.add_node(BlendGraphNode {
            node_type: BlendNodeType::Add,
            mask: 0,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds a blend node to the blend graph with the given weight and
    /// returns its index.
    ///
    /// The blend node will be placed under the supplied `parent` node. During
    /// animation evaluation, the descendants of this blend node will have their
    /// weights multiplied by the weight of the blend. Neither this node nor its
    /// descendants will affect animation targets that belong to mask groups not
    /// in the given `mask`.
    pub fn add_additive_blend_with_mask(
        &mut self,
        mask: AnimationMask,
        weight: f32,
        parent: BlendNodeIndex,
    ) -> BlendNodeIndex {
        let node_index = self.graph.add_node(BlendGraphNode {
            node_type: BlendNodeType::Add,
            mask,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds an edge from the edge `from` to `to`, making `to` a child of
    /// `from`.
    ///
    /// The behavior is unspecified if adding this produces a cycle in the
    /// graph.
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, ());
    }

    /// Removes an edge between `from` and `to` if it exists.
    ///
    /// Returns true if the edge was successfully removed or false if no such
    /// edge existed.
    pub fn remove_edge(&mut self, from: NodeIndex, to: NodeIndex) -> bool {
        self.graph
            .find_edge(from, to)
            .map(|edge| self.graph.remove_edge(edge))
            .is_some()
    }

    /// Returns the [`BlendGraphNode`] associated with the given index.
    ///
    /// If no node with the given index exists, returns `None`.
    pub fn get(&self, animation: BlendNodeIndex) -> Option<&BlendGraphNode> {
        self.graph.node_weight(animation)
    }

    /// Returns a mutable reference to the [`BlendGraphNode`] associated
    /// with the given index.
    ///
    /// If no node with the given index exists, returns `None`.
    pub fn get_mut(&mut self, animation: BlendNodeIndex) -> Option<&mut BlendGraphNode> {
        self.graph.node_weight_mut(animation)
    }

    /// Returns an iterator over the [`BlendGraphNode`]s in this graph.
    pub fn nodes(&self) -> impl Iterator<Item = BlendNodeIndex> {
        self.graph.node_indices()
    }

    /// Serializes the blend graph to the given [`Write`]r in RON format.
    ///
    /// If writing to a file, it can later be loaded with the
    /// [`BlendGraphAssetLoader`] to reconstruct the graph.
    pub fn save<W>(&self, writer: &mut W) -> Result<(), BlendGraphSaveError>
    where
        W: Write,
    {
        let mut ron_serializer = ron::ser::Serializer::new(writer, None)?;
        let serialized_graph: SerializedBlendGraph = self.clone().try_into()?;
        Ok(serialized_graph.serialize(&mut ron_serializer)?)
    }

    /// Adds an animation target (bone) to the mask group with the given ID.
    ///
    /// Calling this method multiple times with the same animation target but
    /// different mask groups will result in that target being added to all of
    /// the specified groups.
    pub fn add_target_to_mask_group(&mut self, target: AnimationTargetId, mask_group: u32) {
        *self.mask_groups.entry(target).or_default() |= 1 << mask_group;
    }
}

impl BlendGraphNode {
    /// Masks out the mask groups specified by the given `mask` bitfield.
    ///
    /// A 1 in bit position N causes this function to mask out mask group N, and
    /// thus neither this node nor its descendants will animate any animation
    /// targets that belong to group N.
    pub fn add_mask(&mut self, mask: AnimationMask) -> &mut Self {
        self.mask |= mask;
        self
    }

    /// Unmasks the mask groups specified by the given `mask` bitfield.
    ///
    /// A 1 in bit position N causes this function to unmask mask group N, and
    /// thus this node and its descendants will be allowed to animate animation
    /// targets that belong to group N, unless another mask masks those targets
    /// out.
    pub fn remove_mask(&mut self, mask: AnimationMask) -> &mut Self {
        self.mask &= !mask;
        self
    }

    /// Masks out the single mask group specified by `group`.
    ///
    /// After calling this function, neither this node nor its descendants will
    /// animate any animation targets that belong to the given `group`.
    pub fn add_mask_group(&mut self, group: u32) -> &mut Self {
        self.add_mask(1 << group)
    }

    /// Unmasks the single mask group specified by `group`.
    ///
    /// After calling this function, this node and its descendants will be
    /// allowed to animate animation targets that belong to the given `group`,
    /// unless another mask masks those targets out.
    pub fn remove_mask_group(&mut self, group: u32) -> &mut Self {
        self.remove_mask(1 << group)
    }
}

impl Index<BlendNodeIndex> for BlendGraph {
    type Output = BlendGraphNode;

    fn index(&self, index: BlendNodeIndex) -> &Self::Output {
        &self.graph[index]
    }
}

impl IndexMut<BlendNodeIndex> for BlendGraph {
    fn index_mut(&mut self, index: BlendNodeIndex) -> &mut Self::Output {
        &mut self.graph[index]
    }
}

impl Default for BlendGraphNode {
    fn default() -> Self {
        Self {
            node_type: Default::default(),
            mask: 0,
            weight: 1.0,
        }
    }
}

impl Default for BlendGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for BlendGraphAssetLoader {
    type Asset = BlendGraph;

    type Settings = ();

    type Error = BlendGraphLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Deserialize a `SerializedBlendGraph` directly, so that we can
        // get the list of the animation clips it refers to and load them.
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let serialized_blend_graph = SerializedBlendGraph::deserialize(&mut deserializer)
            .map_err(|err| deserializer.span_error(err))?;

        // Load all `AssetPath`s to convert from a `SerializedBlendGraph` to a real
        // `BlendGraph`. This is effectively a `DiGraph::map`, but this allows us to return
        // errors.
        let mut blend_graph = DiGraph::with_capacity(
            serialized_blend_graph.graph.node_count(),
            serialized_blend_graph.graph.edge_count(),
        );

        for serialized_node in serialized_blend_graph.graph.node_weights() {
            blend_graph.add_node(BlendGraphNode {
                node_type: match serialized_node.node_type {
                    SerializedBlendNodeType::Clip(ref path) => {
                        BlendNodeType::Clip(load_context.load(path.clone()))
                    }
                    SerializedBlendNodeType::Blend => BlendNodeType::Blend,
                    SerializedBlendNodeType::Add => BlendNodeType::Add,
                },
                mask: serialized_node.mask,
                weight: serialized_node.weight,
            });
        }
        for edge in serialized_blend_graph.graph.raw_edges() {
            blend_graph.add_edge(edge.source(), edge.target(), ());
        }
        Ok(BlendGraph {
            graph: blend_graph,
            root: serialized_blend_graph.root,
            mask_groups: serialized_blend_graph.mask_groups,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["blendgraph", "blendgraph.ron"]
    }
}

impl TryFrom<BlendGraph> for SerializedBlendGraph {
    type Error = NonPathHandleError;

    fn try_from(blend_graph: BlendGraph) -> Result<Self, NonPathHandleError> {
        // Convert all the `Handle<AnimationClip>` to AssetPath, so that
        // `BlendGraphAssetLoader` can load them. This is effectively just doing a
        // `DiGraph::map`, except we need to return an error if any handles aren't associated to a
        // path.
        let mut serialized_graph = DiGraph::with_capacity(
            blend_graph.graph.node_count(),
            blend_graph.graph.edge_count(),
        );
        for node in blend_graph.graph.node_weights() {
            serialized_graph.add_node(SerializedBlendGraphNode {
                weight: node.weight,
                mask: node.mask,
                node_type: match node.node_type {
                    BlendNodeType::Clip(ref clip) => match clip.path() {
                        Some(path) => SerializedBlendNodeType::Clip(path.clone()),
                        None => return Err(NonPathHandleError),
                    },
                    BlendNodeType::Blend => SerializedBlendNodeType::Blend,
                    BlendNodeType::Add => SerializedBlendNodeType::Add,
                },
            });
        }
        for edge in blend_graph.graph.raw_edges() {
            serialized_graph.add_edge(edge.source(), edge.target(), ());
        }
        Ok(Self {
            graph: serialized_graph,
            root: blend_graph.root,
            mask_groups: blend_graph.mask_groups,
        })
    }
}

/// Error for when only path [`Handle`]s are supported.
#[derive(Error, Debug)]
#[error(
    "BlendGraph contains a handle to an AnimationClip that does not correspond to an asset path"
)]
pub struct NonPathHandleError;

/// A system that creates, updates, and removes [`ThreadedBlendGraph`]
/// structures for every changed [`BlendGraph`].
///
/// The [`ThreadedBlendGraph`] contains acceleration structures that allow
/// for quick evaluation of that graph's animations.
pub(crate) fn thread_blend_graphs(
    threaded_blend_graphs: ResMut<ThreadedBlendGraphs>,
    blend_graphs: Res<Assets<BlendGraph>>,
    animation_clips: Res<Assets<AnimationClip>>,
    mut blend_graph_asset_events: MessageReader<AssetEvent<BlendGraph>>,
    mut animation_clip_asset_events: MessageReader<AssetEvent<AnimationClip>>,
    mut animation_target_graphs_rebuilt_this_frame: Local<HashSet<AssetId<BlendGraph>>>,
) {
    // Early out here to avoid running `threaded_blend_graphs.into_inner()`
    // and marking `ThreadedBlendGraphs` as changed if there are obviously
    // no changes.
    if blend_graph_asset_events.is_empty() && animation_clip_asset_events.is_empty() {
        return;
    }

    let threaded_blend_graphs = threaded_blend_graphs.into_inner();

    animation_target_graphs_rebuilt_this_frame.clear();

    for blend_graph_asset_event in blend_graph_asset_events.read() {
        match *blend_graph_asset_event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                // Fetch the blend graph.
                let Some(blend_graph) = blend_graphs.get(id) else {
                    continue;
                };

                // Reuse the allocation if possible.
                let mut threaded_blend_graph =
                    match threaded_blend_graphs.threaded_graphs.remove(&id) {
                        None => ThreadedBlendGraph::default(),
                        Some(mut existing_threaded_blend_graph) => {
                            existing_threaded_blend_graph.remove_from_clip_to_graphs_table(
                                id,
                                &mut threaded_blend_graphs.clip_to_graphs,
                            );
                            existing_threaded_blend_graph.clear();
                            existing_threaded_blend_graph
                        }
                    };

                // Recursively thread the graph in postorder.
                threaded_blend_graph.init(blend_graph);
                threaded_blend_graph
                    .populate_clip_to_graphs_table(id, &mut threaded_blend_graphs.clip_to_graphs);
                threaded_blend_graph.build_from(&blend_graph.graph, blend_graph.root, 0);

                threaded_blend_graph.rebuild_target_subgraphs(&blend_graph.graph, &animation_clips);
                animation_target_graphs_rebuilt_this_frame.insert(id);

                // Write in the threaded graph.
                threaded_blend_graphs
                    .threaded_graphs
                    .insert(id, threaded_blend_graph);
            }

            AssetEvent::Removed { id } => {
                if let Some(threaded_blend_graph) =
                    threaded_blend_graphs.threaded_graphs.remove(&id)
                {
                    threaded_blend_graph.remove_from_clip_to_graphs_table(
                        id,
                        &mut threaded_blend_graphs.clip_to_graphs,
                    );
                }
            }
            AssetEvent::Unused { .. } => {}
        }
    }

    for animation_clip_asset_event in animation_clip_asset_events.read() {
        match *animation_clip_asset_event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                let Some(graph_ids) = threaded_blend_graphs.clip_to_graphs.get(&id) else {
                    continue;
                };
                for graph_id in graph_ids {
                    if animation_target_graphs_rebuilt_this_frame.insert(*graph_id)
                        && let Some(threaded_blend_graph) =
                            threaded_blend_graphs.threaded_graphs.get_mut(graph_id)
                        && let Some(blend_graph) = blend_graphs.get(*graph_id)
                    {
                        threaded_blend_graph
                            .rebuild_target_subgraphs(&blend_graph.graph, &animation_clips);
                    }
                }
            }
            AssetEvent::Unused { .. } | AssetEvent::Removed { .. } => {}
        }
    }
}

impl ThreadedBlendGraph {
    /// Removes all the data in this [`ThreadedBlendGraph`], keeping the
    /// memory around for later reuse.
    fn clear(&mut self) {
        self.threaded_graph.clear();
        self.sorted_edge_list_ranges.clear();
        self.sorted_edges.clear();
        self.animation_clips.clear();
        self.animation_target_to_threaded_subgraph.clear();
    }

    /// Prepares the [`ThreadedBlendGraph`] for recursion.
    fn init(&mut self, blend_graph: &BlendGraph) {
        let node_count = blend_graph.graph.node_count();
        let edge_count = blend_graph.graph.edge_count();

        self.threaded_graph.reserve(node_count);
        self.sorted_edges.reserve(edge_count);

        self.sorted_edge_list_ranges.clear();
        self.sorted_edge_list_ranges
            .extend(iter::repeat_n(0..0, node_count));

        self.computed_masks.clear();
        self.computed_masks.extend(iter::repeat_n(0, node_count));

        self.animation_clips.clear();
        for node in blend_graph.graph.node_weights() {
            match &node.node_type {
                BlendNodeType::Clip(clip_handle) => {
                    self.animation_clips.insert(clip_handle.id());
                }
                BlendNodeType::Blend | BlendNodeType::Add => {}
            }
        }
    }

    /// Recursively constructs the [`ThreadedBlendGraph`] for the subtree
    /// rooted at the given node.
    ///
    /// `mask` specifies the computed mask of the parent node. (It could be
    /// fetched from the [`Self::computed_masks`] field, but we pass it
    /// explicitly as a micro-optimization.)
    fn build_from(&mut self, graph: &BlendDiGraph, node_index: BlendNodeIndex, mut mask: u64) {
        // Accumulate the mask.
        mask |= graph.node_weight(node_index).unwrap().mask;
        self.computed_masks[node_index.index()] = mask;

        // Gather up the indices of our children, and sort them.
        let mut kids: SmallVec<[BlendNodeIndex; 8]> = graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .collect();
        kids.sort_unstable();

        // Write in the list of kids.
        self.sorted_edge_list_ranges[node_index.index()] =
            (self.sorted_edges.len() as u32)..((self.sorted_edges.len() + kids.len()) as u32);
        self.sorted_edges.extend_from_slice(&kids);

        // Recurse. (This is a postorder traversal.)
        for kid in kids.into_iter().rev() {
            self.build_from(graph, kid, mask);
        }

        // Finally, push our index.
        self.threaded_graph.push(node_index);
    }

    /// Creates subgraphs for each animation target consisting of only the nodes
    /// that affect that target.
    fn rebuild_target_subgraphs(&mut self, graph: &BlendDiGraph, clips: &Assets<AnimationClip>) {
        self.animation_target_to_threaded_subgraph.clear();

        // We traverse the graph looking for animation clips. Each animation
        // clip that we've processed is inserted into this list.
        let mut seen_animation_clips = HashSet::new();

        // Create a vector for each node that stores whether the node is
        // relevant to the target in question.
        let mut node_is_relevant: BitVec = iter::repeat_n(false, graph.node_count()).collect();

        // Search for animation clips. When we find one we haven't seen before,
        // create a target subgraph for it.
        for node in graph.node_weights() {
            let animation_clip = match &node.node_type {
                BlendNodeType::Clip(animation_clip_id) => {
                    if !seen_animation_clips.insert(animation_clip_id) {
                        continue;
                    }
                    let Some(animation_clip) = clips.get(animation_clip_id) else {
                        continue;
                    };
                    animation_clip
                }
                BlendNodeType::Blend | BlendNodeType::Add => continue,
            };

            for animation_target_id in animation_clip.curves().keys() {
                if self
                    .animation_target_to_threaded_subgraph
                    .contains_key(animation_target_id)
                {
                    continue;
                };

                let threaded_subgraph = self.create_target_subgraph(
                    *animation_target_id,
                    graph,
                    clips,
                    &mut node_is_relevant,
                );

                self.animation_target_to_threaded_subgraph
                    .insert(*animation_target_id, threaded_subgraph);
            }
        }
    }

    /// Creates a subgraph of the given blend graph that contains only nodes
    /// that affect the given `animation_target_id`.
    fn create_target_subgraph(
        &self,
        animation_target_id: AnimationTargetId,
        graph: &BlendDiGraph,
        clips: &Assets<AnimationClip>,
        node_is_relevant: &mut BitVec,
    ) -> ThreadedBlendSubgraph {
        node_is_relevant.fill(false);

        let mut threaded_subgraph = ThreadedBlendSubgraph::default();

        // We only need to do a single pass over the postorder traversal,
        // because a subset of a postorder traversal is also a postorder
        // traversal.
        for node_index in &self.threaded_graph {
            let Some(node) = graph.node_weight(*node_index) else {
                continue;
            };

            match &node.node_type {
                BlendNodeType::Clip(clip_handle) => {
                    // Only add this node if the clip has curves and/or events
                    // that affect this target.
                    if clips
                        .get(clip_handle)
                        .is_some_and(|clip| clip.is_relevant_to_target(animation_target_id))
                    {
                        threaded_subgraph.add_node(
                            *node_index,
                            &self.sorted_edge_list_ranges,
                            &self.sorted_edges,
                            node_is_relevant,
                        );
                    }
                }

                BlendNodeType::Add | BlendNodeType::Blend => {
                    // Add this node if any of its children are relevant.
                    let sorted_edge_range =
                        self.sorted_edge_list_ranges[node_index.index()].clone();
                    if sorted_edge_range.into_iter().any(|sorted_edge_index| {
                        node_is_relevant[self.sorted_edges[sorted_edge_index as usize].index()]
                    }) {
                        threaded_subgraph.add_node(
                            *node_index,
                            &self.sorted_edge_list_ranges,
                            &self.sorted_edges,
                            node_is_relevant,
                        );
                    }
                }
            }
        }

        threaded_subgraph
    }

    /// Creates the table that maps each animation clip to the blend graphs
    /// that contain that clip.
    ///
    /// Bevy uses this table to determine which subgraphs to invalidate when a
    /// clip is newly loaded or changed.
    fn populate_clip_to_graphs_table(
        &self,
        graph_id: AssetId<BlendGraph>,
        clip_to_graphs: &mut HashMap<AssetId<AnimationClip>, HashSet<AssetId<BlendGraph>>>,
    ) {
        for clip_id in &self.animation_clips {
            clip_to_graphs.entry(*clip_id).or_default().insert(graph_id);
        }
    }

    /// Removes the given blend graph from the table mapping clips to
    /// blend graphs.
    fn remove_from_clip_to_graphs_table(
        &self,
        graph_id: AssetId<BlendGraph>,
        clip_to_graphs: &mut HashMap<AssetId<AnimationClip>, HashSet<AssetId<BlendGraph>>>,
    ) {
        for clip_id in &self.animation_clips {
            let Entry::Occupied(mut graphs) = clip_to_graphs.entry(*clip_id) else {
                continue;
            };
            graphs.get_mut().remove(&graph_id);
            if graphs.get().is_empty() {
                graphs.remove();
            }
        }
    }
}

impl ThreadedBlendSubgraph {
    /// Copies a node from a threaded blend graph to its subgraph and adds
    /// the index of the node to the `node_is_relevant` table.
    ///
    /// `original_sorted_edge_list_ranges` and `original_sorted_edges` are
    /// expected to be the [`ThreadedBlendGraph::sorted_edge_list_ranges`]
    /// and [`ThreadedBlendGraph::sorted_edges`] fields from the containing
    /// threaded graph respectively.
    fn add_node(
        &mut self,
        node_index: NodeIndex<u32>,
        original_sorted_edge_list_ranges: &[Range<u32>],
        original_sorted_edges: &[NodeIndex<u32>],
        node_is_relevant: &mut BitVec,
    ) {
        node_is_relevant.set(node_index.index(), true);

        self.threaded_graph.push(node_index);

        // Copy over the edges.
        let sorted_edge_range_start = self.sorted_edges.len() as u32;
        let original_sorted_edge_list_range =
            original_sorted_edge_list_ranges[node_index.index()].clone();
        for original_sorted_edge_index in original_sorted_edge_list_range {
            let edge_dest = original_sorted_edges[original_sorted_edge_index as usize];
            // Make sure to only copy an edge if it points to a node that we
            // judged relevant.
            // Otherwise, `animate_targets` will spend a lot of time looking at
            // irrelevant nodes.
            // As we're traversing in postorder, we already visited `edge_dest`
            // by now, so we know whether it's relevant or not.
            if node_is_relevant[edge_dest.index()] {
                self.sorted_edges.push(edge_dest);
            }
        }
        self.sorted_edge_list_offsets.push(sorted_edge_range_start);
    }
}

#[cfg(test)]
mod tests {
    use std::array;

    use bevy_asset::Assets;
    use bevy_curve::{ConstantCurve, Interval};
    use bevy_ecs::name::Name;
    use bevy_math::Vec3;
    use bevy_transform::components::Transform;
    use itertools::Itertools;
    use petgraph::graph::NodeIndex;

    use crate::{
        animated_field,
        animation_curves::BlendableCurve,
        graph::{BlendGraph, ThreadedBlendGraph},
        AnimationClip, AnimationTargetId,
    };

    /// Tests that each target's subgraph only contains clips relevant to that
    /// target.
    #[test]
    fn subgraph_prunes_irrelevant_clips() {
        // Create a graph consisting of a root node connected to two clip nodes.
        let (mut graph, mut clips) = (BlendGraph::new(), Assets::<AnimationClip>::default());
        let target_ids_and_nodes = ["A", "B"].map(|target_name| {
            let target_id = AnimationTargetId::from_name(&Name::new(target_name));
            let clip = clips.add(create_animation_clip_for_target(target_id));
            let clip_node = graph.add_clip(clip, 1.0, graph.root);
            (target_id, clip_node)
        });

        // Create subgraphs.
        let threaded_graph = create_threaded_graph_from_blend_graph(&graph, &clips);

        // Check that there is one subgraph for each target, that there are no
        // other subgraphs, and that the subgraphs contain only the graph root
        // and the clip node.
        assert_eq!(
            threaded_graph.animation_target_to_threaded_subgraph.len(),
            2
        );
        for (target_id, clip_node) in target_ids_and_nodes {
            let subgraph = &threaded_graph.animation_target_to_threaded_subgraph[&target_id];
            assert_eq!(
                subgraph.threaded_graph.iter().sorted().collect::<Vec<_>>(),
                [graph.root, clip_node].iter().sorted().collect::<Vec<_>>()
            );
        }
    }

    /// Tests that each target's subgraph only contains blend nodes relevant to
    /// that target.
    #[test]
    fn subgraph_prunes_irrelevant_blend_nodes() {
        // Create a graph consisting of a root node connected to two blend
        // nodes, each of which is in turn connected to a clip node.
        let (mut graph, mut clips) = (BlendGraph::new(), Assets::<AnimationClip>::default());
        let target_ids_and_nodes = ["A", "B"].map(|target_name| {
            let target_id = AnimationTargetId::from_name(&Name::new(target_name));
            let blend_node = graph.add_blend(1.0, graph.root);
            let clip = clips.add(create_animation_clip_for_target(target_id));
            let clip_node = graph.add_clip(clip, 1.0, blend_node);
            (target_id, blend_node, clip_node)
        });

        // Create subgraphs.
        let threaded_graph = create_threaded_graph_from_blend_graph(&graph, &clips);

        // Check that there is one subgraph for each target, that there are no
        // other subgraphs, and that the subgraphs contain only the graph root,
        // the blend node, and the clip node.
        assert_eq!(
            threaded_graph.animation_target_to_threaded_subgraph.len(),
            2
        );
        for (target_id, blend_node, clip_node) in target_ids_and_nodes {
            let subgraph = &threaded_graph.animation_target_to_threaded_subgraph[&target_id];
            assert_eq!(
                subgraph.threaded_graph.iter().sorted().collect::<Vec<_>>(),
                [graph.root, blend_node, clip_node]
                    .iter()
                    .sorted()
                    .collect::<Vec<_>>()
            );
        }
    }

    /// Tests that each subgraph includes all clips animating a single animation
    /// target.
    #[test]
    fn subgraph_includes_all_clips_for_a_target() {
        // Create a graph consisting of one blend node that blends two clip
        // nodes.
        let (mut graph, mut clips) = (BlendGraph::new(), Assets::<AnimationClip>::default());
        let blend_node = graph.add_blend(1.0, graph.root);
        let target_id = AnimationTargetId::from_name(&Name::new("MyTarget"));
        let clip_nodes: [_; 2] = array::from_fn(|_| {
            let clip = clips.add(create_animation_clip_for_target(target_id));
            graph.add_clip(clip, 1.0, blend_node)
        });

        // Create subgraphs.
        let threaded_graph = create_threaded_graph_from_blend_graph(&graph, &clips);

        // Check that there is only one target and that that target's subgraph contains all the graph nodes.
        assert_eq!(
            threaded_graph.animation_target_to_threaded_subgraph.len(),
            1
        );
        let subgraph = &threaded_graph.animation_target_to_threaded_subgraph[&target_id];
        assert_eq!(
            subgraph.threaded_graph.iter().sorted().collect::<Vec<_>>(),
            [graph.root, blend_node, clip_nodes[0], clip_nodes[1]]
                .iter()
                .sorted()
                .collect::<Vec<_>>()
        );
    }

    /// Tests that we can create subgraphs with node indices in arbitrary order.
    #[test]
    fn subgraphs_are_correct_with_arbitrarily_ordered_nodes() {
        // Create a graph consisting of a root node connected to two blend
        // nodes, each of which is in turn connected to a clip node.
        // But create the blends before the clips.
        let (mut graph, mut clips) = (BlendGraph::new(), Assets::<AnimationClip>::default());
        let blend_nodes: [NodeIndex; 2] = array::from_fn(|_| graph.add_blend(1.0, graph.root));
        let target_ids_and_clip_nodes: [(AnimationTargetId, NodeIndex); 2] =
            array::from_fn(|index| {
                let target_id = AnimationTargetId::from_name(&Name::new(["A", "B"][index]));
                let clip = clips.add(create_animation_clip_for_target(target_id));
                let clip_node = graph.add_clip(clip, 1.0, blend_nodes[index]);
                (target_id, clip_node)
            });

        // Create subgraphs.
        let threaded_graph = create_threaded_graph_from_blend_graph(&graph, &clips);

        // Check that there is one subgraph for each target, that there are no
        // other subgraphs, and that the subgraphs contain only the graph root,
        // the blend node, and the clip node.
        assert_eq!(
            threaded_graph.animation_target_to_threaded_subgraph.len(),
            2
        );
        for (blend_node, (target_id, clip_node)) in
            blend_nodes.iter().zip(target_ids_and_clip_nodes.iter())
        {
            let subgraph = &threaded_graph.animation_target_to_threaded_subgraph[target_id];
            assert_eq!(
                subgraph.threaded_graph.iter().sorted().collect::<Vec<_>>(),
                [graph.root, *blend_node, *clip_node]
                    .iter()
                    .sorted()
                    .collect::<Vec<_>>()
            );
        }
    }

    /// Creates a threaded graph and all the target subgraphs from the given
    /// blend graph, just as `thread_blend_graphs` does.
    fn create_threaded_graph_from_blend_graph(
        blend_graph: &BlendGraph,
        animation_clips: &Assets<AnimationClip>,
    ) -> ThreadedBlendGraph {
        let mut threaded_blend_graph = ThreadedBlendGraph::default();
        threaded_blend_graph.init(blend_graph);
        threaded_blend_graph.build_from(&blend_graph.graph, blend_graph.root, 0);
        threaded_blend_graph.rebuild_target_subgraphs(&blend_graph.graph, animation_clips);
        threaded_blend_graph
    }

    /// Creates a simple animation clip animating the given target.
    fn create_animation_clip_for_target(animation_target_id: AnimationTargetId) -> AnimationClip {
        let mut animation_clip = AnimationClip::default();
        let blendable_curve = BlendableCurve::new(
            animated_field!(Transform::translation),
            ConstantCurve::new(Interval::EVERYWHERE, Vec3::ONE),
        );
        animation_clip.add_curve_to_target(animation_target_id, blendable_curve);
        animation_clip
    }
}
