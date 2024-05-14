use crate::TaskPool;

mod adapters;
pub use adapters::*;

/// [`ParallelIterator`] closely emulates the `std::iter::Iterator`
/// interface. However, it uses `bevy_task` to compute batches in parallel.
///
/// Note that the overhead of [`ParallelIterator`] is high relative to some
/// workloads. In particular, if the batch size is too small or task being
/// run in parallel is inexpensive, *a [`ParallelIterator`] could take longer
/// than a normal [`Iterator`]*. Therefore, you should profile your code before
/// using [`ParallelIterator`].
pub trait ParallelIterator<BatchIter>
where
    BatchIter: Iterator + Send,
    Self: Sized + Send,
{
    /// Returns the next batch of items for processing.
    ///
    /// Each batch is an iterator with items of the same type as the
    /// [`ParallelIterator`]. Returns `None` when there are no batches left.
    fn next_batch(&mut self) -> Option<BatchIter>;

    /// Returns the bounds on the remaining number of items in the
    /// parallel iterator.
    ///
    /// See [`Iterator::size_hint()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.size_hint)
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Consumes the parallel iterator and returns the number of items.
    ///
    /// See [`Iterator::count()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.count)
    fn count(mut self, pool: &TaskPool) -> usize {
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                s.spawn(async move { batch.count() });
            }
        })
        .iter()
        .sum()
    }

    /// Consumes the parallel iterator and returns the last item.
    ///
    /// See [`Iterator::last()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.last)
    fn last(mut self, _pool: &TaskPool) -> Option<BatchIter::Item> {
        let mut last_item = None;
        while let Some(batch) = self.next_batch() {
            last_item = batch.last();
        }
        last_item
    }

    /// Consumes the parallel iterator and returns the nth item.
    ///
    /// See [`Iterator::nth()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.nth)
    // TODO: Optimize with size_hint on each batch
    fn nth(mut self, _pool: &TaskPool, n: usize) -> Option<BatchIter::Item> {
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

    /// Takes two parallel iterators and returns a parallel iterators over
    /// both in sequence.
    ///
    /// See [`Iterator::chain()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.chain)
    // TODO: Use IntoParallelIterator for U
    fn chain<U>(self, other: U) -> Chain<Self, U>
    where
        U: ParallelIterator<BatchIter>,
    {
        Chain {
            left: self,
            right: other,
            left_in_progress: true,
        }
    }

    /// Takes a closure and creates a parallel iterator which calls that
    /// closure on each item.
    ///
    /// See [`Iterator::map()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map)
    fn map<T, F>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(BatchIter::Item) -> T + Send + Clone,
    {
        Map { iter: self, f }
    }

    /// Calls a closure on each item of a parallel iterator.
    ///
    /// See [`Iterator::for_each()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.for_each)
    fn for_each<F>(mut self, pool: &TaskPool, f: F)
    where
        F: FnMut(BatchIter::Item) + Send + Clone + Sync,
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

    /// Creates a parallel iterator which uses a closure to determine
    /// if an element should be yielded.
    ///
    /// See [`Iterator::filter()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter)
    fn filter<F>(self, predicate: F) -> Filter<Self, F>
    where
        F: FnMut(&BatchIter::Item) -> bool,
    {
        Filter {
            iter: self,
            predicate,
        }
    }

    /// Creates a parallel iterator that both filters and maps.
    ///
    /// See [`Iterator::filter_map()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter_map)
    fn filter_map<R, F>(self, f: F) -> FilterMap<Self, F>
    where
        F: FnMut(BatchIter::Item) -> Option<R>,
    {
        FilterMap { iter: self, f }
    }

    /// Creates a parallel iterator that works like map, but flattens
    /// nested structure.
    ///
    /// See [`Iterator::flat_map()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.flat_map)
    fn flat_map<U, F>(self, f: F) -> FlatMap<Self, F>
    where
        F: FnMut(BatchIter::Item) -> U,
        U: IntoIterator,
    {
        FlatMap { iter: self, f }
    }

    /// Creates a parallel iterator that flattens nested structure.
    ///
    /// See [`Iterator::flatten()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.flatten)
    fn flatten(self) -> Flatten<Self>
    where
        BatchIter::Item: IntoIterator,
    {
        Flatten { iter: self }
    }

    /// Creates a parallel iterator which ends after the first None.
    ///
    /// See [`Iterator::fuse()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fuse)
    fn fuse(self) -> Fuse<Self> {
        Fuse { iter: Some(self) }
    }

    /// Does something with each item of a parallel iterator, passing
    /// the value on.
    ///
    /// See [`Iterator::inspect()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.inspect)
    fn inspect<F>(self, f: F) -> Inspect<Self, F>
    where
        F: FnMut(&BatchIter::Item),
    {
        Inspect { iter: self, f }
    }

    /// Borrows a parallel iterator, rather than consuming it.
    ///
    /// See [`Iterator::by_ref()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.by_ref)
    fn by_ref(&mut self) -> &mut Self {
        self
    }

    /// Transforms a parallel iterator into a collection.
    ///
    /// See [`Iterator::collect()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect)
    // TODO: Investigate optimizations for less copying
    fn collect<C>(mut self, pool: &TaskPool) -> C
    where
        C: FromIterator<BatchIter::Item>,
        BatchIter::Item: Send + 'static,
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

    /// Consumes a parallel iterator, creating two collections from it.
    ///
    /// See [`Iterator::partition()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.partition)
    // TODO: Investigate optimizations for less copying
    fn partition<C, F>(mut self, pool: &TaskPool, f: F) -> (C, C)
    where
        C: Default + Extend<BatchIter::Item> + Send,
        F: FnMut(&BatchIter::Item) -> bool + Send + Sync + Clone,
        BatchIter::Item: Send + 'static,
    {
        let (mut a, mut b) = <(C, C)>::default();
        pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let newf = f.clone();
                s.spawn(async move { batch.partition::<Vec<_>, F>(newf) });
            }
        })
        .into_iter()
        .for_each(|(c, d)| {
            a.extend(c);
            b.extend(d);
        });
        (a, b)
    }

    /// Repeatedly applies a function to items of each batch of a parallel
    /// iterator, producing a Vec of final values.
    ///
    /// *Note that this folds each batch independently and returns a Vec of
    /// results (in batch order).*
    ///
    /// See [`Iterator::fold()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold)
    fn fold<C, F, D>(mut self, pool: &TaskPool, init: C, f: F) -> Vec<C>
    where
        F: FnMut(C, BatchIter::Item) -> C + Send + Sync + Clone,
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

    /// Tests if every element of the parallel iterator matches a predicate.
    ///
    /// *Note that all is **not** short circuiting.*
    ///
    /// See [`Iterator::all()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.all)
    fn all<F>(mut self, pool: &TaskPool, f: F) -> bool
    where
        F: FnMut(BatchIter::Item) -> bool + Send + Sync + Clone,
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

    /// Tests if any element of the parallel iterator matches a predicate.
    ///
    /// *Note that any is **not** short circuiting.*
    ///
    /// See [`Iterator::any()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.any)
    fn any<F>(mut self, pool: &TaskPool, f: F) -> bool
    where
        F: FnMut(BatchIter::Item) -> bool + Send + Sync + Clone,
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

    /// Searches for an element in a parallel iterator, returning its index.
    ///
    /// *Note that position consumes the whole iterator.*
    ///
    /// See [`Iterator::position()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.position)
    // TODO: Investigate optimizations for less copying
    fn position<F>(mut self, pool: &TaskPool, f: F) -> Option<usize>
    where
        F: FnMut(BatchIter::Item) -> bool + Send + Sync + Clone,
    {
        let poses = pool.scope(|s| {
            while let Some(batch) = self.next_batch() {
                let mut newf = f.clone();
                s.spawn(async move {
                    let mut len = 0;
                    let mut pos = None;
                    for item in batch {
                        if pos.is_none() && newf(item) {
                            pos = Some(len);
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

    /// Returns the maximum item of a parallel iterator.
    ///
    /// See [`Iterator::max()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.max)
    fn max(mut self, pool: &TaskPool) -> Option<BatchIter::Item>
    where
        BatchIter::Item: Ord + Send + 'static,
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

    /// Returns the minimum item of a parallel iterator.
    ///
    /// See [`Iterator::min()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.min)
    fn min(mut self, pool: &TaskPool) -> Option<BatchIter::Item>
    where
        BatchIter::Item: Ord + Send + 'static,
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

    /// Returns the item that gives the maximum value from the specified function.
    ///
    /// See [`Iterator::max_by_key()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.max_by_key)
    fn max_by_key<R, F>(mut self, pool: &TaskPool, f: F) -> Option<BatchIter::Item>
    where
        R: Ord,
        F: FnMut(&BatchIter::Item) -> R + Send + Sync + Clone,
        BatchIter::Item: Send + 'static,
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

    /// Returns the item that gives the maximum value with respect to the specified comparison
    /// function.
    ///
    /// See [`Iterator::max_by()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.max_by)
    fn max_by<F>(mut self, pool: &TaskPool, f: F) -> Option<BatchIter::Item>
    where
        F: FnMut(&BatchIter::Item, &BatchIter::Item) -> std::cmp::Ordering + Send + Sync + Clone,
        BatchIter::Item: Send + 'static,
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

    /// Returns the item that gives the minimum value from the specified function.
    ///
    /// See [`Iterator::min_by_key()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.min_by_key)
    fn min_by_key<R, F>(mut self, pool: &TaskPool, f: F) -> Option<BatchIter::Item>
    where
        R: Ord,
        F: FnMut(&BatchIter::Item) -> R + Send + Sync + Clone,
        BatchIter::Item: Send + 'static,
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

    /// Returns the item that gives the minimum value with respect to the specified comparison
    /// function.
    ///
    /// See [`Iterator::min_by()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.min_by)
    fn min_by<F>(mut self, pool: &TaskPool, f: F) -> Option<BatchIter::Item>
    where
        F: FnMut(&BatchIter::Item, &BatchIter::Item) -> std::cmp::Ordering + Send + Sync + Clone,
        BatchIter::Item: Send + 'static,
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

    /// Creates a parallel iterator which copies all of its items.
    ///
    /// See [`Iterator::copied()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.copied)
    fn copied<'a, T>(self) -> Copied<Self>
    where
        Self: ParallelIterator<BatchIter>,
        T: 'a + Copy,
    {
        Copied { iter: self }
    }

    /// Creates a parallel iterator which clones all of its items.
    ///
    /// See [`Iterator::cloned()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.cloned)
    fn cloned<'a, T>(self) -> Cloned<Self>
    where
        Self: ParallelIterator<BatchIter>,
        T: 'a + Copy,
    {
        Cloned { iter: self }
    }

    /// Repeats a parallel iterator endlessly.
    ///
    /// See [`Iterator::cycle()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.cycle)
    fn cycle(self) -> Cycle<Self>
    where
        Self: Clone,
    {
        Cycle {
            iter: self,
            curr: None,
        }
    }

    /// Sums the items of a parallel iterator.
    ///
    /// See [`Iterator::sum()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.sum)
    fn sum<S, R>(mut self, pool: &TaskPool) -> R
    where
        S: std::iter::Sum<BatchIter::Item> + Send + 'static,
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

    /// Multiplies all the items of a parallel iterator.
    ///
    /// See [`Iterator::product()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.product)
    fn product<S, R>(mut self, pool: &TaskPool) -> R
    where
        S: std::iter::Product<BatchIter::Item> + Send + 'static,
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
