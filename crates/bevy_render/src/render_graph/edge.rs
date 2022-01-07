use super::NodeId;

/// An edge, which connects two [`Nodes`](super::Node) in
/// a [`RenderGraph`](crate::render_graph::RenderGraph).
///
/// They are used to describe the ordering (which node has to run first)
/// and may be of two kinds: [`NodeEdge`](Self::NodeEdge) and [`SlotEdge`](Self::SlotEdge).
///
/// Edges are added via the render_graph::add_node_edge(output_node, input_node) and the
/// render_graph::add_slot_edge(output_node, output_slot, input_node, input_slot) methods.
///
/// The former simply states that the `output_node` has to be run before the `input_node`,
/// while the later connects an output slot of the `output_node`
/// with an input slot of the `input_node` to pass additional data along.
/// For more information see [`SlotType`](super::SlotType).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub before: NodeId,
    pub after: NodeId,
}


impl Edge {
    /// Returns the id of the 'input_node'.
    pub fn get_before_node(&self) -> NodeId {
        self.before
    }

    /// Returns the id of the 'output_node'.
    pub fn get_after_node(&self) -> NodeId {
        self.after
    }
}
