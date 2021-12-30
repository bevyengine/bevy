use super::NodeId;

/// An edge, which connects two [`Nodes`](super::Node) in
/// a [`RenderGraph`](crate::render_graph::RenderGraph).
///
/// They are used to describe the ordering (which node has to run first)
/// and may be of two kinds: [`NodeEdge`](Self::NodeEdge) and [`SlotEdge`](Self::SlotEdge).
///
/// Edges are added via the render_graph::add_node_edge(output_node, input_node) and the
/// render_graph::add_slot_edge(output_node, output_slot, input_node, input_slot) methode.
///
/// The former simply states that the `output_node` has to be run before the `input_node`,
/// while the later connects an output slot of the `output_node`
/// with an input slot of the `input_node` to pass additional data along.
/// For more information see [`SlotType`](super::SlotType).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Edge {
    /// An edge describing to ordering of both nodes (`output_node` before `input_node`)
    /// and connecting the output slot at the `output_index` of the output_node
    /// with the slot at the `input_index` of the `input_node`.
    SlotEdge {
        input_node: NodeId,
        input_index: usize,
        output_node: NodeId,
        output_index: usize,
    },
    /// An edge describing to ordering of both nodes (`output_node` before `input_node`).
    NodeEdge {
        input_node: NodeId,
        output_node: NodeId,
    },
}

impl Edge {
    /// Returns the id of the 'input_node'.
    pub fn get_input_node(&self) -> NodeId {
        match self {
            Edge::SlotEdge { input_node, .. } => *input_node,
            Edge::NodeEdge { input_node, .. } => *input_node,
        }
    }

    /// Returns the id of the 'output_node'.
    pub fn get_output_node(&self) -> NodeId {
        match self {
            Edge::SlotEdge { output_node, .. } => *output_node,
            Edge::NodeEdge { output_node, .. } => *output_node,
        }
    }
}
