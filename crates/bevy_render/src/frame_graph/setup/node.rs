use core::fmt::Debug;

use bevy_ecs::world::World;
use downcast_rs::{impl_downcast, Downcast};

use crate::{frame_graph::FrameGraph, render_graph::InternedRenderLabel};

use super::{Edge, SetupGraphContext, SetupGraphError, SlotInfo, SlotInfos};

#[derive(Debug, thiserror::Error)]
pub enum SetupRunError {}

pub trait Setup: Downcast + Send + Sync + 'static {
    fn input(&self) -> Vec<SlotInfo> {
        Vec::new()
    }

    fn output(&self) -> Vec<SlotInfo> {
        Vec::new()
    }
    fn update(&mut self, _world: &mut World) {}

    fn run<'w>(
        &self,
        graph: &mut SetupGraphContext,
        render_context: &mut FrameGraph,
        world: &'w World,
    ) -> Result<(), SetupRunError>;
}

impl_downcast!(Setup);

#[derive(Debug)]
pub struct Edges {
    label: InternedRenderLabel,
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

    /// Returns this node's label.
    #[inline]
    pub fn label(&self) -> InternedRenderLabel {
        self.label
    }

    /// Adds an edge to the `input_edges` if it does not already exist.
    pub(crate) fn add_input_edge(&mut self, edge: Edge) -> Result<(), SetupGraphError> {
        if self.has_input_edge(&edge) {
            return Err(SetupGraphError::EdgeAlreadyExists(edge));
        }
        self.input_edges.push(edge);
        Ok(())
    }

    /// Removes an edge from the `input_edges` if it exists.
    pub(crate) fn remove_input_edge(&mut self, edge: Edge) -> Result<(), SetupGraphError> {
        if let Some(index) = self.input_edges.iter().position(|e| *e == edge) {
            self.input_edges.swap_remove(index);
            Ok(())
        } else {
            Err(SetupGraphError::EdgeDoesNotExist(edge))
        }
    }

    /// Adds an edge to the `output_edges` if it does not already exist.
    pub(crate) fn add_output_edge(&mut self, edge: Edge) -> Result<(), SetupGraphError> {
        if self.has_output_edge(&edge) {
            return Err(SetupGraphError::EdgeAlreadyExists(edge));
        }
        self.output_edges.push(edge);
        Ok(())
    }

    /// Removes an edge from the `output_edges` if it exists.
    pub(crate) fn remove_output_edge(&mut self, edge: Edge) -> Result<(), SetupGraphError> {
        if let Some(index) = self.output_edges.iter().position(|e| *e == edge) {
            self.output_edges.swap_remove(index);
            Ok(())
        } else {
            Err(SetupGraphError::EdgeDoesNotExist(edge))
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
    pub fn get_input_slot_edge(&self, index: usize) -> Result<&Edge, SetupGraphError> {
        self.input_edges
            .iter()
            .find(|e| {
                if let Edge::SlotEdge { input_index, .. } = e {
                    *input_index == index
                } else {
                    false
                }
            })
            .ok_or(SetupGraphError::UnconnectedNodeInputSlot {
                input_slot: index,
                node: self.label,
            })
    }

    /// Searches the `output_edges` for a [`Edge::SlotEdge`],
    /// which `output_index` matches the `index`;
    pub fn get_output_slot_edge(&self, index: usize) -> Result<&Edge, SetupGraphError> {
        self.output_edges
            .iter()
            .find(|e| {
                if let Edge::SlotEdge { output_index, .. } = e {
                    *output_index == index
                } else {
                    false
                }
            })
            .ok_or(SetupGraphError::UnconnectedNodeOutputSlot {
                output_slot: index,
                node: self.label,
            })
    }
}

pub struct NodeState {
    pub label: InternedRenderLabel,
    /// The name of the type that implements [`Node`].
    pub type_name: &'static str,
    pub node: Box<dyn Setup>,
    pub input_slots: SlotInfos,
    pub output_slots: SlotInfos,
    pub edges: Edges,
}

impl Debug for NodeState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{:?} ({})", self.label, self.type_name)
    }
}

impl NodeState {
    /// Creates an [`NodeState`] without edges, but the `input_slots` and `output_slots`
    /// are provided by the `node`.
    pub fn new<T>(label: InternedRenderLabel, node: T) -> Self
    where
        T: Setup,
    {
        NodeState {
            label,
            input_slots: node.input().into(),
            output_slots: node.output().into(),
            node: Box::new(node),
            type_name: core::any::type_name::<T>(),
            edges: Edges {
                label,
                input_edges: Vec::new(),
                output_edges: Vec::new(),
            },
        }
    }

    /// Retrieves the [`Node`].
    pub fn node<T>(&self) -> Result<&T, SetupGraphError>
    where
        T: Setup,
    {
        self.node
            .downcast_ref::<T>()
            .ok_or(SetupGraphError::WrongNodeType)
    }

    /// Retrieves the [`Node`] mutably.
    pub fn node_mut<T>(&mut self) -> Result<&mut T, SetupGraphError>
    where
        T: Setup,
    {
        self.node
            .downcast_mut::<T>()
            .ok_or(SetupGraphError::WrongNodeType)
    }

    /// Validates that each input slot corresponds to an input edge.
    pub fn validate_input_slots(&self) -> Result<(), SetupGraphError> {
        for i in 0..self.input_slots.len() {
            self.edges.get_input_slot_edge(i)?;
        }

        Ok(())
    }

    /// Validates that each output slot corresponds to an output edge.
    pub fn validate_output_slots(&self) -> Result<(), SetupGraphError> {
        for i in 0..self.output_slots.len() {
            self.edges.get_output_slot_edge(i)?;
        }

        Ok(())
    }
}
