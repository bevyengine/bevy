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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    fn count(mut self, pool: &TaskPool) -> usize {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.count() })
            }
        })
        .iter()
        .sum()
    }

    fn last(mut self) -> Option<Self::Item> {
        let mut last_item = None;
        while let Some(batch) = self.next_batch() {
            last_item = batch.last();
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

    fn map<T, F>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(Self::Item) -> T + Send + Clone,
    {
        Map { iter: self, f }
    }

    fn for_each<F>(mut self, pool: &TaskPool, f: F)
    where
        F: FnMut(Self::Item) + Send + Clone + Sync,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move {
                    batch.for_each(newf);
                });
            }
        });
    }

    fn filter<F>(self, predicate: F) -> Filter<Self, F>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        Filter {
            iter: self,
            predicate,
        }
    }

    fn filter_map<R, F>(self, f: F) -> FilterMap<Self, F>
    where
        F: FnMut(Self::Item) -> Option<R>,
    {
        FilterMap { iter: self, f }
    }

    fn flat_map<U, F>(self, f: F) -> FlatMap<Self, F>
    where
        F: FnMut(Self::Item) -> U,
        U: IntoIterator,
    {
        FlatMap { iter: self, f }
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self::Item: IntoIterator,
    {
        Flatten { iter: self }
    }

    fn fuse(self) -> Fuse<Self> {
        Fuse { iter: Some(self) }
    }

    fn inspect<F>(self, f: F) -> Inspect<Self, F>
    where
        F: FnMut(&Self::Item),
    {
        Inspect { iter: self, f }
    }

    fn by_ref(&mut self) -> &mut Self {
        self
    }

    // TODO: Investigate optimizations for less copying
    fn collect<C>(mut self, pool: &TaskPool) -> C
    where
        C: std::iter::FromIterator<Self::Item>,
        Self::Item: Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.collect::<Vec<_>>() });
            }
        })
        .into_iter()
        .flatten()
        .collect()
    }

    // TODO: Investigate optimizations for less copying
    fn partition<C, F>(mut self, pool: &TaskPool, f: F) -> (C, C)
    where
        C: Default + Extend<Self::Item> + Send,
        F: FnMut(&Self::Item) -> bool + Send + Sync + Clone,
        Self::Item: Send + 'static,
    {
        let (mut a, mut b) = <(C, C)>::default();
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.partition::<Vec<_>, F>(newf) })
            }
        })
        .into_iter()
        .for_each(|(c, d)| {
            a.extend(c);
            b.extend(d);
        });
        (a, b)
    }

    /// Note that this folds each batch independently and returns a Vec of
    /// results (in batch order).
    fn fold<C, F, D>(mut self, pool: &TaskPool, init: C, f: F) -> Vec<C>
    where
        F: FnMut(C, Self::Item) -> C + Send + Sync + Clone,
        C: Clone + Send + Sync + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                let newi = init.clone();
                s.spawn(async move { batch.fold(newi, newf) });
            }
        })
    }

    /// Note that all is *not* short circuiting
    fn all<F>(mut self, pool: &TaskPool, f: F) -> bool
    where
        F: FnMut(Self::Item) -> bool + Send + Sync + Clone,
    {
        pool.scope(|s| {
            while let Some(mut batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.all(newf) });
            }
        })
        .into_iter()
        .all(std::convert::identity)
    }

    /// Note that any is *not* short circuiting
    fn any<F>(mut self, pool: &TaskPool, f: F) -> bool
    where
        F: FnMut(Self::Item) -> bool + Send + Sync + Clone,
    {
        pool.scope(|s| {
            while let Some(mut batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.any(newf) });
            }
        })
        .into_iter()
        .any(std::convert::identity)
    }

    // TODO: Investigate optimizations for less copying
    /// Note that position consumes the whole iterator
    fn position<F>(mut self, pool: &TaskPool, f: F) -> Option<usize>
    where
        F: FnMut(Self::Item) -> bool + Send + Sync + Clone,
    {
        let poses = pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let mut newf = f.clone();
                s.spawn(async move {
                    let mut len = 0;
                    let mut pos = None;
                    for item in batch {
                        if newf(item) {
                            pos = pos.or(Some(len));
                        }
                        len += 1;
                    }
                    (len, pos)
                });
            }
        });
        let mut start = 0;
        for (len, pos) in poses {
            if let Some(pos) = pos {
                return Some(start + pos);
            }
            start += len;
        }
        None
    }

    fn max(mut self, pool: &TaskPool) -> Option<Self::Item>
    where
        Self::Item: Ord + Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.max() });
            }
        })
        .into_iter()
        .flatten()
        .max()
    }

    fn min(mut self, pool: &TaskPool) -> Option<Self::Item>
    where
        Self::Item: Ord + Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.min() });
            }
        })
        .into_iter()
        .flatten()
        .min()
    }

    fn max_by_key<R, F>(mut self, pool: &TaskPool, f: F) -> Option<Self::Item>
    where
        R: Ord,
        F: FnMut(&Self::Item) -> R + Send + Sync + Clone,
        Self::Item: Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.max_by_key(newf) });
            }
        })
        .into_iter()
        .flatten()
        .max_by_key(f)
    }

    fn max_by<F>(mut self, pool: &TaskPool, f: F) -> Option<Self::Item>
    where
        F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering + Send + Sync + Clone,
        Self::Item: Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.max_by(newf) });
            }
        })
        .into_iter()
        .flatten()
        .max_by(f)
    }

    fn min_by_key<R, F>(mut self, pool: &TaskPool, f: F) -> Option<Self::Item>
    where
        R: Ord,
        F: FnMut(&Self::Item) -> R + Send + Sync + Clone,
        Self::Item: Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.min_by_key(newf) });
            }
        })
        .into_iter()
        .flatten()
        .min_by_key(f)
    }

    fn min_by<F>(mut self, pool: &TaskPool, f: F) -> Option<Self::Item>
    where
        F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering + Send + Sync + Clone,
        Self::Item: Send + 'static,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.min_by(newf) });
            }
        })
        .into_iter()
        .flatten()
        .min_by(f)
    }

    fn copied<'a, T>(self) -> Copied<Self>
    where
        Self: ParallelIterator<B, Item = &'a T>,
        T: 'a + Copy,
    {
        Copied { iter: self }
    }

    fn cloned<'a, T>(self) -> Cloned<Self>
    where
        Self: ParallelIterator<B, Item = &'a T>,
        T: 'a + Copy,
    {
        Cloned { iter: self }
    }

    fn cycle(self) -> Cycle<Self>
    where
        Self: Clone,
    {
        Cycle {
            iter: self,
            curr: None,
        }
    }

    fn sum<S, R>(mut self, pool: &TaskPool) -> R
    where
        S: std::iter::Sum<Self::Item> + Send + 'static,
        R: std::iter::Sum<S>,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.sum() });
            }
        })
        .into_iter()
        .sum()
    }

    fn product<S, R>(mut self, pool: &TaskPool) -> R
    where
        S: std::iter::Product<Self::Item> + Send + 'static,
        R: std::iter::Product<S>,
    {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.product() });
            }
        })
        .into_iter()
        .product()
    }
}
