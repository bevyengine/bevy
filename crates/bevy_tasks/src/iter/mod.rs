use std::cmp;

use crate::{compute_task_pool_thread_num, ComputeTaskPool, TaskPool};

mod collect;
mod extend;
mod for_each;
mod from_par_iter;
mod noop;

// pub use self::collect::special_extend;
/// This helper function is used to "connect" a parallel iterator to a
/// consumer. It will convert the `par_iter` into a producer P and
/// then pull items from P and feed them to `consumer`, splitting and
/// creating parallel threads as needed.
///
/// This is useful when you are implementing your own parallel
/// iterators: it is often used as the definition of the
/// [`drive_unindexed`] or [`drive`] methods.
///
/// [`drive_unindexed`]: ../trait.ParallelIterator.html#tymethod.drive_unindexed
/// [`drive`]: ../trait.IndexedParallelIterator.html#tymethod.drive
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
            ra = Some(op_a());
        });
        s.spawn(async {
            rb = Some(op_b());
        });
    });
    (ra.unwrap(), rb.unwrap())
}

/// todo
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

pub trait Producer: Send + Sized {
    /// The type of item that will be produced by this producer once
    /// it is converted into an iterator.
    type Item;

    /// The type of iterator we will become.
    type IntoIter: Iterator<Item = Self::Item> + DoubleEndedIterator + ExactSizeIterator;

    /// Convert `self` into an iterator; at this point, no more parallel splits
    /// are possible.
    fn into_iter(self) -> Self::IntoIter;

    /// The minimum number of items that we will process
    /// sequentially. Defaults to 1, which means that we will split
    /// all the way down to a single item. This can be raised higher
    /// using the [`with_min_len`] method, which will force us to
    /// create sequential tasks at a larger granularity. Note that
    /// Rayon automatically normally attempts to adjust the size of
    /// parallel splits to reduce overhead, so this should not be
    /// needed.
    ///
    /// [`with_min_len`]: ../trait.IndexedParallelIterator.html#method.with_min_len
    fn min_len(&self) -> usize {
        1
    }

    /// The maximum number of items that we will process
    /// sequentially. Defaults to MAX, which means that we can choose
    /// not to split at all. This can be lowered using the
    /// [`with_max_len`] method, which will force us to create more
    /// parallel tasks. Note that Rayon automatically normally
    /// attempts to adjust the size of parallel splits to reduce
    /// overhead, so this should not be needed.
    ///
    /// [`with_max_len`]: ../trait.IndexedParallelIterator.html#method.with_max_len
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
/// [analogous to `FnOnce`][FnOnce]. See [the corresponding section in
/// the plumbing README][r] for more details.
///
/// [r]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md#producer-callback
/// [FnOnce]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html
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

pub trait Reducer<Result> {
    /// Reduce two final results into one; this is executed after a
    /// split.
    fn reduce(self, left: Result, right: Result) -> Result;
}

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
    /// the data. (The ordering matters for methods like find_first --
    /// values produced by the returned value are given precedence
    /// over values produced by `self`.) Once the left and right
    /// halves have been fully consumed, you should reduce the results
    /// with the result of `to_reducer`.
    fn split_off_left(&self) -> Self;

    /// Creates a reducer that can be used to combine the results from
    /// a split consumer.
    fn to_reducer(&self) -> Self::Reducer;
}

pub trait ParallelIterator: Sized + Send {
    /// The type of item that this parallel iterator produces.
    /// For example, if you use the [`for_each`] method, this is the type of
    /// item that your closure will be invoked with.
    ///
    /// [`for_each`]: #method.for_each
    type Item: Send;
    /// Executes `OP` on each item produced by the iterator, in parallel.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// (0..100).into_par_iter().for_each(|x| println!("{:?}", x));
    /// ```
    fn for_each<OP>(self, op: OP)
    where
        OP: Fn(Self::Item) + Sync + Send,
    {
        for_each::for_each(self, &op)
    }

    fn collect<C>(self) -> C
    where
        C: FromParallelIterator<Self::Item>,
    {
        C::from_par_iter(self)
    }

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method causes the iterator `self` to start producing
    /// items and to feed them to the consumer `consumer` one by one.
    /// It may split the consumer before doing so to create the
    /// opportunity to produce in parallel.
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md
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

pub trait IndexedParallelIterator: ParallelIterator {
    /// Produces an exact count of how many items this iterator will
    /// produce, presuming no panic occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let par_iter = (0..100).into_par_iter().zip(vec![0; 10]);
    /// assert_eq!(par_iter.len(), 10);
    ///
    /// let vec: Vec<_> = par_iter.collect();
    /// assert_eq!(vec.len(), 10);
    /// ```
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
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md
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
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md
    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output;
}

/// `FromParallelIterator` implements the creation of a collection
/// from a [`ParallelIterator`]. By implementing
/// `FromParallelIterator` for a given type, you define how it will be
/// created from an iterator.
///
/// `FromParallelIterator` is used through [`ParallelIterator`]'s [`collect()`] method.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`collect()`]: trait.ParallelIterator.html#method.collect
///
/// # Examples
///
/// Implementing `FromParallelIterator` for your type:
///
/// ```
/// use rayon::prelude::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<T: Send> FromParallelIterator<T> for BlackHole {
///     fn from_par_iter<I>(par_iter: I) -> Self
///         where I: IntoParallelIterator<Item = T>
///     {
///         let par_iter = par_iter.into_par_iter();
///         BlackHole {
///             mass: par_iter.count() * mem::size_of::<T>(),
///         }
///     }
/// }
///
/// let bh: BlackHole = (0i32..1000).into_par_iter().collect();
/// assert_eq!(bh.mass, 4000);
/// ```
pub trait FromParallelIterator<T>
where
    T: Send,
{
    /// Creates an instance of the collection from the parallel iterator `par_iter`.
    ///
    /// If your collection is not naturally parallel, the easiest (and
    /// fastest) way to do this is often to collect `par_iter` into a
    /// [`LinkedList`] or other intermediate data structure and then
    /// sequentially extend your collection. However, a more 'native'
    /// technique is to use the [`par_iter.fold`] or
    /// [`par_iter.fold_with`] methods to create the collection.
    /// Alternatively, if your collection is 'natively' parallel, you
    /// can use `par_iter.for_each` to process each element in turn.
    ///
    /// [`LinkedList`]: https://doc.rust-lang.org/std/collections/struct.LinkedList.html
    /// [`par_iter.fold`]: trait.ParallelIterator.html#method.fold
    /// [`par_iter.fold_with`]: trait.ParallelIterator.html#method.fold_with
    /// [`par_iter.for_each`]: trait.ParallelIterator.html#method.for_each
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = T>;
}

/// `IntoParallelIterator` implements the conversion to a [`ParallelIterator`].
///
/// By implementing `IntoParallelIterator` for a type, you define how it will
/// transformed into an iterator. This is a parallel version of the standard
/// library's [`std::iter::IntoIterator`] trait.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`std::iter::IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
pub trait IntoParallelIterator {
    /// The parallel iterator type that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    type Item: Send;

    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// println!("counting in parallel:");
    /// (0..100).into_par_iter()
    ///     .for_each(|i| println!("{}", i));
    /// ```
    ///
    /// This conversion is often implicit for arguments to methods like [`zip`].
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let v: Vec<_> = (0..5).into_par_iter().zip(5..10).collect();
    /// assert_eq!(v, [(0, 5), (1, 6), (2, 7), (3, 8), (4, 9)]);
    /// ```
    ///
    /// [`zip`]: trait.IndexedParallelIterator.html#method.zip
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
///
/// This is a parallel version of the `iter()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefIterator<'data> {
    /// The type of the parallel iterator that will be returned.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This will typically be an `&'data T` reference type.
    type Item: Send + 'data;

    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let v: Vec<_> = (0..100).collect();
    /// assert_eq!(v.par_iter().sum::<i32>(), 100 * 99 / 2);
    ///
    /// // `v.par_iter()` is shorthand for `(&v).into_par_iter()`,
    /// // producing the exact same references.
    /// assert!(v.par_iter().zip(&v)
    ///          .all(|(a, b)| std::ptr::eq(a, b)));
    /// ```
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

pub trait IntoParallelRefMutIterator<'data> {
    /// The type of iterator that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that will be produced; this is typically an
    /// `&'data mut T` reference.
    type Item: Send + 'data;

    /// Creates the parallel iterator from `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut v = vec![0usize; 5];
    /// v.par_iter_mut().enumerate().for_each(|(i, x)| *x = i);
    /// assert_eq!(v, [0, 1, 2, 3, 4]);
    /// ```
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
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
///
/// # Examples
///
/// Implementing `ParallelExtend` for your type:
///
/// ```
/// use rayon::prelude::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<T: Send> ParallelExtend<T> for BlackHole {
///     fn par_extend<I>(&mut self, par_iter: I)
///         where I: IntoParallelIterator<Item = T>
///     {
///         let par_iter = par_iter.into_par_iter();
///         self.mass += par_iter.count() * mem::size_of::<T>();
///     }
/// }
///
/// let mut bh = BlackHole { mass: 0 };
/// bh.par_extend(0i32..1000);
/// assert_eq!(bh.mass, 4000);
/// bh.par_extend(0i64..10);
/// assert_eq!(bh.mass, 4080);
/// ```
pub trait ParallelExtend<T>
where
    T: Send,
{
    /// Extends an instance of the collection with the elements drawn
    /// from the parallel iterator `par_iter`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut vec = vec![];
    /// vec.par_extend(0..5);
    /// vec.par_extend((0..5).into_par_iter().map(|i| i * i));
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 0, 1, 4, 9, 16]);
    /// ```
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
            // Not stolen, and no more splits -- we're done!
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
