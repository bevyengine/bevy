//! The animation graph, which allows animations to be blended together.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::ops::{Index, IndexMut};
use std::path::Path;

use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetId, AssetLoader, AssetPath, AsyncReadExt as _, Handle, LoadContext};
use bevy_reflect::Reflect;
use bevy_utils::{BoxedFuture, HashMap};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Dfs, Visitable};
use petgraph::Graph;
use ron::de::SpannedError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::AnimationClip;

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
/// There are two types of nodes: *blend nodes* and *clip nodes*, both of which
/// can have an associated weight. Blend nodes have no associated animation clip
/// and simply affect the weights of all their descendant nodes. Clip nodes
/// specify an animation clip to play. When a graph is created, it starts with
/// only a single blend node, the root node.
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
/// Animation graphs are assets and can be serialized to and loaded from [RON]
/// files. Canonically, such files have an `.animgraph.ron` extension.
///
/// The animation graph implements [RFC 51]. See that document for more
/// information.
///
/// [RON]: https://github.com/ron-rs/ron
///
/// [RFC 51]: https://github.com/bevyengine/rfcs/blob/main/rfcs/51-animation-composition.md
#[derive(Asset, Reflect, Clone, Debug, Serialize, Deserialize)]
#[serde(into = "SerializedAnimationGraph", from = "SerializedAnimationGraph")]
pub struct AnimationGraph {
    /// The `petgraph` data structure that defines the animation graph.
    pub graph: AnimationDiGraph,
    /// The index of the root node in the animation graph.
    pub root: NodeIndex,
}

/// A type alias for the `petgraph` data structure that defines the animation
/// graph.
pub type AnimationDiGraph = DiGraph<AnimationGraphNode, (), u32>;

/// The index of either an animation or blend node in the animation graph.
///
/// These indices are the way that [`crate::AnimationPlayer`]s identify
/// particular animations.
pub type AnimationNodeIndex = NodeIndex<u32>;

/// An individual node within an animation graph.
///
/// If `clip` is present, this is a *clip node*. Otherwise, it's a *blend node*.
/// Both clip and blend nodes can have weights, and those weights are propagated
/// down to descendants.
#[derive(Clone, Reflect, Debug, Serialize, Deserialize)]
pub struct AnimationGraphNode {
    /// The animation clip associated with this node, if any.
    ///
    /// If the clip is present, this node is an *animation clip node*.
    /// Otherwise, this node is a *blend node*.
    #[serde(serialize_with = "serialize_clip_handle")]
    #[serde(deserialize_with = "deserialize_clip_handle")]
    pub clip: Option<Handle<AnimationClip>>,

    /// The weight of this node.
    ///
    /// Weights are propagated down to descendants. Thus if an animation clip
    /// has weight 0.3 and its parent blend node has weight 0.6, the computed
    /// weight of the animation clip is 0.18.
    pub weight: f32,
}

/// An [`AssetLoader`] that can load [`AnimationGraph`]s as assets.
///
/// The canonical extension for [`AnimationGraph`]s is `.animgraph.ron`. Plain
/// `.animgraph` is supported as well.
#[derive(Default)]
pub struct AnimationGraphAssetLoader;

/// Various errors that can occur when serializing or deserializing animation
/// graphs to and from RON, respectively.
#[derive(Error, Debug)]
pub enum AnimationGraphLoadError {
    /// An I/O error occurred.
    #[error("I/O")]
    Io(#[from] io::Error),
    /// An error occurred in RON serialization or deserialization.
    #[error("RON serialization")]
    Ron(#[from] ron::Error),
    /// An error occurred in RON deserialization, and the location of the error
    /// is supplied.
    #[error("RON serialization")]
    SpannedRon(#[from] SpannedError),
}

/// A version of [`AnimationGraph`] suitable for serializing as an asset.
///
/// Animation nodes can refer to external animation clips, and the [`AssetId`]
/// is typically not sufficient to identify the clips, since the
/// [`bevy_asset::AssetServer`] assigns IDs in unpredictable ways. That fact
/// motivates this type, which pairs the [`AnimationGraph`] data with a list of
/// asset paths. Loading an animation graph via the [`bevy_asset::AssetServer`]
/// actually loads a serialized instance of this type, as does serializing an
/// [`AnimationGraph`] through `serde`.
#[derive(Serialize, Deserialize)]
pub struct SerializedAnimationGraph {
    /// Corresponds to the `graph` field on [`AnimationGraph`].
    pub graph: AnimationDiGraph,
    /// Corresponds to the `root` field on [`AnimationGraph`].
    pub root: NodeIndex,
    /// A mapping from the animation graph's asset IDs to their paths.
    pub asset_paths: HashMap<AssetId<AnimationClip>, AssetPath<'static>>,
}

impl AnimationGraph {
    /// Creates a new animation graph with a root node and no other nodes.
    pub fn new() -> Self {
        let mut graph = DiGraph::default();
        let root = graph.add_node(AnimationGraphNode::default());
        Self { graph, root }
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

    /// Adds an [`AnimationClip`] to the animation graph with the given weight
    /// and returns its index.
    ///
    /// The animation clip will be the child of the given parent.
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
        <I as std::iter::IntoIterator>::IntoIter: 'a,
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
    /// weights multiplied by the weight of the blend.
    pub fn add_blend(&mut self, weight: f32, parent: AnimationNodeIndex) -> AnimationNodeIndex {
        let node_index = self
            .graph
            .add_node(AnimationGraphNode { clip: None, weight });
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

    /// Performs a depth-first search on the animation graph.
    pub(crate) fn dfs(
        &self,
    ) -> Dfs<AnimationNodeIndex, <Graph<AnimationGraphNode, ()> as Visitable>::Map> {
        Dfs::new(&self.graph, self.root)
    }

    /// Serializes the animation graph to the given [`Write`]r in RON format.
    pub fn save<W>(&self, writer: &mut W) -> Result<(), AnimationGraphLoadError>
    where
        W: Write,
    {
        let mut ron_serializer = ron::ser::Serializer::new(writer, None)?;
        Ok(self.serialize(&mut ron_serializer)?)
    }

    /// A convenience method to serialize the animation graph to a file.
    ///
    /// This file can later be loaded with the [`AnimationGraphAssetLoader`] to
    /// reconstruct the graph.
    pub fn save_to<P>(&self, path: &P) -> Result<(), AnimationGraphLoadError>
    where
        P: AsRef<Path>,
    {
        let mut writer = BufWriter::new(File::create(path)?);
        self.save(&mut writer)
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

impl AssetLoader for AnimationGraphAssetLoader {
    type Asset = AnimationGraph;

    type Settings = ();

    type Error = AnimationGraphLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            // Deserialize a `SerializedAnimationGraph` directly, so that we can
            // get the list of the animation clips it refers to and load them.
            let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
            let mut serialized_animation_graph =
                SerializedAnimationGraph::deserialize(&mut deserializer)
                    .map_err(|err| deserializer.span_error(err))?;

            // If the animation graph refers to assets on disk (the common
            // case), then load them.
            if !serialized_animation_graph.asset_paths.is_empty() {
                for node in serialized_animation_graph.graph.node_weights_mut() {
                    let Some(ref mut clip_handle) = node.clip else {
                        continue;
                    };
                    let Some(asset_path) = serialized_animation_graph
                        .asset_paths
                        .get(&clip_handle.id())
                    else {
                        continue;
                    };
                    *clip_handle = load_context.load(asset_path);
                }
            }

            // Now that we've fixed up all handles, convert to an
            // `AnimationGraph`.
            Ok(serialized_animation_graph.into())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["animgraph", "animgraph.ron"]
    }
}

// This is just a hack to allow `Handle<AnimationClip>` to be serialized. We
// could use the `TypedReflectSerializer` for this, but that would require a
// `TypeRegistry` handle, which Serde doesn't have. We opt to use Serde for
// serialization and deserialization of animation graphs because implementing
// reflection support for `petgraph` graphs would be burdensome.
fn serialize_clip_handle<S>(
    clip: &Option<Handle<AnimationClip>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    clip.as_ref().map(|clip| clip.id()).serialize(serializer)
}

// This is just a hack to allow `Handle<AnimationClip>` to be deserialized. We
// could use the `TypedReflectDeserializer` for this, but that would require a
// `TypeRegistry` handle, which Serde doesn't have. We opt to use Serde for
// serialization and deserialization of animation graphs because implementing
// reflection support for `petgraph` graphs would be burdensome.
fn deserialize_clip_handle<'de, D>(
    deserializer: D,
) -> Result<Option<Handle<AnimationClip>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        <Option<AssetId<AnimationClip>> as Deserialize>::deserialize(deserializer)?
            .map(Handle::Weak),
    )
}

impl From<AnimationGraph> for SerializedAnimationGraph {
    fn from(animation_graph: AnimationGraph) -> Self {
        // Before serializing, gather up all animation clips with paths that the
        // graph refers to, and write that path information alongside the graph.
        let asset_paths = animation_graph
            .graph
            .node_weights()
            .filter_map(|node| node.clip.as_ref())
            .filter_map(|clip_handle| {
                clip_handle
                    .path()
                    .map(|asset_path| (clip_handle.id(), asset_path.clone()))
            })
            .collect();

        // Transfer the fields over.
        Self {
            graph: animation_graph.graph,
            root: animation_graph.root,
            asset_paths,
        }
    }
}

impl From<SerializedAnimationGraph> for AnimationGraph {
    fn from(animation_graph: SerializedAnimationGraph) -> Self {
        // Simply discard the asset paths metadata. Since this is expected to
        // be called after the asset loader has already loaded all asset paths,
        // this is OK.
        Self {
            graph: animation_graph.graph,
            root: animation_graph.root,
        }
    }
}
