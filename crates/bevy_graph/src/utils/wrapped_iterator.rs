/// Trait with helping function for getting the inner iterator of a wrapping iterator.
pub trait WrappedIterator<T>: Iterator {
    /// The inner iterator type.
    type Inner: Iterator<Item = T>;

    /// Gets the inner iterator the current one is wrapping around.
    fn into_inner(self) -> Self::Inner;
}
