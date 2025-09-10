//! The animation graph, which allows animations to be blended together.

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
    system::{Res, ResMut},
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use derive_more::derive::From;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    Direction,
};
use ron::de::SpannedError;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;
use tracing::warn;

use crate::{AnimationClip, AnimationTargetId};

/// A graph structure that describes how animation clips are to be blended
/// together.
///
/// Applications frequently want to be able to play multiple animations at once
/// and to fine-tune the influence that animations have on a skinned mesh. Bevy
/// uses an *animation graph* to store this information. Animation graphs are a
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
/// the context of an animation graph.
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
/// Animation graphs are assets and can be serialized to and loaded from [RON]
/// files. Canonically, such files have an `.animgraph.ron` extension.
///
/// The animation graph implements [RFC 51]. See that document for more
/// information.
///
/// [RON]: https://github.com/ron-rs/ron
///
/// [RFC 51]: https://github.com/bevyengine/rfcs/blob/main/rfcs/51-animation-composition.md
#[derive(Asset, Reflect, Clone, Debug)]
#[reflect(Debug, Clone)]
pub struct AnimationGraph {
    /// The `petgraph` data structure that defines the animation graph.
    pub graph: AnimationDiGraph,

    /// The index of the root node in the animation graph.
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

/// A [`Handle`] to the [`AnimationGraph`] to be used by the [`AnimationPlayer`](crate::AnimationPlayer) on the same entity.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Clone)]
pub struct AnimationGraphHandle(pub Handle<AnimationGraph>);

impl From<AnimationGraphHandle> for AssetId<AnimationGraph> {
    fn from(handle: AnimationGraphHandle) -> Self {
        handle.id()
    }
}

impl From<&AnimationGraphHandle> for AssetId<AnimationGraph> {
    fn from(handle: &AnimationGraphHandle) -> Self {
        handle.id()
    }
}

/// A type alias for the `petgraph` data structure that defines the animation
/// graph.
pub type AnimationDiGraph = DiGraph<AnimationGraphNode, (), u32>;

/// The index of either an animation or blend node in the animation graph.
///
/// These indices are the way that [animation players] identify each animation.
///
/// [animation players]: crate::AnimationPlayer
pub type AnimationNodeIndex = NodeIndex<u32>;

/// An individual node within an animation graph.
///
/// The [`AnimationGraphNode::node_type`] field specifies the type of node: one
/// of a *clip node*, a *blend node*, or an *add node*. Clip nodes, the leaves
/// of the graph, contain animation clips to play. Blend and add nodes describe
/// how to combine their children to produce a final animation.
#[derive(Clone, Reflect, Debug)]
#[reflect(Clone)]
pub struct AnimationGraphNode {
    /// Animation node data specific to the type of node (clip, blend, or add).
    ///
    /// In the case of clip nodes, this contains the actual animation clip
    /// associated with the node.
    pub node_type: AnimationNodeType,

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
    /// [Blend]: AnimationNodeType::Blend
    /// [Add]: AnimationNodeType::Add
    /// [active animation weight]: crate::ActiveAnimation::weight
    pub weight: f32,
}

/// Animation node data specific to the type of node (clip, blend, or add).
///
/// In the case of clip nodes, this contains the actual animation clip
/// associated with the node.
#[derive(Clone, Default, Reflect, Debug)]
#[reflect(Clone)]
pub enum AnimationNodeType {
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

/// An [`AssetLoader`] that can load [`AnimationGraph`]s as assets.
///
/// The canonical extension for [`AnimationGraph`]s is `.animgraph.ron`. Plain
/// `.animgraph` is supported as well.
#[derive(Default)]
pub struct AnimationGraphAssetLoader;

/// Errors that can occur when serializing animation graphs to RON.
#[derive(Error, Debug)]
pub enum AnimationGraphSaveError {
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

/// Errors that can occur when deserializing animation graphs from RON.
#[derive(Error, Debug)]
pub enum AnimationGraphLoadError {
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
        "The deserialized AnimationGraph contained an AnimationClip referenced by an AssetId, \
    which is no longer supported. Consider manually deserializing the SerializedAnimationGraph \
    type and determine how to migrate any SerializedAnimationClip::AssetId animation clips"
    )]
    GraphContainsLegacyAssetId,
}

/// Acceleration structures for animation graphs that allows Bevy to evaluate
/// them quickly.
///
/// These are kept up to date as [`AnimationGraph`] instances are added,
/// modified, and removed.
#[derive(Default, Reflect, Resource)]
pub struct ThreadedAnimationGraphs(
    pub(crate) HashMap<AssetId<AnimationGraph>, ThreadedAnimationGraph>,
);

/// An acceleration structure for an animation graph that allows Bevy to
/// evaluate it quickly.
///
/// This is kept up to date as the associated [`AnimationGraph`] instance is
/// added, modified, or removed.
#[derive(Default, Reflect)]
pub struct ThreadedAnimationGraph {
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
    pub threaded_graph: Vec<AnimationNodeIndex>,

    /// A mapping from each parent node index to the range within
    /// [`Self::sorted_edges`].
    ///
    /// This allows for quick lookup of the children of each node, sorted in
    /// ascending order of node index, without having to sort the result of the
    /// `petgraph` traversal functions every frame.
    pub sorted_edge_ranges: Vec<Range<u32>>,

    /// A list of the children of each node, sorted in ascending order.
    pub sorted_edges: Vec<AnimationNodeIndex>,

    /// A mapping from node index to a bitfield specifying the mask groups that
    /// this node masks *out* (i.e. doesn't animate).
    ///
    /// A 1 in bit position N indicates that this node doesn't animate any
    /// targets of mask group N.
    pub computed_masks: Vec<u64>,
}

/// A version of [`AnimationGraph`] suitable for serializing as an asset.
///
/// Animation nodes can refer to external animation clips, and the [`AssetId`]
/// is typically not sufficient to identify the clips, since the
/// [`bevy_asset::AssetServer`] assigns IDs in unpredictable ways. That fact
/// motivates this type, which replaces the `Handle<AnimationClip>` with an
/// asset path.  Loading an animation graph via the [`bevy_asset::AssetServer`]
/// actually loads a serialized instance of this type, as does serializing an
/// [`AnimationGraph`] through `serde`.
#[derive(Serialize, Deserialize)]
pub struct SerializedAnimationGraph {
    /// Corresponds to the `graph` field on [`AnimationGraph`].
    pub graph: DiGraph<SerializedAnimationGraphNode, (), u32>,
    /// Corresponds to the `root` field on [`AnimationGraph`].
    pub root: NodeIndex,
    /// Corresponds to the `mask_groups` field on [`AnimationGraph`].
    pub mask_groups: HashMap<AnimationTargetId, AnimationMask>,
}

/// A version of [`AnimationGraphNode`] suitable for serializing as an asset.
///
/// See the comments in [`SerializedAnimationGraph`] for more information.
#[derive(Serialize, Deserialize)]
pub struct SerializedAnimationGraphNode {
    /// Corresponds to the `node_type` field on [`AnimationGraphNode`].
    pub node_type: SerializedAnimationNodeType,
    /// Corresponds to the `mask` field on [`AnimationGraphNode`].
    pub mask: AnimationMask,
    /// Corresponds to the `weight` field on [`AnimationGraphNode`].
    pub weight: f32,
}

/// A version of [`AnimationNodeType`] suitable for serializing as part of a
/// [`SerializedAnimationGraphNode`] asset.
#[derive(Serialize, Deserialize)]
pub enum SerializedAnimationNodeType {
    /// Corresponds to [`AnimationNodeType::Clip`].
    Clip(MigrationSerializedAnimationClip),
    /// Corresponds to [`AnimationNodeType::Blend`].
    Blend,
    /// Corresponds to [`AnimationNodeType::Add`].
    Add,
}

/// A type to facilitate migration from the legacy format of [`SerializedAnimationGraph`] to the
/// new format.
///
/// By using untagged serde deserialization, we can try to deserialize the modern form, then
/// fallback to the legacy form. Users must migrate to the modern form by Bevy 0.18.
// TODO: Delete this after Bevy 0.17.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MigrationSerializedAnimationClip {
    /// This is the new type of this field.
    Modern(AssetPath<'static>),
    /// This is the legacy type of this field. Users must migrate away from this.
    #[serde(skip_serializing)]
    Legacy(SerializedAnimationClip),
}

/// The legacy form of serialized animation clips. This allows raw asset IDs to be deserialized.
// TODO: Delete this after Bevy 0.17.
#[derive(Deserialize)]
pub enum SerializedAnimationClip {
    /// Records an asset path.
    AssetPath(AssetPath<'static>),
    /// The fallback that records an asset ID.
    ///
    /// Because asset IDs can change, this should not be relied upon. Prefer to
    /// use asset paths where possible.
    AssetId(AssetId<AnimationClip>),
}

/// The type of an animation mask bitfield.
///
/// Bit N corresponds to mask group N.
///
/// Because this is a 64-bit value, there is currently a limitation of 64 mask
/// groups per animation graph.
pub type AnimationMask = u64;

impl AnimationGraph {
    /// Creates a new animation graph with a root node and no other nodes.
    pub fn new() -> Self {
        let mut graph = DiGraph::default();
        let root = graph.add_node(AnimationGraphNode::default());
        Self {
            graph,
            root,
            mask_groups: HashMap::default(),
        }
    }

    /// A convenience function for creating an [`AnimationGraph`] from a single
    /// [`AnimationClip`].
    ///
    /// The clip will be a direct child of the root with weight 1.0. Both the
    /// graph and the index of the added node are returned as a tuple.
    pub fn from_clip(clip: Handle<AnimationClip>) -> (Self, AnimationNodeIndex) {
        let mut graph = Self::new();
        let node_index = graph.add_clip(clip, 1.0, graph.root);
        (graph, node_index)
    }

    /// A convenience method to create an [`AnimationGraph`]s with an iterator
    /// of clips.
    ///
    /// All of the animation clips will be direct children of the root with
    /// weight 1.0.
    ///
    /// Returns the graph and indices of the new nodes.
    pub fn from_clips<'a, I>(clips: I) -> (Self, Vec<AnimationNodeIndex>)
    where
        I: IntoIterator<Item = Handle<AnimationClip>>,
        <I as IntoIterator>::IntoIter: 'a,
    {
        let mut graph = Self::new();
        let indices = graph.add_clips(clips, 1.0, graph.root).collect();
        (graph, indices)
    }

    /// Adds an [`AnimationClip`] to the animation graph with the given weight
    /// and returns its index.
    ///
    /// The animation clip will be the child of the given parent. The resulting
    /// node will have no mask.
    pub fn add_clip(
        &mut self,
        clip: Handle<AnimationClip>,
        weight: f32,
        parent: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            node_type: AnimationNodeType::Clip(clip),
            mask: 0,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds an [`AnimationClip`] to the animation graph with the given weight
    /// and mask, and returns its index.
    ///
    /// The animation clip will be the child of the given parent.
    pub fn add_clip_with_mask(
        &mut self,
        clip: Handle<AnimationClip>,
        mask: AnimationMask,
        weight: f32,
        parent: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            node_type: AnimationNodeType::Clip(clip),
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
        parent: AnimationNodeIndex,
    ) -> impl Iterator<Item = AnimationNodeIndex> + 'a
    where
        I: IntoIterator<Item = Handle<AnimationClip>>,
        <I as IntoIterator>::IntoIter: 'a,
    {
        clips
            .into_iter()
            .map(move |clip| self.add_clip(clip, weight, parent))
    }

    /// Adds a blend node to the animation graph with the given weight and
    /// returns its index.
    ///
    /// The blend node will be placed under the supplied `parent` node. During
    /// animation evaluation, the descendants of this blend node will have their
    /// weights multiplied by the weight of the blend. The blend node will have
    /// no mask.
    pub fn add_blend(&mut self, weight: f32, parent: AnimationNodeIndex) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            node_type: AnimationNodeType::Blend,
            mask: 0,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds a blend node to the animation graph with the given weight and
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
        parent: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            node_type: AnimationNodeType::Blend,
            mask,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds a blend node to the animation graph with the given weight and
    /// returns its index.
    ///
    /// The blend node will be placed under the supplied `parent` node. During
    /// animation evaluation, the descendants of this blend node will have their
    /// weights multiplied by the weight of the blend. The blend node will have
    /// no mask.
    pub fn add_additive_blend(
        &mut self,
        weight: f32,
        parent: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            node_type: AnimationNodeType::Add,
            mask: 0,
            weight,
        });
        self.graph.add_edge(parent, node_index, ());
        node_index
    }

    /// Adds a blend node to the animation graph with the given weight and
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
        parent: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let node_index = self.graph.add_node(AnimationGraphNode {
            node_type: AnimationNodeType::Add,
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

    /// Returns the [`AnimationGraphNode`] associated with the given index.
    ///
    /// If no node with the given index exists, returns `None`.
    pub fn get(&self, animation: AnimationNodeIndex) -> Option<&AnimationGraphNode> {
        self.graph.node_weight(animation)
    }

    /// Returns a mutable reference to the [`AnimationGraphNode`] associated
    /// with the given index.
    ///
    /// If no node with the given index exists, returns `None`.
    pub fn get_mut(&mut self, animation: AnimationNodeIndex) -> Option<&mut AnimationGraphNode> {
        self.graph.node_weight_mut(animation)
    }

    /// Returns an iterator over the [`AnimationGraphNode`]s in this graph.
    pub fn nodes(&self) -> impl Iterator<Item = AnimationNodeIndex> {
        self.graph.node_indices()
    }

    /// Serializes the animation graph to the given [`Write`]r in RON format.
    ///
    /// If writing to a file, it can later be loaded with the
    /// [`AnimationGraphAssetLoader`] to reconstruct the graph.
    pub fn save<W>(&self, writer: &mut W) -> Result<(), AnimationGraphSaveError>
    where
        W: Write,
    {
        let mut ron_serializer = ron::ser::Serializer::new(writer, None)?;
        let serialized_graph: SerializedAnimationGraph = self.clone().try_into()?;
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

impl AnimationGraphNode {
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
            node_type: Default::default(),
            mask: 0,
            weight: 1.0,
        }
    }
}

impl Default for AnimationGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for AnimationGraphAssetLoader {
    type Asset = AnimationGraph;

    type Settings = ();

    type Error = AnimationGraphLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Deserialize a `SerializedAnimationGraph` directly, so that we can
        // get the list of the animation clips it refers to and load them.
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let serialized_animation_graph = SerializedAnimationGraph::deserialize(&mut deserializer)
            .map_err(|err| deserializer.span_error(err))?;

        // Load all `AssetPath`s to convert from a `SerializedAnimationGraph` to a real
        // `AnimationGraph`. This is effectively a `DiGraph::map`, but this allows us to return
        // errors.
        let mut animation_graph = DiGraph::with_capacity(
            serialized_animation_graph.graph.node_count(),
            serialized_animation_graph.graph.edge_count(),
        );

        let mut already_warned = false;
        for serialized_node in serialized_animation_graph.graph.node_weights() {
            animation_graph.add_node(AnimationGraphNode {
                node_type: match serialized_node.node_type {
                    SerializedAnimationNodeType::Clip(ref clip) => match clip {
                        MigrationSerializedAnimationClip::Modern(path) => {
                            AnimationNodeType::Clip(load_context.load(path.clone()))
                        }
                        MigrationSerializedAnimationClip::Legacy(
                            SerializedAnimationClip::AssetPath(path),
                        ) => {
                            if !already_warned {
                                let path = load_context.asset_path();
                                warn!(
                                    "Loaded an AnimationGraph asset at \"{path}\" which contains a \
                                    legacy-style SerializedAnimationClip. Please re-save the asset \
                                    using AnimationGraph::save to automatically migrate to the new \
                                    format"
                                );
                                already_warned = true;
                            }
                            AnimationNodeType::Clip(load_context.load(path.clone()))
                        }
                        MigrationSerializedAnimationClip::Legacy(
                            SerializedAnimationClip::AssetId(_),
                        ) => {
                            return Err(AnimationGraphLoadError::GraphContainsLegacyAssetId);
                        }
                    },
                    SerializedAnimationNodeType::Blend => AnimationNodeType::Blend,
                    SerializedAnimationNodeType::Add => AnimationNodeType::Add,
                },
                mask: serialized_node.mask,
                weight: serialized_node.weight,
            });
        }
        for edge in serialized_animation_graph.graph.raw_edges() {
            animation_graph.add_edge(edge.source(), edge.target(), ());
        }
        Ok(AnimationGraph {
            graph: animation_graph,
            root: serialized_animation_graph.root,
            mask_groups: serialized_animation_graph.mask_groups,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["animgraph", "animgraph.ron"]
    }
}

impl TryFrom<AnimationGraph> for SerializedAnimationGraph {
    type Error = NonPathHandleError;

    fn try_from(animation_graph: AnimationGraph) -> Result<Self, NonPathHandleError> {
        // Convert all the `Handle<AnimationClip>` to AssetPath, so that
        // `AnimationGraphAssetLoader` can load them. This is effectively just doing a
        // `DiGraph::map`, except we need to return an error if any handles aren't associated to a
        // path.
        let mut serialized_graph = DiGraph::with_capacity(
            animation_graph.graph.node_count(),
            animation_graph.graph.edge_count(),
        );
        for node in animation_graph.graph.node_weights() {
            serialized_graph.add_node(SerializedAnimationGraphNode {
                weight: node.weight,
                mask: node.mask,
                node_type: match node.node_type {
                    AnimationNodeType::Clip(ref clip) => match clip.path() {
                        Some(path) => SerializedAnimationNodeType::Clip(
                            MigrationSerializedAnimationClip::Modern(path.clone()),
                        ),
                        None => return Err(NonPathHandleError),
                    },
                    AnimationNodeType::Blend => SerializedAnimationNodeType::Blend,
                    AnimationNodeType::Add => SerializedAnimationNodeType::Add,
                },
            });
        }
        for edge in animation_graph.graph.raw_edges() {
            serialized_graph.add_edge(edge.source(), edge.target(), ());
        }
        Ok(Self {
            graph: serialized_graph,
            root: animation_graph.root,
            mask_groups: animation_graph.mask_groups,
        })
    }
}

/// Error for when only path [`Handle`]s are supported.
#[derive(Error, Debug)]
#[error("AnimationGraph contains a handle to an AnimationClip that does not correspond to an asset path")]
pub struct NonPathHandleError;

/// A system that creates, updates, and removes [`ThreadedAnimationGraph`]
/// structures for every changed [`AnimationGraph`].
///
/// The [`ThreadedAnimationGraph`] contains acceleration structures that allow
/// for quick evaluation of that graph's animations.
pub(crate) fn thread_animation_graphs(
    mut threaded_animation_graphs: ResMut<ThreadedAnimationGraphs>,
    animation_graphs: Res<Assets<AnimationGraph>>,
    mut animation_graph_asset_events: MessageReader<AssetEvent<AnimationGraph>>,
) {
    for animation_graph_asset_event in animation_graph_asset_events.read() {
        match *animation_graph_asset_event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                // Fetch the animation graph.
                let Some(animation_graph) = animation_graphs.get(id) else {
                    continue;
                };

                // Reuse the allocation if possible.
                let mut threaded_animation_graph =
                    threaded_animation_graphs.0.remove(&id).unwrap_or_default();
                threaded_animation_graph.clear();

                // Recursively thread the graph in postorder.
                threaded_animation_graph.init(animation_graph);
                threaded_animation_graph.build_from(
                    &animation_graph.graph,
                    animation_graph.root,
                    0,
                );

                // Write in the threaded graph.
                threaded_animation_graphs
                    .0
                    .insert(id, threaded_animation_graph);
            }

            AssetEvent::Removed { id } => {
                threaded_animation_graphs.0.remove(&id);
            }
            AssetEvent::Unused { .. } => {}
        }
    }
}

impl ThreadedAnimationGraph {
    /// Removes all the data in this [`ThreadedAnimationGraph`], keeping the
    /// memory around for later reuse.
    fn clear(&mut self) {
        self.threaded_graph.clear();
        self.sorted_edge_ranges.clear();
        self.sorted_edges.clear();
    }

    /// Prepares the [`ThreadedAnimationGraph`] for recursion.
    fn init(&mut self, animation_graph: &AnimationGraph) {
        let node_count = animation_graph.graph.node_count();
        let edge_count = animation_graph.graph.edge_count();

        self.threaded_graph.reserve(node_count);
        self.sorted_edges.reserve(edge_count);

        self.sorted_edge_ranges.clear();
        self.sorted_edge_ranges
            .extend(iter::repeat_n(0..0, node_count));

        self.computed_masks.clear();
        self.computed_masks.extend(iter::repeat_n(0, node_count));
    }

    /// Recursively constructs the [`ThreadedAnimationGraph`] for the subtree
    /// rooted at the given node.
    ///
    /// `mask` specifies the computed mask of the parent node. (It could be
    /// fetched from the [`Self::computed_masks`] field, but we pass it
    /// explicitly as a micro-optimization.)
    fn build_from(
        &mut self,
        graph: &AnimationDiGraph,
        node_index: AnimationNodeIndex,
        mut mask: u64,
    ) {
        // Accumulate the mask.
        mask |= graph.node_weight(node_index).unwrap().mask;
        self.computed_masks[node_index.index()] = mask;

        // Gather up the indices of our children, and sort them.
        let mut kids: SmallVec<[AnimationNodeIndex; 8]> = graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .collect();
        kids.sort_unstable();

        // Write in the list of kids.
        self.sorted_edge_ranges[node_index.index()] =
            (self.sorted_edges.len() as u32)..((self.sorted_edges.len() + kids.len()) as u32);
        self.sorted_edges.extend_from_slice(&kids);

        // Recurse. (This is a postorder traversal.)
        for kid in kids.into_iter().rev() {
            self.build_from(graph, kid, mask);
        }

        // Finally, push our index.
        self.threaded_graph.push(node_index);
    }
}
