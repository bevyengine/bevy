/// Operation to perform to the output attachment at the start of a renderpass.
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum LoadOp<V> {
    /// Clear with a specified value.
    Clear(V),
    /// Load from memory.
    Load,
}

/// Pair of load and store operations for an attachment aspect.
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Operations<V> {
    /// How data should be read through this attachment.
    pub load: LoadOp<V>,
    /// Whether data will be written to through this attachment.
    pub store: bool,
}