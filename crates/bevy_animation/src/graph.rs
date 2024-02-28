//! The animation graph.

use std::any::TypeId;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::ops::{Index, IndexMut};
use std::path::Path;

use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetId, AssetLoader, AsyncReadExt as _, Handle, LoadContext};
use bevy_reflect::serde::{TypedReflectDeserializer, TypedReflectSerializer};
use bevy_reflect::{Reflect, TypeRegistry, TypeRegistryArc};
use bevy_utils::BoxedFuture;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Dfs, Visitable};
use petgraph::Graph;
use ron::de::SpannedError;
use serde::de::DeserializeSeed;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::AnimationClip;

#[derive(Asset, Reflect, Debug, Serialize, Deserialize)]
pub struct AnimationGraph {
    pub graph: AnimationDiGraph,
    pub root: NodeIndex,
}

pub type AnimationDiGraph = DiGraph<AnimationGraphNode, (), u32>;

pub type AnimationNodeIndex = NodeIndex<u32>;

#[derive(Clone, Reflect, Debug, Serialize, Deserialize)]
pub struct AnimationGraphNode {
    #[serde(serialize_with = "serialize_clip_handle")]
    #[serde(deserialize_with = "deserialize_clip_handle")]
    pub clip: Option<Handle<AnimationClip>>,
    pub weight: f32,
}

pub struct AnimationGraphAssetLoader;

#[derive(Error, Debug)]
pub enum AnimationGraphLoadError {
    #[error("I/O")]
    Io(#[from] io::Error),
    #[error("RON serialization")]
    Ron(#[from] ron::Error),
    #[error("RON serialization")]
    SpannedRon(#[from] SpannedError),
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

    pub fn save<W>(&self, writer: &mut W) -> Result<(), AnimationGraphLoadError>
    where
        W: Write,
    {
        let mut ron_serializer = ron::ser::Serializer::new(writer, None)?;
        Ok(self.serialize(&mut ron_serializer)?)
    }

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
        _: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
            AnimationGraph::deserialize(&mut deserializer)
                .map_err(|err| deserializer.span_error(err).into())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["animgraph", "animgraph.ron"]
    }
}

fn serialize_clip_handle<S>(
    clip: &Option<Handle<AnimationClip>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    clip.as_ref().map(|clip| clip.id()).serialize(serializer)
}

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
