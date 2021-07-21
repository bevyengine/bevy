use crate::iter::ParallelIterator;

#[derive(Debug)]
pub struct Chain<T, U> {
    pub(crate) left: T,
    pub(crate) right: U,
    pub(crate) left_in_progress: bool,
}

enum ChainBatch<A, B> {
    A(A),
    B(B),
}

enum Either<A, B> {
    A(A),
    B(B),
}

impl<A, B, I1, I2, T> IntoIterator for ChainBatch<A, B>
where
    A: IntoIterator<IntoIter = I1> + Send,
    B: IntoIterator<IntoIter = I2> + Send,
    I1: Iterator<Item = T>,
    I2: Iterator<Item = T>,
{
    type Item = T;

    type IntoIter = Either<I1, I2>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ChainBatch::A(a) => Either::A(a.into_iter()),
            ChainBatch::B(b) => Either::B(b.into_iter()),
        }
    }
}

impl<A, B, T> Iterator for Either<A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Either::A(a) => a.into_iter().next(),
            Either::B(b) => b.into_iter().next(),
        }
    }
}

impl<I1, I2, T, U, I> ParallelIterator<ChainBatch<I1, I2>> for Chain<T, U>
where
    I1: IntoIterator<Item = I> + Send,
    I2: IntoIterator<Item = I> + Send,
    T: ParallelIterator<I1, Item = I>,
    U: ParallelIterator<I2, Item = I>,
{
    type Item = T::Item;

    fn next_batch(&mut self) -> Option<ChainBatch<I1, I2>> {
        if self.left_in_progress {
            match self.left.next_batch() {
                b @ Some(_) => return b.map(ChainBatch::A),
                None => self.left_in_progress = false,
            }
        }
        self.right.next_batch().map(ChainBatch::B)
    }
}

#[derive(Debug)]
pub struct Map<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, U, T, F> ParallelIterator<Map<B, F>> for Map<U, F>
where
    B: IntoIterator + Send,
    U: ParallelIterator<B, Item = B::Item>,
    F: FnMut(U::Item) -> T + Send + Clone,
{
    type Item = T;

    fn next_batch(&mut self) -> Option<Map<B, F>> {
        self.iter.next_batch().map(|b| Map {
            f: self.f.clone(),
            iter: b,
        })
    }
}

impl<B, F, T> IntoIterator for Map<B, F>
where
    B: IntoIterator + Send,
    F: FnMut(B::Item) -> T + Send + Clone,
{
    type Item = T;

    type IntoIter = std::iter::Map<B::IntoIter, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().map(self.f)
    }
}

#[derive(Debug)]
pub struct Filter<P, F> {
    pub(crate) iter: P,
    pub(crate) predicate: F,
}

impl<B, P, F> ParallelIterator<Filter<B, F>> for Filter<P, F>
where
    B: IntoIterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(&P::Item) -> bool + Send + Clone,
{
    type Item = P::Item;

    fn next_batch(&mut self) -> Option<Filter<B, F>> {
        self.iter.next_batch().map(|iter| Filter {
            iter,
            predicate: self.predicate.clone(),
        })
    }
}

impl<I, F> IntoIterator for Filter<I, F>
where
    I: IntoIterator + Send,
    F: FnMut(&I::Item) -> bool + Send + Clone,
{
    type Item = I::Item;

    type IntoIter = std::iter::Filter<I::IntoIter, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().filter(self.predicate)
    }
}

#[derive(Debug)]
pub struct FilterMap<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, R, F> ParallelIterator<FilterMap<B, F>> for FilterMap<P, F>
where
    B: IntoIterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(P::Item) -> Option<R> + Send + Clone,
{
    type Item = R;

    fn next_batch(&mut self) -> Option<FilterMap<B, F>> {
        self.iter.next_batch().map(|b| FilterMap {
            iter: b,
            f: self.f.clone(),
        })
    }
}

impl<I, F, R> IntoIterator for FilterMap<I, F>
where
    I: IntoIterator + Send,
    F: FnMut(I::Item) -> Option<R> + Send + Clone,
{
    type Item = R;

    type IntoIter = std::iter::FilterMap<I::IntoIter, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().filter_map(self.f)
    }
}

#[derive(Debug)]
pub struct FlatMap<P, F> {
    pub(crate) iter: P,
    pub(crate) f: F,
}

impl<B, P, U, F> ParallelIterator<FlatMap<B, F>> for FlatMap<P, F>
where
    B: IntoIterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(P::Item) -> U + Send + Clone,
    U: IntoIterator,
    U::IntoIter: Send,
{
    type Item = U::Item;

    // This extends each batch using the flat map. The other option is
    // to turn each IntoIter into its own batch.
    fn next_batch(&mut self) -> Option<FlatMap<B, F>> {
        self.iter.next_batch().map(|b| FlatMap {
            iter: b,
            f: self.f.clone(),
        })
    }
}

impl<I, F, U> IntoIterator for FlatMap<I, F>
where
    I: IntoIterator + Send,
    F: FnMut(I::Item) -> U + Send + Clone,
    U: IntoIterator,
    U::IntoIter: Send,
{
    type Item = U::Item;

    type IntoIter = std::iter::FlatMap<I::IntoIter, U, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().flat_map(self.f)
    }
}

#[derive(Debug)]
pub struct Flatten<P> {
    pub(crate) iter: P,
}

impl<B, P> ParallelIterator<Flatten<B>> for Flatten<P>
where
    B: IntoIterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    B::Item: IntoIterator,
    <B::Item as IntoIterator>::IntoIter: Send,
{
    type Item = <P::Item as IntoIterator>::Item;

    // This extends each batch using the flatten. The other option is to
    // turn each IntoIter into its own batch.
    fn next_batch(&mut self) -> Option<Flatten<B>> {
        self.iter.next_batch().map(|b| Flatten { iter: b })
    }
}

impl<I> IntoIterator for Flatten<I>
where
    I: IntoIterator + Send,
    I::Item: IntoIterator,
{
    type Item = <I::Item as IntoIterator>::Item;

    type IntoIter = std::iter::Flatten<I::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().flatten()
    }
}

#[derive(Debug)]
pub struct Fuse<P> {
    pub(crate) iter: Option<P>,
}

impl<B, P> ParallelIterator<B> for Fuse<P>
where
    B: IntoIterator + Send,
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

impl<B, P, F> ParallelIterator<Inspect<B, F>> for Inspect<P, F>
where
    B: IntoIterator + Send,
    P: ParallelIterator<B, Item = B::Item>,
    F: FnMut(&P::Item) + Send + Clone,
{
    type Item = P::Item;

    fn next_batch(&mut self) -> Option<Inspect<B, F>> {
        self.iter.next_batch().map(|b| Inspect {
            iter: b,
            f: self.f.clone(),
        })
    }
}

impl<I, F> IntoIterator for Inspect<I, F>
where
    I: IntoIterator + Send,
    F: FnMut(&I::Item) + Send + Clone,
{
    type Item = I::Item;

    type IntoIter = std::iter::Inspect<I::IntoIter, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().inspect(self.f)
    }
}

#[derive(Debug)]
pub struct Copied<P> {
    pub(crate) iter: P,
}

impl<'a, B, P, T> ParallelIterator<Copied<B>> for Copied<P>
where
    B: IntoIterator<Item = &'a T> + Send,
    P: ParallelIterator<B, Item = &'a T>,
    T: 'a + Copy,
{
    type Item = T;

    fn next_batch(&mut self) -> Option<Copied<B>> {
        self.iter.next_batch().map(|b| Copied { iter: b })
    }
}

impl<'a, I, T> IntoIterator for Copied<I>
where
    I: IntoIterator<Item = &'a T> + Send,
    T: 'a + Copy,
{
    type Item = T;

    type IntoIter = std::iter::Copied<I::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().copied()
    }
}

#[derive(Debug)]
pub struct Cloned<P> {
    pub(crate) iter: P,
}

impl<'a, B, P, T> ParallelIterator<Cloned<B>> for Cloned<P>
where
    B: IntoIterator<Item = &'a T> + Send,
    P: ParallelIterator<B, Item = &'a T>,
    T: 'a + Copy,
{
    type Item = T;

    fn next_batch(&mut self) -> Option<Cloned<B>> {
        self.iter.next_batch().map(|b| Cloned { iter: b })
    }
}

impl<'a, I, T> IntoIterator for Cloned<I>
where
    I: IntoIterator<Item = &'a T> + Send,
    T: 'a + Copy,
{
    type Item = T;

    type IntoIter = std::iter::Cloned<I::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter.into_iter().cloned()
    }
}

#[derive(Debug)]
pub struct Cycle<P> {
    pub(crate) iter: P,
    pub(crate) curr: Option<P>,
}

impl<B, P> ParallelIterator<B> for Cycle<P>
where
    B: IntoIterator + Send,
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
