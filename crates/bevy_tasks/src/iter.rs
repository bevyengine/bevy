use crate::TaskPool;

/// ParallelIterator closely emulates the std::iter::Iterator
/// interface. However, it uses bevy_task to compute batches in parallel.
pub trait ParallelIterator<B>
where
    B: Iterator<Item = Self::Item> + Send,
    Self: Sized + Send,
{
    type Item;

    fn next_batch(&mut self) -> Option<B>;
    fn task_pool(&self) -> &TaskPool;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    fn count(mut self) -> usize {
        self.task_pool()
            .clone()
            .scope(|s| {
                while let Some(batch) = self.next_batch() {
                    s.spawn(async move { batch.count() })
                }
            })
            .iter()
            .sum()
    }

    fn last(mut self) -> Option<Self::Item> {
        let mut last_item = None;
        loop {
            match self.next_batch() {
                Some(batch) => last_item = batch.last(),
                None => break,
            }
        }
        last_item
    }

    // TODO: Optimize with size_hint on each batch
    fn nth(mut self, n: usize) -> Option<Self::Item> {
        let mut i = 0;
        while let Some(batch) = self.next_batch() {
            for item in batch {
                if i == n {
                    return Some(item);
                }
                i += 1;
            }
        }
        None
    }

    // TODO: Use IntoParallelIterator for U
    fn chain<U>(self, other: U) -> Chain<Self, U>
    where
        U: ParallelIterator<B, Item = Self::Item>,
    {
        Chain {
            left: self,
            right: other,
            left_in_progress: true,
        }
    }

    // TODO: Use IntoParallelIterator for U
    fn zip<U, B2>(self, other: U) -> Zip<B, B2, Self, U>
    where
        B2: Iterator + Send,
        U: ParallelIterator<B2, Item = B2::Item>,
    {
        Zip {
            left: self,
            left_batch: None,
            right: other,
            right_batch: None,
        }
    }

    fn map<T, F>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(Self::Item) -> T + Send + Clone,
    {
        Map { iter: self, f }
    }

    fn for_each<F>(mut self, f: F)
    where
        F: FnMut(Self::Item) + Send + Clone + Sync,
    {
        self.task_pool().clone().scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move {
                    batch.for_each(newf);
                });
            }
        });
    }
}

pub struct Chain<T, U> {
    left: T,
    right: U,
    left_in_progress: bool,
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

    fn task_pool(&self) -> &TaskPool {
        if self.left_in_progress {
            self.left.task_pool()
        } else {
            self.right.task_pool()
        }
    }
}

pub struct Zip<B1, B2, T, U> {
    left: T,
    left_batch: Option<B1>,
    right: U,
    right_batch: Option<B2>,
}

impl<B1, B2, T, U> ParallelIterator<std::iter::Zip<B1, B2>> for Zip<B1, B2, T, U>
where
    B1: Iterator + Send,
    B2: Iterator + Send,
    T: ParallelIterator<B1, Item = B1::Item>,
    U: ParallelIterator<B2, Item = B2::Item>,
{
    type Item = (T::Item, U::Item);

    fn next_batch(&mut self) -> Option<std::iter::Zip<B1, B2>> {
        unimplemented!()
    }

    // TODO: not sure what to do with this
    fn task_pool(&self) -> &TaskPool {
        self.left.task_pool()
    }
}

pub struct Map<P, F> {
    iter: P,
    f: F,
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

    fn task_pool(&self) -> &TaskPool {
        self.iter.task_pool()
    }
}
