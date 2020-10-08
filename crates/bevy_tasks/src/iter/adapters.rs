use crate::iter::ParallelIterator;

#[derive(Debug)]
pub struct Chain<T, U> {
    pub(crate) left: T,
    pub(crate) right: U,
    pub(crate) left_in_progress: bool,
}

impl<B, T, U> ParallelIterator<B> for Chain<T, U>
where
    B: Iterator + Send,
    T: ParallelIterator<B, Item = B::Item>,
    U: ParallelIterator<B, Item = T::Item>,
{
    type Item = T::Item;

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

#[derive(Debug)]
pub struct Map<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, U, T, F> ParallelIterator<std::iter::Map<B, F>> for Map<U, F>
where
    B: Iterator + Send,
    U: ParallelIterator<B, Item = B::Item>,
    F: FnMut(U::Item) -> T + Send + Clone,
{
    type Item = T;

    fn next_batch(&mut self) -> Option<std::iter::Map<B, F>> {
        self.iter.next_batch().map(|b| b.map(self.f.clone()))
    }
}

#[derive(Debug)]
pub struct Filter<P, F> {
    pub(crate) iter: P,
    pub(crate) predicate: F,
}

impl<B, P, F> ParallelIterator<std::iter::Filter<B, F>> for Filter<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(&P::Item) -> bool + Send + Clone,
{
    type Item = P::Item;

    fn next_batch(&mut self) -> Option<std::iter::Filter<B, F>> {
        self.iter
            .next_batch()
            .map(|b| b.filter(self.predicate.clone()))
    }
}

#[derive(Debug)]
pub struct FilterMap<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, R, F> ParallelIterator<std::iter::FilterMap<B, F>> for FilterMap<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(P::Item) -> Option<R> + Send + Clone,
{
    type Item = R;

    fn next_batch(&mut self) -> Option<std::iter::FilterMap<B, F>> {
        self.iter.next_batch().map(|b| b.filter_map(self.f.clone()))
    }
}

#[derive(Debug)]
pub struct FlatMap<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, U, F> ParallelIterator<std::iter::FlatMap<B, U, F>> for FlatMap<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(P::Item) -> U + Send + Clone,
    U: IntoIterator,
    U::IntoIter: Send,
{
    type Item = U::Item;

    // This extends each batch using the flat map. The other option is
    // to turn each IntoIter into its own batch.
    fn next_batch(&mut self) -> Option<std::iter::FlatMap<B, U, F>> {
        self.iter.next_batch().map(|b| b.flat_map(self.f.clone()))
    }
}

#[derive(Debug)]
pub struct Flatten<P> {
    pub(crate) iter: P,
}

impl<B, P> ParallelIterator<std::iter::Flatten<B>> for Flatten<P>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    B::Item: IntoIterator,
    <B::Item as IntoIterator>::IntoIter: Send,
{
    type Item = <P::Item as IntoIterator>::Item;

    // This extends each batch using the flatten. The other option is to
    // turn each IntoIter into its own batch.
    fn next_batch(&mut self) -> Option<std::iter::Flatten<B>> {
        self.iter.next_batch().map(|b| b.flatten())
    }
}

#[derive(Debug)]
pub struct Fuse<P> {
    pub(crate) iter: Option<P>,
}

impl<B, P> ParallelIterator<B> for Fuse<P>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
{
    type Item = P::Item;

    fn next_batch(&mut self) -> Option<B> {
        match &mut self.iter {
            Some(iter) => match iter.next_batch() {
                b @ Some(_) => b,
                None => {
                    self.iter = None;
                    None
                }
            },
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct Inspect<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, F> ParallelIterator<std::iter::Inspect<B, F>> for Inspect<P, F>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(&P::Item) + Send + Clone,
{
    type Item = P::Item;

    fn next_batch(&mut self) -> Option<std::iter::Inspect<B, F>> {
        self.iter.next_batch().map(|b| b.inspect(self.f.clone()))
    }
}

#[derive(Debug)]
pub struct Copied<P> {
    pub(crate) iter: P,
}

impl<'a, B, P, T> ParallelIterator<std::iter::Copied<B>> for Copied<P>
where
    B: Iterator<Item = &'a T> + Send,
    P: ParallelIterator<B, Item = &'a T>,
    T: 'a + Copy,
{
    type Item = T;

    fn next_batch(&mut self) -> Option<std::iter::Copied<B>> {
        self.iter.next_batch().map(|b| b.copied())
    }
}

#[derive(Debug)]
pub struct Cloned<P> {
    pub(crate) iter: P,
}

impl<'a, B, P, T> ParallelIterator<std::iter::Cloned<B>> for Cloned<P>
where
    B: Iterator<Item = &'a T> + Send,
    P: ParallelIterator<B, Item = &'a T>,
    T: 'a + Copy,
{
    type Item = T;

    fn next_batch(&mut self) -> Option<std::iter::Cloned<B>> {
        self.iter.next_batch().map(|b| b.cloned())
    }
}

#[derive(Debug)]
pub struct Cycle<P> {
    pub(crate) iter: P,
    pub(crate) curr: Option<P>,
}

impl<B, P> ParallelIterator<B> for Cycle<P>
where
    B: Iterator + Send,
    P: ParallelIterator<B, Item = B::Item> + Clone,
{
    type Item = P::Item;

    fn next_batch(&mut self) -> Option<B> {
        match self.curr.as_mut().and_then(|c| c.next_batch()) {
            batch @ Some(_) => batch,
            None => {
                self.curr = Some(self.iter.clone());
                self.next_batch()
            }
        }
    }
}
