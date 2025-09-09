use crate::iter::ParallelIterator;

/// Chains two [`ParallelIterator`]s `T` and `U`, first returning
/// batches from `T`, and then from `U`.
#[derive(Debug)]
pub struct Chain<T, U> {
    pub(crate) left: T,
    pub(crate) right: U,
    pub(crate) left_in_progress: bool,
}

impl<B, T, U> ParallelIterator<B> for Chain<T, U>
where
    B: Iterator + Send,
    T: ParallelIterator<B>,
    U: ParallelIterator<B>,
{
    fn next_batch(&mut self) -> Option<B> {
        if self.left_in_progress {
            match self.left.next_batch() {
                b @ Some(_) => return b,
                None => self.left_in_progress = false,
            }
        }
        self.right.next_batch()
    }
}

/// Maps a [`ParallelIterator`] `P` using the provided function `F`.
#[derive(Debug)]
pub struct Map<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, U, T, F> ParallelIterator<core::iter::Map<B, F>> for Map<U, F>
where
    B: Iterator + Send,
    U: ParallelIterator<B>,
    F: FnMut(B::Item) -> T + Send + Clone,
{
    fn next_batch(&mut self) -> Option<core::iter::Map<B, F>> {
        self.iter.next_batch().map(|b| b.map(self.f.clone()))
    }
}

/// Filters a [`ParallelIterator`] `P` using the provided predicate `F`.
#[derive(Debug)]
pub struct Filter<P, F> {
    pub(crate) iter: P,
    pub(crate) predicate: F,
}

impl<B, P, F> ParallelIterator<core::iter::Filter<B, F>> for Filter<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B>,
    F: FnMut(&B::Item) -> bool + Send + Clone,
{
    fn next_batch(&mut self) -> Option<core::iter::Filter<B, F>> {
        self.iter
            .next_batch()
            .map(|b| b.filter(self.predicate.clone()))
    }
}

/// Filter-maps a [`ParallelIterator`] `P` using the provided function `F`.
#[derive(Debug)]
pub struct FilterMap<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, R, F> ParallelIterator<core::iter::FilterMap<B, F>> for FilterMap<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B>,
    F: FnMut(B::Item) -> Option<R> + Send + Clone,
{
    fn next_batch(&mut self) -> Option<core::iter::FilterMap<B, F>> {
        self.iter.next_batch().map(|b| b.filter_map(self.f.clone()))
    }
}

/// Flat-maps a [`ParallelIterator`] `P` using the provided function `F`.
#[derive(Debug)]
pub struct FlatMap<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, U, F> ParallelIterator<core::iter::FlatMap<B, U, F>> for FlatMap<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B>,
    F: FnMut(B::Item) -> U + Send + Clone,
    U: IntoIterator,
    U::IntoIter: Send,
{
    // This extends each batch using the flat map. The other option is
    // to turn each IntoIter into its own batch.
    fn next_batch(&mut self) -> Option<core::iter::FlatMap<B, U, F>> {
        self.iter.next_batch().map(|b| b.flat_map(self.f.clone()))
    }
}

/// Flattens a [`ParallelIterator`] `P`.
#[derive(Debug)]
pub struct Flatten<P> {
    pub(crate) iter: P,
}

impl<B, P> ParallelIterator<core::iter::Flatten<B>> for Flatten<P>
where
    B: Iterator + Send,
    P: ParallelIterator<B>,
    B::Item: IntoIterator,
    <B::Item as IntoIterator>::IntoIter: Send,
{
    // This extends each batch using the flatten. The other option is to
    // turn each IntoIter into its own batch.
    fn next_batch(&mut self) -> Option<core::iter::Flatten<B>> {
        self.iter.next_batch().map(Iterator::flatten)
    }
}

/// Fuses a [`ParallelIterator`] `P`, ensuring once it returns [`None`] once, it always
/// returns [`None`].
#[derive(Debug)]
pub struct Fuse<P> {
    pub(crate) iter: Option<P>,
}

impl<B, P> ParallelIterator<B> for Fuse<P>
where
    B: Iterator + Send,
    P: ParallelIterator<B>,
{
    fn next_batch(&mut self) -> Option<B> {
        match &mut self.iter {
            Some(iter) => iter.next_batch().or_else(|| {
                self.iter = None;
                None
            }),
            None => None,
        }
    }
}

/// Inspects a [`ParallelIterator`] `P` using the provided function `F`.
#[derive(Debug)]
pub struct Inspect<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, F> ParallelIterator<core::iter::Inspect<B, F>> for Inspect<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B>,
    F: FnMut(&B::Item) + Send + Clone,
{
    fn next_batch(&mut self) -> Option<core::iter::Inspect<B, F>> {
        self.iter.next_batch().map(|b| b.inspect(self.f.clone()))
    }
}

/// Copies a [`ParallelIterator`] `P`'s returned values.
#[derive(Debug)]
pub struct Copied<P> {
    pub(crate) iter: P,
}

impl<'a, B, P, T> ParallelIterator<core::iter::Copied<B>> for Copied<P>
where
    B: Iterator<Item = &'a T> + Send,
    P: ParallelIterator<B>,
    T: 'a + Copy,
{
    fn next_batch(&mut self) -> Option<core::iter::Copied<B>> {
        self.iter.next_batch().map(Iterator::copied)
    }
}

/// Clones a [`ParallelIterator`] `P`'s returned values.
#[derive(Debug)]
pub struct Cloned<P> {
    pub(crate) iter: P,
}

impl<'a, B, P, T> ParallelIterator<core::iter::Cloned<B>> for Cloned<P>
where
    B: Iterator<Item = &'a T> + Send,
    P: ParallelIterator<B>,
    T: 'a + Copy,
{
    fn next_batch(&mut self) -> Option<core::iter::Cloned<B>> {
        self.iter.next_batch().map(Iterator::cloned)
    }
}

/// Cycles a [`ParallelIterator`] `P` indefinitely.
#[derive(Debug)]
pub struct Cycle<P> {
    pub(crate) iter: P,
    pub(crate) curr: Option<P>,
}

impl<B, P> ParallelIterator<B> for Cycle<P>
where
    B: Iterator + Send,
    P: ParallelIterator<B> + Clone,
{
    fn next_batch(&mut self) -> Option<B> {
        self.curr
            .as_mut()
            .and_then(ParallelIterator::next_batch)
            .or_else(|| {
                self.curr = Some(self.iter.clone());
                self.next_batch()
            })
    }
}
