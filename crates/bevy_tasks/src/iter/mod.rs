use crate::TaskPool;

mod adapters;
pub use adapters::*;

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
