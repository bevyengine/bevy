//! Various utilities to make working with ranges easier.

use core::ops::{Bound, Range, RangeBounds};

/// Returns another range that describes the same bounds as the given range.
///
/// Using this function instead of requiring that the value that implements
/// `RangeBounds<T>` also implement `Clone` results in tidier function
/// signatures for functions that take `impl RangeBounds<T>`.
pub fn clone_range<T>(range: &impl RangeBounds<T>) -> (Bound<T>, Bound<T>)
where
    T: Clone,
{
    (range.start_bound().cloned(), range.end_bound().cloned())
}

/// Converts a `u32` range to a `usize` range.
///
/// This could be implemented as a blanket conversion that converts `Range<T>`
/// to `Range<U>` for any `U: From<T>`, except that `usize` doesn't implement
/// `From<u32>`. The types are guaranteed to be convertible on any system that
/// supports Bevy, though (i.e. one with at least 32-bit pointers).
pub fn u32_range_to_usize_range(range: impl RangeBounds<u32>) -> (Bound<usize>, Bound<usize>) {
    (
        range.start_bound().map(|&value| value as usize),
        range.end_bound().map(|&value| value as usize),
    )
}

/// Resolves a `RangeBounds<usize>` to a concrete `Range<usize>`, given a
/// length of the slice that the range refers to.
///
/// This is basically like `std::slice::range`, but available on stable Rust.
pub fn slice_range(range: impl RangeBounds<usize>, len: usize) -> Range<usize> {
    let start = match range.start_bound() {
        Bound::Included(index) => *index,
        Bound::Excluded(index) => *index + 1,
        Bound::Unbounded => 0,
    };
    #[cfg(debug_assertions)]
    assert!(start <= len, "tried to create an out-of-bounds slice");

    let end = match range.end_bound() {
        Bound::Included(index) => *index + 1,
        Bound::Excluded(index) => *index,
        Bound::Unbounded => len,
    };
    #[cfg(debug_assertions)]
    assert!(end <= len, "tried to create an out-of-bounds slice");

    start..end
}
