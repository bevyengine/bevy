use super::NodeId;

/// An edge, which connects two [`Nodes`](super::Node) in
/// a [`RenderGraph`](crate::render_graph::RenderGraph).
///
/// They are used to describe the ordering (which node has to run first)
/// and may be of two kinds: [`NodeEdge`](Self::NodeEdge) and [`SlotEdge`](Self::SlotEdge).
///
/// Edges are added via the `render_graph::add_node_edge(output_node, input_node)` and the
/// `render_graph::add_slot_edge(output_node, output_slot, input_node, input_slot)` methods.
///
/// The former simply states that the `output_node` has to be run before the `input_node`,
/// while the later connects an output slot of the `output_node`
/// with an input slot of the `input_node` to pass additional data along.
/// For more information see [`SlotType`](super::SlotType).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    /// An edge describing to ordering of both nodes (`output_node` before `input_node`).
    pub input_node: NodeId,
    pub output_node: NodeId,
}

#[derive(PartialEq, Eq)]
pub enum EdgeExistence {
    Exists,
    DoesNotExist,
}
