use super::{NodeId, NodeState};
use bevy_utils::HashMap;
use std::borrow::Cow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Edge {
    SlotEdge {
        input_node: NodeId,
        input_index: usize,
        output_node: NodeId,
        output_index: usize,
    },
    NodeEdge {
        input_node: NodeId,
        output_node: NodeId,
    },
}

impl Edge {
    pub fn get_input_node(&self) -> NodeId {
        match self {
            Edge::SlotEdge { input_node, .. } => *input_node,
            Edge::NodeEdge { input_node, .. } => *input_node,
        }
    }

    pub fn get_output_node(&self) -> NodeId {
        match self {
            Edge::SlotEdge { output_node, .. } => *output_node,
            Edge::NodeEdge { output_node, .. } => *output_node,
        }
    }

    pub fn fmt_as_output_edge(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        nodes: &HashMap<NodeId, NodeState>,
    ) -> std::fmt::Result {
        match self {
            Edge::SlotEdge { input_index, output_index, .. } => write!(f, " SlotEdge(in #{}, out #{}", input_index, output_index)?,
            Edge::NodeEdge { .. } => write!(f, " NodeEdge(")?,
        }
        let node = nodes.get(&self.get_input_node()).unwrap();
        write!(f, "{:?})", node)
    }

    pub fn fmt_as_input_edge(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        nodes: &HashMap<NodeId, NodeState>,
    ) -> std::fmt::Result {
        match self {
            Edge::SlotEdge { input_index, output_index, .. } => write!(f, " SlotEdge( in #{}, out #{}, ", input_index, output_index)?,
            Edge::NodeEdge {..} => write!(f, " NodeEdge(")?,
        }
        let node = nodes.get(&self.get_output_node()).unwrap();
        write!(f, "{:?})", node)
    }
}
