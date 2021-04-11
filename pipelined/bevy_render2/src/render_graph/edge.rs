use super::NodeId;

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
}
