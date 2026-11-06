pub trait BsnFmt {
    /// Formats the node.
    /// `base_indent` is the starting column of the macro (for context alignment).
    /// `level` tracks the internal depth of the BSN tree.
    fn fmt(&self, base_indent: usize, level: usize) -> String;
}
