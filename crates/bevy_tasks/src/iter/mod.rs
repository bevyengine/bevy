use std::cmp;

use crate::{compute_task_pool_thread_num, ComputeTaskPool, TaskPool};

use self::map::Map;

mod collect;
mod extend;
mod for_each;
mod from_par_iter;
mod map;
mod noop;

/// This helper function is used to "connect" a parallel iterator to a
/// consumer. It will convert the `par_iter` into a producer P and
/// then pull items from P and feed them to `consumer`, splitting and
/// creating parallel threads as needed.
pub fn bridge<I, C>(par_iter: I, consumer: C) -> C::Result
where
    I: IndexedParallelIterator,
    C: Consumer<I::Item>,
{
    let len = par_iter.len();
    return par_iter.with_producer(Callback { len, consumer });

    struct Callback<C> {
        len: usize,
        consumer: C,
    }

    impl<C, I> ProducerCallback<I> for Callback<C>
    where
        C: Consumer<I>,
    {
        type Output = C::Result;
        fn callback<P>(self, producer: P) -> C::Result
        where
            P: Producer<Item = I>,
        {
            bridge_producer_consumer(self.len, producer, self.consumer)
        }
    }
}

/// TODO: optimize it
fn join<A, B, RA, RB>(pool: &TaskPool, op_a: A, op_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    let mut ra = None;
    let mut rb = None;
    pool.scope(|s| {
        s.spawn(async {
            rb = Some(op_b());
        });
        ra = Some(op_a());
    });
    (ra.unwrap(), rb.unwrap())
}

/// This helper function is used to "connect" a producer and a
/// consumer. You may prefer to call [`bridge`], which wraps this
/// function. This function will draw items from `producer` and feed
/// them to `consumer`, splitting and creating parallel tasks when
/// needed.
pub fn bridge_producer_consumer<P, C>(len: usize, producer: P, consumer: C) -> C::Result
where
    P: Producer,
    C: Consumer<P::Item>,
{
    let splitter = LengthSplitter::new(producer.min_len(), producer.max_len(), len);
    return helper(len, splitter, producer, consumer);

    fn helper<P, C>(len: usize, mut splitter: LengthSplitter, producer: P, consumer: C) -> C::Result
    where
        P: Producer,
        C: Consumer<P::Item>,
    {
        if consumer.full() {
            consumer.into_folder().complete()
        } else if splitter.try_split(len) {
            // TODO: optimize it
            // Increasing thread number may not necessarily enhance performance due to the split method.
            // Additional benefits can only be realized when the number of threads reaches the next power of 2.
            // Rayon may split tasks into smaller slices in some cases, but Bevy's executor suffers from overhead
            // when spawning a large number of small tasks.
            let mid = len / 2;
            let (left_producer, right_producer) = producer.split_at(mid);
            let (left_consumer, right_consumer, reducer) = consumer.split_at(mid);
            let (left_result, right_result) = join(
                ComputeTaskPool::get(),
                || helper(mid, splitter, left_producer, left_consumer),
                || helper(len - mid, splitter, right_producer, right_consumer),
            );
            reducer.reduce(left_result, right_result)
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }
}

/// The `Folder` trait encapsulates [the standard fold
/// operation][fold].  It can be fed many items using the `consume`
/// method. At the end, once all items have been consumed, it can then
/// be converted (using `complete`) into a final value.
///
/// [fold]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold
pub trait Folder<Item>: Sized {
    /// The type of result that will ultimately be produced by the folder.
    type Result;

    /// Consume next item and return new sequential state.
    fn consume(self, item: Item) -> Self;

    /// Consume items from the iterator until full, and return new sequential state.
    ///
    /// This method is **optional**. The default simply iterates over
    /// `iter`, invoking `consume` and checking after each iteration
    /// whether `full` returns false.
    ///
    /// The main reason to override it is if you can provide a more
    /// specialized, efficient implementation.
    fn consume_iter<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = Item>,
    {
        for item in iter {
            self = self.consume(item);
            if self.full() {
                break;
            }
        }
        self
    }

    /// Finish consuming items, produce final result.
    fn complete(self) -> Self::Result;

    /// Hint whether this `Folder` would like to stop processing
    /// further items, e.g. if a search has been completed.
    fn full(&self) -> bool;
}

/// A `Producer` is effectively a "splittable `IntoIterator`". That
/// is, a producer is a value which can be converted into an iterator
/// at any time: at that point, it simply produces items on demand,
/// like any iterator. But what makes a `Producer` special is that,
/// *before* we convert to an iterator, we can also **split** it at a
/// particular point using the `split_at` method. This will yield up
/// two producers, one producing the items before that point, and one
/// producing the items after that point (these two producers can then
/// independently be split further, or be converted into iterators).
pub trait Producer: Send + Sized {
    /// The type of item that will be produced by this producer once
    /// it is converted into an iterator.
    type Item;

    /// The type of iterator we will become.
    type IntoIter: Iterator<Item = Self::Item>;

    /// Convert `self` into an iterator; at this point, no more parallel splits
    /// are possible.
    fn into_iter(self) -> Self::IntoIter;

    /// The minimum number of items that we will process
    /// sequentially. Defaults to 1, which means that we will split
    /// all the way down to a single item.
    fn min_len(&self) -> usize {
        1
    }

    /// The maximum number of items that we will process
    /// sequentially. Defaults to MAX, which means that we can choose
    /// not to split at all.  
    fn max_len(&self) -> usize {
        usize::MAX
    }

    /// Split into two producers; one produces items `0..index`, the
    /// other `index..N`. Index must be less than or equal to `N`.
    fn split_at(self, index: usize) -> (Self, Self);

    /// Iterate the producer, feeding each element to `folder`, and
    /// stop when the folder is full (or all elements have been consumed).
    ///
    /// The provided implementation is sufficient for most iterables.
    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        folder.consume_iter(self.into_iter())
    }
}

/// The `ProducerCallback` trait is a kind of generic closure,
/// [analogous to `FnOnce`][FnOnce].
pub trait ProducerCallback<T> {
    /// The type of value returned by this callback. Analogous to
    /// [`Output` from the `FnOnce` trait][Output].
    ///
    /// [Output]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html#associatedtype.Output
    type Output;

    /// Invokes the callback with the given producer as argument. The
    /// key point of this trait is that this method is generic over
    /// `P`, and hence implementors must be defined for any producer.
    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>;
}

/// The reducer is the final step of a `Consumer` -- after a consumer
/// has been split into two parts, and each of those parts has been
/// fully processed, we are left with two results. The reducer is then
/// used to combine those two results into one.
pub trait Reducer<Result> {
    /// Reduce two final results into one; this is executed after a
    /// split.
    fn reduce(self, left: Result, right: Result) -> Result;
}

/// A consumer is effectively a [generalized "fold" operation][fold],
/// and in fact each consumer will eventually be converted into a
/// [`Folder`]. What makes a consumer special is that, like a
/// [`Producer`], it can be **split** into multiple consumers using
/// the `split_at` method. When a consumer is split, it produces two
/// consumers, as well as a **reducer**. The two consumers can be fed
/// items independently, and when they are done the reducer is used to
/// combine their two results into one.
///
/// [fold]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold
pub trait Consumer<Item>: Send + Sized {
    /// The type of folder that this consumer can be converted into.
    type Folder: Folder<Item, Result = Self::Result>;

    /// The type of reducer that is produced if this consumer is split.
    type Reducer: Reducer<Self::Result>;

    /// The type of result that this consumer will ultimately produce.
    type Result: Send;

    /// Divide the consumer into two consumers, one processing items
    /// `0..index` and one processing items from `index..`. Also
    /// produces a reducer that can be used to reduce the results at
    /// the end.
    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer);

    /// Convert the consumer into a folder that can consume items
    /// sequentially, eventually producing a final result.
    fn into_folder(self) -> Self::Folder;

    /// Hint whether this `Consumer` would like to stop processing
    /// further items, e.g. if a search has been completed.
    fn full(&self) -> bool;
}

/// A stateless consumer can be freely copied. These consumers can be
/// used like regular consumers, but they also support a
/// `split_off_left` method that does not take an index to split, but
/// simply splits at some arbitrary point (`for_each`, for example,
/// produces an unindexed consumer).
pub trait UnindexedConsumer<I>: Consumer<I> {
    /// Splits off a "left" consumer and returns it. The `self`
    /// consumer should then be used to consume the "right" portion of
    /// the data. (The ordering matters for methods like `find_first` --
    /// values produced by the returned value are given precedence
    /// over values produced by `self`.) Once the left and right
    /// halves have been fully consumed, you should reduce the results
    /// with the result of `to_reducer`.
    fn split_off_left(&self) -> Self;

    /// Creates a reducer that can be used to combine the results from
    /// a split consumer.
    fn to_reducer(&self) -> Self::Reducer;
}

/// Parallel version of the standard iterator trait.
///
/// The combinators on this trait are available on **all** parallel
/// iterators.  Additional methods can be found on the
/// [`IndexedParallelIterator`] trait: those methods are only
/// available for parallel iterators where the number of items is
/// known in advance (so, e.g., after invoking `filter`, those methods
/// become unavailable).
pub trait ParallelIterator: Sized + Send {
    /// The type of item that this parallel iterator produces.
    /// For example, if you use the [`for_each`] method, this is the type of
    /// item that your closure will be invoked with.
    ///
    /// [`for_each`]: #method.for_each
    type Item: Send;
    /// Executes `OP` on each item produced by the iterator, in parallel.
    fn for_each<OP>(self, op: OP)
    where
        OP: Fn(Self::Item) + Sync + Send,
    {
        for_each::for_each(self, &op);
    }

    /// Applies `map_op` to each item of this iterator, producing a new
    /// iterator with the results.
    fn map<F, R>(self, map_op: F) -> Map<Self, F>
    where
        F: Fn(Self::Item) -> R + Sync + Send,
        R: Send,
    {
        Map::new(self, map_op)
    }

    /// Creates a fresh collection containing all the elements produced
    /// by this parallel iterator.
    fn collect<C>(self) -> C
    where
        C: FromParallelIterator<Self::Item>,
    {
        C::from_par_iter(self)
    }

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// Returns the number of items produced by this iterator, if known
    /// statically. This can be used by consumers to trigger special fast
    /// paths. Therefore, if `Some(_)` is returned, this iterator must only
    /// use the (indexed) `Consumer` methods when driving a consumer, such
    /// as `split_at()`. Calling `UnindexedConsumer::split_off_left()` or
    /// other `UnindexedConsumer` methods -- or returning an inaccurate
    /// value -- may result in panics.
    ///
    /// This method is currently used to optimize `collect` for want
    /// of true Rust specialization; it may be removed when
    /// specialization is stable.
    fn opt_len(&self) -> Option<usize> {
        None
    }
}

/// An iterator that supports "random access" to its data, meaning
/// that you can split it at arbitrary indices and draw data from
/// those points.
///
/// **Note:** Not implemented for `u64`, `i64`, `u128`, or `i128` ranges
// Waiting for `ExactSizeIterator::is_empty` to be stabilized. See rust-lang/rust#35428
#[allow(clippy::len_without_is_empty)]
pub trait IndexedParallelIterator: ParallelIterator {
    /// Produces an exact count of how many items this iterator will
    /// produce, presuming no panic occurs.
    fn len(&self) -> usize;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method causes the iterator `self` to start producing
    /// items and to feed them to the consumer `consumer` one by one.
    /// It may split the consumer before doing so to create the
    /// opportunity to produce in parallel. If a split does happen, it
    /// will inform the consumer of the index where the split should
    /// occur (unlike `ParallelIterator::drive_unindexed()`).
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method converts the iterator into a producer P and then
    /// invokes `callback.callback()` with P. Note that the type of
    /// this producer is not defined as part of the API, since
    /// `callback` must be defined generically for all producers. This
    /// allows the producer type to contain references; it also means
    /// that parallel iterators can adjust that type without causing a
    /// breaking change.
    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output;
}

/// `FromParallelIterator` implements the creation of a collection
/// from a [`ParallelIterator`]. By implementing
/// `FromParallelIterator` for a given type, you define how it will be
/// created from an iterator.
///
/// `FromParallelIterator` is used through [`ParallelIterator`]
pub trait FromParallelIterator<T>
where
    T: Send,
{
    /// Creates an instance of the collection from the parallel iterator `par_iter`.
    ///
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = T>;
}

/// `IntoParallelIterator` implements the conversion to a [`ParallelIterator`].
pub trait IntoParallelIterator {
    /// The parallel iterator type that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    type Item: Send;

    /// Converts `self` into a parallel iterator.
    fn into_par_iter(self) -> Self::Iter;
}

impl<T: ParallelIterator> IntoParallelIterator for T {
    type Iter = T;
    type Item = T::Item;

    fn into_par_iter(self) -> T {
        self
    }
}

/// `IntoParallelRefIterator` implements the conversion to a
/// [`ParallelIterator`], providing shared references to the data.
pub trait IntoParallelRefIterator<'data> {
    /// The type of the parallel iterator that will be returned.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This will typically be an `&'data T` reference type.
    type Item: Send + 'data;

    /// Converts `self` into a parallel iterator.
    fn par_iter(&'data self) -> Self::Iter;
}

impl<'data, I: 'data + ?Sized> IntoParallelRefIterator<'data> for I
where
    &'data I: IntoParallelIterator,
{
    type Iter = <&'data I as IntoParallelIterator>::Iter;
    type Item = <&'data I as IntoParallelIterator>::Item;

    fn par_iter(&'data self) -> Self::Iter {
        self.into_par_iter()
    }
}

/// `IntoParallelRefMutIterator` implements the conversion to a
/// [`ParallelIterator`], providing mutable references to the data.
///
/// This is a parallel version of the `iter_mut()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &mut I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
pub trait IntoParallelRefMutIterator<'data> {
    /// The type of iterator that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that will be produced; this is typically an
    /// `&'data mut T` reference.
    type Item: Send + 'data;

    /// Creates the parallel iterator from `self`.
    fn par_iter_mut(&'data mut self) -> Self::Iter;
}

impl<'data, I: 'data + ?Sized> IntoParallelRefMutIterator<'data> for I
where
    &'data mut I: IntoParallelIterator,
{
    type Iter = <&'data mut I as IntoParallelIterator>::Iter;
    type Item = <&'data mut I as IntoParallelIterator>::Item;

    fn par_iter_mut(&'data mut self) -> Self::Iter {
        self.into_par_iter()
    }
}

/// `ParallelExtend` extends an existing collection with items from a [`ParallelIterator`].
pub trait ParallelExtend<T>
where
    T: Send,
{
    /// Extends an instance of the collection with the elements drawn
    /// from the parallel iterator `par_iter`.
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>;
}

/// A splitter controls the policy for splitting into smaller work items.
///
/// Thief-splitting is an adaptive policy that starts by splitting into
/// enough jobs for every worker thread, and then resets itself whenever a
/// job is actually stolen into a different thread.
#[derive(Clone, Copy)]
struct Splitter {
    /// The `splits` tell us approximately how many remaining times we'd
    /// like to split this job.  We always just divide it by two though, so
    /// the effective number of pieces will be `next_power_of_two()`.
    splits: usize,
}

impl Splitter {
    #[inline]
    fn new() -> Splitter {
        Splitter {
            splits: compute_task_pool_thread_num(),
        }
    }

    #[inline]
    fn try_split(&mut self) -> bool {
        let Splitter { splits } = *self;

        if splits > 0 {
            // We have splits remaining, make it so.
            self.splits /= 2;
            true
        } else {
            false
        }
    }
}

/// The length splitter is built on thief-splitting, but additionally takes
/// into account the remaining length of the iterator.
#[derive(Clone, Copy)]
struct LengthSplitter {
    inner: Splitter,

    /// The smallest we're willing to divide into.  Usually this is just 1,
    /// but you can choose a larger working size with `with_min_len()`.
    min: usize,
}

impl LengthSplitter {
    /// Creates a new splitter based on lengths.
    ///
    /// The `min` is a hard lower bound.  We'll never split below that, but
    /// of course an iterator might start out smaller already.
    ///
    /// The `max` is an upper bound on the working size, used to determine
    /// the minimum number of times we need to split to get under that limit.
    /// The adaptive algorithm may very well split even further, but never
    /// smaller than the `min`.
    #[inline]
    fn new(min: usize, max: usize, len: usize) -> LengthSplitter {
        let mut splitter = LengthSplitter {
            inner: Splitter::new(),
            min: cmp::max(min, 1),
        };

        // Divide the given length by the max working length to get the minimum
        // number of splits we need to get under that max.  This rounds down,
        // but the splitter actually gives `next_power_of_two()` pieces anyway.
        // e.g. len 12345 / max 100 = 123 min_splits -> 128 pieces.
        let min_splits = len / cmp::max(max, 1);

        // Only update the value if it's not splitting enough already.
        if min_splits > splitter.inner.splits {
            splitter.inner.splits = min_splits;
        }

        splitter
    }

    #[inline]
    fn try_split(&mut self, len: usize) -> bool {
        // If splitting wouldn't make us too small, try the inner splitter.
        len / 2 >= self.min && self.inner.try_split()
    }
}
