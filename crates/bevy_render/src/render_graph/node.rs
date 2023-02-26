use crate::{
    define_atomic_id,
    render_graph::{
        Edge, InputSlotError, OutputSlotError, RenderGraphContext, RenderGraphError,
        RunSubGraphError, SlotInfo, SlotInfos, SlotType, SlotValue,
    },
    renderer::RenderContext,
};
use bevy_ecs::world::World;
use downcast_rs::{impl_downcast, Downcast};
use std::{borrow::Cow, fmt::Debug};
use thiserror::Error;

define_atomic_id!(NodeId);

/// A render node that can be added to a [`RenderGraph`](super::RenderGraph).
///
/// Nodes are the fundamental part of the graph and used to extend its functionality, by
/// generating draw calls and/or running subgraphs.
/// They are added via the `render_graph::add_node(my_node)` method.
///
/// To determine their position in the graph and ensure that all required dependencies (inputs)
/// are already executed, [`Edges`](Edge) are used.
///
/// A node can produce outputs used as dependencies by other nodes.
/// Those inputs and outputs are called slots and are the default way of passing render data
/// inside the graph. For more information see [`SlotType`](super::SlotType).
pub trait Node: Downcast + Send + Sync + 'static {
    /// Specifies the required input slots for this node.
    /// They will then be available during the run method inside the [`RenderGraphContext`].
    fn input(&self) -> Vec<SlotInfo> {
        Vec::new()
    }

    /// Specifies the produced output slots for this node.
    /// They can then be passed one inside [`RenderGraphContext`] during the run method.
    fn output(&self) -> Vec<SlotInfo> {
        Vec::new()
    }

    /// Updates internal node state using the current render [`World`] prior to the run method.
    fn update(&mut self, _world: &mut World) {}

    /// Runs the graph node logic, issues draw calls, updates the output slots and
    /// optionally queues up subgraphs for execution. The graph data, input and output values are
    /// passed via the [`RenderGraphContext`].
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError>;
}

impl_downcast!(Node);

#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeRunError {
    #[error("encountered an input slot error")]
    InputSlotError(#[from] InputSlotError),
    #[error("encountered an output slot error")]
    OutputSlotError(#[from] OutputSlotError),
    #[error("encountered an error when running a sub-graph")]
    RunSubGraphError(#[from] RunSubGraphError),
}

/// A collection of input and output [`Edges`](Edge) for a [`Node`].
#[derive(Debug)]
pub struct Edges {
    id: NodeId,
    input_edges: Vec<Edge>,
    output_edges: Vec<Edge>,
}

impl Edges {
    /// Returns all "input edges" (edges going "in") for this node .
    #[inline]
    pub fn input_edges(&self) -> &[Edge] {
        &self.input_edges
    }

    /// Returns all "output edges" (edges going "out") for this node .
    #[inline]
    pub fn output_edges(&self) -> &[Edge] {
        &self.output_edges
    }

    /// Returns this node's id.
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Adds an edge to the `input_edges` if it does not already exist.
    pub(crate) fn add_input_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if self.has_input_edge(&edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge));
        }
        self.input_edges.push(edge);
        Ok(())
    }

    /// Removes an edge from the `input_edges` if it exists.
    pub(crate) fn remove_input_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if let Some(index) = self.input_edges.iter().position(|e| *e == edge) {
            self.input_edges.swap_remove(index);
            Ok(())
        } else {
            Err(RenderGraphError::EdgeDoesNotExist(edge))
        }
    }

    /// Adds an edge to the `output_edges` if it does not already exist.
    pub(crate) fn add_output_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if self.has_output_edge(&edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge));
        }
        self.output_edges.push(edge);
        Ok(())
    }

    /// Removes an edge from the `output_edges` if it exists.
    pub(crate) fn remove_output_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if let Some(index) = self.output_edges.iter().position(|e| *e == edge) {
            self.output_edges.swap_remove(index);
            Ok(())
        } else {
            Err(RenderGraphError::EdgeDoesNotExist(edge))
        }
    }

    /// Checks whether the input edge already exists.
    pub fn has_input_edge(&self, edge: &Edge) -> bool {
        self.input_edges.contains(edge)
    }

    /// Checks whether the output edge already exists.
    pub fn has_output_edge(&self, edge: &Edge) -> bool {
        self.output_edges.contains(edge)
    }

    /// Searches the `input_edges` for a [`Edge::SlotEdge`],
    /// which `input_index` matches the `index`;
    pub fn get_input_slot_edge(&self, index: usize) -> Result<&Edge, RenderGraphError> {
        self.input_edges
            .iter()
            .find(|e| {
                if let Edge::SlotEdge { input_index, .. } = e {
                    *input_index == index
                } else {
                    false
                }
            })
            .ok_or(RenderGraphError::UnconnectedNodeInputSlot {
                input_slot: index,
                node: self.id,
            })
    }

    /// Searches the `output_edges` for a [`Edge::SlotEdge`],
    /// which `output_index` matches the `index`;
    pub fn get_output_slot_edge(&self, index: usize) -> Result<&Edge, RenderGraphError> {
        self.output_edges
            .iter()
            .find(|e| {
                if let Edge::SlotEdge { output_index, .. } = e {
                    *output_index == index
                } else {
                    false
                }
            })
            .ok_or(RenderGraphError::UnconnectedNodeOutputSlot {
                output_slot: index,
                node: self.id,
            })
    }
}

/// The internal representation of a [`Node`], with all data required
/// by the [`RenderGraph`](super::RenderGraph).
///
/// The `input_slots` and `output_slots` are provided by the `node`.
pub struct NodeState {
    pub id: NodeId,
    pub name: Option<Cow<'static, str>>,
    /// The name of the type that implements [`Node`].
    pub type_name: &'static str,
    pub node: Box<dyn Node>,
    pub input_slots: SlotInfos,
    pub output_slots: SlotInfos,
    pub edges: Edges,
}

impl Debug for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} ({:?})", self.id, self.name)
    }
}

impl NodeState {
    /// Creates an [`NodeState`] without edges, but the `input_slots` and `output_slots`
    /// are provided by the `node`.
    pub fn new<T>(id: NodeId, node: T) -> Self
    where
        T: Node,
    {
        NodeState {
            id,
            name: None,
            input_slots: node.input().into(),
            output_slots: node.output().into(),
            node: Box::new(node),
            type_name: std::any::type_name::<T>(),
            edges: Edges {
                id,
                input_edges: Vec::new(),
                output_edges: Vec::new(),
            },
        }
    }

    /// Retrieves the [`Node`].
    pub fn node<T>(&self) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.node
            .downcast_ref::<T>()
            .ok_or(RenderGraphError::WrongNodeType)
    }

    /// Retrieves the [`Node`] mutably.
    pub fn node_mut<T>(&mut self) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.node
            .downcast_mut::<T>()
            .ok_or(RenderGraphError::WrongNodeType)
    }

    /// Validates that each input slot corresponds to an input edge.
    pub fn validate_input_slots(&self) -> Result<(), RenderGraphError> {
        for i in 0..self.input_slots.len() {
            self.edges.get_input_slot_edge(i)?;
        }

        Ok(())
    }

    /// Validates that each output slot corresponds to an output edge.
    pub fn validate_output_slots(&self) -> Result<(), RenderGraphError> {
        for i in 0..self.output_slots.len() {
            self.edges.get_output_slot_edge(i)?;
        }

        Ok(())
    }
}

/// A [`NodeLabel`] is used to reference a [`NodeState`] by either its name or [`NodeId`]
/// inside the [`RenderGraph`](super::RenderGraph).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeLabel {
    Id(NodeId),
    Name(Cow<'static, str>),
}

impl From<&NodeLabel> for NodeLabel {
    fn from(value: &NodeLabel) -> Self {
        value.clone()
    }
}

impl From<String> for NodeLabel {
    fn from(value: String) -> Self {
        NodeLabel::Name(value.into())
    }
}

impl From<&'static str> for NodeLabel {
    fn from(value: &'static str) -> Self {
        NodeLabel::Name(value.into())
    }
}

impl From<NodeId> for NodeLabel {
    fn from(value: NodeId) -> Self {
        NodeLabel::Id(value)
    }
}

/// A [`Node`] without any inputs, outputs and subgraphs, which does nothing when run.
/// Used (as a label) to bundle multiple dependencies into one inside
/// the [`RenderGraph`](super::RenderGraph).
pub struct EmptyNode;

impl Node for EmptyNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

/// A [`RenderGraph`](super::RenderGraph) [`Node`] that takes a view entity as input and runs the configured graph name once.
/// This makes it easier to insert sub-graph runs into a graph.
pub struct RunGraphOnViewNode {
    graph_name: Cow<'static, str>,
}

impl RunGraphOnViewNode {
    pub const IN_VIEW: &'static str = "view";
    pub fn new<T: Into<Cow<'static, str>>>(graph_name: T) -> Self {
        Self {
            graph_name: graph_name.into(),
        }
    }
}

impl Node for RunGraphOnViewNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        graph.run_sub_graph(
            self.graph_name.clone(),
            vec![SlotValue::Entity(view_entity)],
        )?;
        Ok(())
    }
}
