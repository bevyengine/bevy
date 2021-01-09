use super::{Edge, RenderGraphError, ResourceSlotInfo, ResourceSlots};
use crate::renderer::RenderContext;
use bevy_ecs::{BoxedSystem, Commands, Resources, World};
use bevy_utils::Uuid;
use downcast_rs::{impl_downcast, Downcast};
use std::{borrow::Cow, fmt::Debug};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(Uuid);

impl NodeId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }
}

pub trait Node: Downcast + Send + Sync + 'static {
    fn input(&self) -> &[ResourceSlotInfo] {
        &[]
    }

    fn output(&self) -> &[ResourceSlotInfo] {
        &[]
    }

    fn update(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        output: &mut ResourceSlots,
    );
}

impl_downcast!(Node);

pub trait SystemNode: Node {
    fn get_system(&self, commands: &mut Commands) -> BoxedSystem;
}

#[derive(Debug)]
pub struct Edges {
    pub id: NodeId,
    pub input_edges: Vec<Edge>,
    pub output_edges: Vec<Edge>,
}

impl Edges {
    pub(crate) fn add_input_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if self.has_input_edge(&edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge));
        }
        self.input_edges.push(edge);
        Ok(())
    }

    pub(crate) fn add_output_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if self.has_output_edge(&edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge));
        }
        self.output_edges.push(edge);
        Ok(())
    }

    pub fn has_input_edge(&self, edge: &Edge) -> bool {
        self.input_edges.contains(edge)
    }

    pub fn has_output_edge(&self, edge: &Edge) -> bool {
        self.output_edges.contains(edge)
    }

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

pub struct NodeState {
    pub id: NodeId,
    pub name: Option<Cow<'static, str>>,
    pub node: Box<dyn Node>,
    pub input_slots: ResourceSlots,
    pub output_slots: ResourceSlots,
    pub edges: Edges,
}

impl Debug for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} ({:?})", self.id, self.name)
    }
}

impl NodeState {
    pub fn new<T>(id: NodeId, node: T) -> Self
    where
        T: Node,
    {
        NodeState {
            id,
            name: None,
            input_slots: ResourceSlots::from(node.input()),
            output_slots: ResourceSlots::from(node.output()),
            node: Box::new(node),
            edges: Edges {
                id,
                input_edges: Vec::new(),
                output_edges: Vec::new(),
            },
        }
    }

    pub fn node<T>(&self) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.node
            .downcast_ref::<T>()
            .ok_or(RenderGraphError::WrongNodeType)
    }

    pub fn node_mut<T>(&mut self) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.node
            .downcast_mut::<T>()
            .ok_or(RenderGraphError::WrongNodeType)
    }

    pub fn validate_output_slots(&self) -> Result<(), RenderGraphError> {
        for i in 0..self.output_slots.len() {
            self.edges.get_output_slot_edge(i)?;
        }

        Ok(())
    }

    pub fn validate_input_slots(&self) -> Result<(), RenderGraphError> {
        for i in 0..self.input_slots.len() {
            self.edges.get_input_slot_edge(i)?;
        }

        Ok(())
    }
}

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
