/// Trait with helping function for yielding the index instead of the value.
pub trait WrappedIndicesIterator<I>: Iterator {
    /// The indices iterator type.
    type IndicesIter: Iterator<Item = I>;

    /// Returns an iterator which yields the index instead of the value.
    fn into_indices(self) -> Self::IndicesIter;
}
