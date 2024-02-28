use super::{Consumer, Folder, ParallelIterator, Reducer, UnindexedConsumer};
use std::{marker::PhantomData, ptr, slice};

/// We need to transmit raw pointers across threads. It is possible to do this
/// without any unsafe code by converting pointers to usize or to `AtomicPtr<T>`
/// then back to a raw pointer for use. We prefer this approach because code
/// that uses this type is more explicit.
///
/// Unsafe code is still required to dereference the pointer, so this type is
/// not unsound on its own, although it does partly lift the unconditional
/// !Send and !Sync on raw pointers. As always, dereference with care.
struct SendPtr<T>(*mut T);

// SAFETY: !Send for raw pointers is not for safety, just as a lint
unsafe impl<T: Send> Send for SendPtr<T> {}

// SAFETY: !Sync for raw pointers is not for safety, just as a lint
unsafe impl<T: Send> Sync for SendPtr<T> {}

// Implement Clone without the T: Clone bound from the derive
impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implement Copy without the T: Copy bound from the derive
impl<T> Copy for SendPtr<T> {}

pub(super) struct CollectConsumer<'c, T: Send> {
    /// See `CollectResult` for explanation of why this is not a slice
    start: SendPtr<T>,
    len: usize,
    marker: PhantomData<&'c mut T>,
}

impl<T: Send> CollectConsumer<'_, T> {
    /// Create a collector for `len` items in the unused capacity of the vector.
    pub(super) fn appender(vec: &mut Vec<T>, len: usize) -> CollectConsumer<'_, T> {
        let start = vec.len();
        assert!(vec.capacity() - start >= len);

        // SAFETY: We already made sure to have the additional space allocated.
        // The pointer is derived from `Vec` directly, not through a `Deref`,
        // so it has provenance over the whole allocation.
        unsafe { CollectConsumer::new(vec.as_mut_ptr().add(start), len) }
    }
}

impl<'c, T: Send + 'c> CollectConsumer<'c, T> {
    /// The target memory is considered uninitialized, and will be
    /// overwritten without reading or dropping existing values.
    unsafe fn new(start: *mut T, len: usize) -> Self {
        CollectConsumer {
            start: SendPtr(start),
            len,
            marker: PhantomData,
        }
    }
}

impl<'c, T: Send + 'c> UnindexedConsumer<T> for CollectConsumer<'c, T> {
    fn split_off_left(&self) -> Self {
        unreachable!("CollectConsumer must be indexed!")
    }
    fn to_reducer(&self) -> Self::Reducer {
        CollectReducer
    }
}

impl<'c, T: Send + 'c> Consumer<T> for CollectConsumer<'c, T> {
    type Folder = CollectResult<'c, T>;
    type Reducer = CollectReducer;
    type Result = CollectResult<'c, T>;

    fn split_at(self, index: usize) -> (Self, Self, CollectReducer) {
        let CollectConsumer { start, len, .. } = self;

        // Produce new consumers.
        // SAFETY: This assert checks that `index` is a valid offset for `start`
        unsafe {
            assert!(index <= len);
            (
                CollectConsumer::new(start.0, index),
                CollectConsumer::new(start.0.add(index), len - index),
                CollectReducer,
            )
        }
    }

    fn full(&self) -> bool {
        false
    }

    fn into_folder(self) -> Self::Folder {
        // Create a result/folder that consumes values and writes them
        // into the region after start. The initial result has length 0.
        CollectResult {
            start: self.start,
            total_len: self.len,
            initialized_len: 0,
            invariant_lifetime: PhantomData,
        }
    }
}

pub(super) struct CollectReducer;

impl<'c, T> Reducer<CollectResult<'c, T>> for CollectReducer {
    fn reduce(
        self,
        mut left: CollectResult<'c, T>,
        right: CollectResult<'c, T>,
    ) -> CollectResult<'c, T> {
        // Merge if the CollectResults are adjacent and in left to right order
        // else: drop the right piece now and total length will end up short in the end,
        // when the correctness of the collected result is asserted.
        // SAFETY: left and right should be Continuous
        unsafe {
            let left_end = left.start.0.add(left.initialized_len);
            if left_end == right.start.0 {
                left.total_len += right.total_len;
                left.initialized_len += right.release_ownership();
            }
            left
        }
    }
}

/// `CollectResult` represents an initialized part of the target slice.
///
/// This is a proxy owner of the elements in the slice; when it drops,
/// the elements will be dropped, unless its ownership is released before then.
#[must_use]
pub(super) struct CollectResult<'c, T> {
    /// This pointer and length has the same representation as a slice,
    /// but retains the provenance of the entire array so that we can merge
    /// these regions together in `CollectReducer`.
    start: SendPtr<T>,
    total_len: usize,
    /// The current initialized length after `start`
    initialized_len: usize,
    /// Lifetime invariance guarantees that the data flows from consumer to result,
    /// especially for the `scope_fn` callback in `Collect::with_consumer`.
    invariant_lifetime: PhantomData<&'c mut &'c mut [T]>,
}

impl<'c, T> CollectResult<'c, T> {
    /// The current length of the collect result
    pub(super) fn len(&self) -> usize {
        self.initialized_len
    }

    /// Release ownership of the slice of elements, and return the length
    pub(super) fn release_ownership(mut self) -> usize {
        let ret = self.initialized_len;
        self.initialized_len = 0;
        ret
    }
}

// SAFETY: CollectResult<'c,T> can be safely sent across threads as long as its generic type `T` is also `Send`.
unsafe impl<'c, T> Send for CollectResult<'c, T> where T: Send {}

impl<'c, T> Drop for CollectResult<'c, T> {
    fn drop(&mut self) {
        // Drop the first `self.initialized_len` elements, which have been recorded
        // to be initialized by the folder.
        // SAFETY: Caller assures that `release_ownership` has been called
        unsafe {
            ptr::drop_in_place(slice::from_raw_parts_mut(
                self.start.0,
                self.initialized_len,
            ));
        }
    }
}

impl<'c, T: Send + 'c> Folder<T> for CollectResult<'c, T> {
    type Result = Self;

    fn consume(mut self, item: T) -> Self {
        assert!(
            self.initialized_len < self.total_len,
            "too many values pushed to consumer"
        );

        // SAFETY: The assert above is a bounds check for this write, and we
        // avoid assignment here so we do not drop an uninitialized T.
        unsafe {
            // Write item and increase the initialized length
            self.start.0.add(self.initialized_len).write(item);
            self.initialized_len += 1;
        }

        self
    }

    fn complete(self) -> Self::Result {
        // NB: We don't explicitly check that the local writes were complete,
        // but Collect will assert the total result length in the end.
        self
    }

    fn full(&self) -> bool {
        false
    }
}

/// Collects the results of the iterator into the specified vector.
///
/// Technically, this only works for `IndexedParallelIterator`, but we're faking a
/// bit of specialization here until Rust can do that natively.  Callers are
/// using `opt_len` to find the length before calling this, and only exact
/// iterators will return anything but `None` there.
///
/// Since the type system doesn't understand that contract, we have to allow
/// *any* `ParallelIterator` here, and `CollectConsumer` has to also implement
/// `UnindexedConsumer`.  That implementation panics `unreachable!` in case
/// there's a bug where we actually do try to use this unindexed.
pub(crate) fn special_extend<I, T>(pi: I, len: usize, v: &mut Vec<T>)
where
    I: ParallelIterator<Item = T>,
    T: Send,
{
    collect_with_consumer(v, len, |consumer| pi.drive_unindexed(consumer));
}

/// Create a consumer on the slice of memory we are collecting into.
///
/// The consumer needs to be used inside the scope function, and the
/// complete collect result passed back.
///
/// This method will verify the collect result, and panic if the slice
/// was not fully written into. Otherwise, in the successful case,
/// the vector is complete with the collected result.
fn collect_with_consumer<T, F>(vec: &mut Vec<T>, len: usize, scope_fn: F)
where
    T: Send,
    F: FnOnce(CollectConsumer<'_, T>) -> CollectResult<'_, T>,
{
    // Reserve space for `len` more elements in the vector,
    vec.reserve(len);

    // Create the consumer and run the callback for collection.
    let result = scope_fn(CollectConsumer::appender(vec, len));

    // The `CollectResult` represents a contiguous part of the slice, that has
    // been written to. On unwind here, the `CollectResult` will be dropped. If
    // some producers on the way did not produce enough elements, partial
    // `CollectResult`s may have been dropped without being reduced to the final
    // result, and we will see that as the length coming up short.
    //
    // Here, we assert that added length is fully initialized. This is checked
    // by the following assert, which verifies if a complete `CollectResult`
    // was produced; if the length is correct, it is necessarily covering the
    // target slice. Since we know that the consumer cannot have escaped from
    // `drive` (by parametricity, essentially), we know that any stores that
    // will happen, have happened. Unless some code is buggy, that means we
    // should have seen `len` total writes.
    let actual_writes = result.len();
    assert!(
        actual_writes == len,
        "expected {} total writes, but got {}",
        len,
        actual_writes
    );

    // Release the result's mutable borrow and "proxy ownership"
    // of the elements, before the vector takes it over.
    result.release_ownership();

    let new_len = vec.len() + len;

    // SAFETY: The assert checks `new_len` is valid
    unsafe {
        vec.set_len(new_len);
    }
}
