use super::NodeId;

/// An edge, which connects two [`Nodes`](super::Node) in
/// a [`RenderGraph`](crate::render_graph::RenderGraph).
///
/// They are used to describe the ordering **on the gpu side** (the order which the commands are run)
///
/// Edges are added via the `render_graph::add_edge(before, after)`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub before: NodeId,
    pub after: NodeId,
}

impl Edge {
    /// Returns the id of the before node.
    pub fn get_before_node(&self) -> NodeId {
        self.before
    }

    /// Returns the id of the after node.
    pub fn get_after_node(&self) -> NodeId {
        self.after
    }
}
