use slotmap::new_key_type;

new_key_type! {
    /// A key that holds an index to a node in a graph.
    pub struct NodeIdx;
    /// A key that holds an index to an edge in a graph.
    pub struct EdgeIdx;
}
