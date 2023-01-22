/// Trait with helping function for getting the inner iterator of a wrapping iterator.
pub trait WrappedIterator<S: Iterator<Item = T>, T, I: Iterator>: Iterator<Item = T> {
    /// Gets the inner iterator the current one is wrapping around.
    fn into_inner(self) -> I;
}
