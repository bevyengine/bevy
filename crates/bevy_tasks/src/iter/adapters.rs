use crate::{iter::ParallelIterator, TaskPool};

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

pub struct Zip<B1, B2, T, U> {
    pub(crate) left: T,
    pub(crate) left_batch: Option<B1>,
    pub(crate) right: U,
    pub(crate) right_batch: Option<B2>,
}

impl<B1, B2, T, U> ParallelIterator<std::iter::Zip<B1, B2>> for Zip<B1, B2, T, U>
where
    B1: Iterator + Send,
    B2: Iterator + Send,
    T: ParallelIterator<B1, Item = B1::Item>,
    U: ParallelIterator<B2, Item = B2::Item>,
{
    type Item = (T::Item, U::Item);

    /// Note: Zip::next_batch() nessesarily reduces the batch size to 1
    fn next_batch(&mut self) -> Option<std::iter::Zip<B1, B2>> {
        unimplemented!()
    }
}

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
