use core::{
    array::TryFromSliceError,
    borrow::Borrow,
    cmp::Ordering,
    fmt::Debug,
    iter::FusedIterator,
    ops::{
        Bound, Deref, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
        RangeToInclusive,
    },
    ptr,
    slice::{self, SliceIndex},
};

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::VecDeque,
    rc::Rc,
    sync::Arc,
    vec::Vec,
};

use super::{
    unique_vec, EntitySet, EntitySetIterator, FromEntitySetIterator, TrustedEntityBorrow,
    UniqueEntityArray, UniqueEntityIter, UniqueEntityVec,
};

/// A slice that contains only unique entities.
///
/// It can be obtained by slicing [`UniqueEntityVec`].
#[repr(transparent)]
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntitySlice<T: TrustedEntityBorrow>([T]);

impl<T: TrustedEntityBorrow> UniqueEntitySlice<T> {
    /// Constructs a `UniqueEntitySlice` from a [`&[T]`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub const unsafe fn from_slice_unchecked(slice: &[T]) -> &Self {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { &*(ptr::from_ref(slice) as *const Self) }
    }

    /// Constructs a `UniqueEntitySlice` from a [`&mut [T]`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut [T]) -> &mut Self {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Casts to `self` to a standard slice.
    pub const fn as_inner(&self) -> &[T] {
        &self.0
    }

    /// Constructs a `UniqueEntitySlice` from a [`Box<[T]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<[T]>) -> Box<Self> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_boxed_inner(self: Box<Self>) -> Box<[T]> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { Box::from_raw(Box::into_raw(self) as *mut [T]) }
    }

    /// Constructs a `UniqueEntitySlice` from a [`Arc<[T]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_arc_slice_unchecked(slice: Arc<[T]>) -> Arc<Self> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { Arc::from_raw(Arc::into_raw(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_arc_inner(self: Arc<Self>) -> Arc<[T]> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { Arc::from_raw(Arc::into_raw(self) as *mut [T]) }
    }

    // Constructs a `UniqueEntitySlice` from a [`Rc<[T]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_rc_slice_unchecked(slice: Rc<[T]>) -> Rc<Self> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { Rc::from_raw(Rc::into_raw(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_rc_inner(self: Rc<Self>) -> Rc<[T]> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { Rc::from_raw(Rc::into_raw(self) as *mut [T]) }
    }

    /// Returns the first and all the rest of the elements of the slice, or `None` if it is empty.
    ///
    /// Equivalent to [`[T]::split_first`](slice::split_first).
    pub const fn split_first(&self) -> Option<(&T, &Self)> {
        let Some((first, rest)) = self.0.split_first() else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        Some((first, unsafe { Self::from_slice_unchecked(rest) }))
    }

    /// Returns the last and all the rest of the elements of the slice, or `None` if it is empty.
    ///
    /// Equivalent to [`[T]::split_last`](slice::split_last).
    pub const fn split_last(&self) -> Option<(&T, &Self)> {
        let Some((last, rest)) = self.0.split_last() else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        Some((last, unsafe { Self::from_slice_unchecked(rest) }))
    }

    /// Returns an array reference to the first `N` items in the slice.
    ///
    /// Equivalent to [`[T]::first_chunk`](slice::first_chunk).
    pub const fn first_chunk<const N: usize>(&self) -> Option<&UniqueEntityArray<T, N>> {
        let Some(chunk) = self.0.first_chunk() else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        Some(unsafe { UniqueEntityArray::from_array_ref_unchecked(chunk) })
    }

    /// Returns an array reference to the first `N` items in the slice and the remaining slice.
    ///
    /// Equivalent to [`[T]::split_first_chunk`](slice::split_first_chunk).
    pub const fn split_first_chunk<const N: usize>(
        &self,
    ) -> Option<(&UniqueEntityArray<T, N>, &UniqueEntitySlice<T>)> {
        let Some((chunk, rest)) = self.0.split_first_chunk() else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            Some((
                UniqueEntityArray::from_array_ref_unchecked(chunk),
                Self::from_slice_unchecked(rest),
            ))
        }
    }

    /// Returns an array reference to the last `N` items in the slice and the remaining slice.
    ///
    /// Equivalent to [`[T]::split_last_chunk`](slice::split_last_chunk).
    pub const fn split_last_chunk<const N: usize>(
        &self,
    ) -> Option<(&UniqueEntitySlice<T>, &UniqueEntityArray<T, N>)> {
        let Some((rest, chunk)) = self.0.split_last_chunk() else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            Some((
                Self::from_slice_unchecked(rest),
                UniqueEntityArray::from_array_ref_unchecked(chunk),
            ))
        }
    }

    /// Returns an array reference to the last `N` items in the slice.
    ///
    /// Equivalent to [`[T]::last_chunk`](slice::last_chunk).
    pub const fn last_chunk<const N: usize>(&self) -> Option<&UniqueEntityArray<T, N>> {
        let Some(chunk) = self.0.last_chunk() else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        Some(unsafe { UniqueEntityArray::from_array_ref_unchecked(chunk) })
    }

    /// Returns a reference to a subslice.
    ///
    /// Equivalent to the range functionality of [`[T]::get`].
    ///
    /// Note that only the inner [`[T]::get`] supports indexing with a [`usize`].
    ///
    /// [`[T]::get`]: `slice::get`
    pub fn get<I>(&self, index: I) -> Option<&Self>
    where
        Self: Index<I>,
        I: SliceIndex<[T], Output = [T]>,
    {
        self.0.get(index).map(|slice|
            // SAFETY: All elements in the original slice are unique.
            unsafe { Self::from_slice_unchecked(slice) })
    }

    /// Returns a mutable reference to a subslice.
    ///
    /// Equivalent to the range functionality of [`[T]::get_mut`].
    ///
    /// Note that `UniqueEntitySlice::get_mut` cannot be called with a [`usize`].
    ///
    /// [`[T]::get_mut`]: `slice::get_mut`s
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut Self>
    where
        Self: Index<I>,
        I: SliceIndex<[T], Output = [T]>,
    {
        self.0.get_mut(index).map(|slice|
            // SAFETY: All elements in the original slice are unique.
            unsafe { Self::from_slice_unchecked_mut(slice) })
    }

    /// Returns a reference to a subslice, without doing bounds checking.
    ///
    /// Equivalent to the range functionality of [`[T]::get_unchecked`].
    ///
    /// Note that only the inner [`[T]::get_unchecked`] supports indexing with a [`usize`].
    ///
    /// # Safety
    ///
    /// `index` must be safe to use with [`[T]::get_unchecked`]
    ///
    /// [`[T]::get_unchecked`]: `slice::get_unchecked`
    pub unsafe fn get_unchecked<I>(&self, index: I) -> &Self
    where
        Self: Index<I>,
        I: SliceIndex<[T], Output = [T]>,
    {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.get_unchecked(index)) }
    }
    /// Returns a mutable reference to a subslice, without doing bounds checking.
    ///
    /// Equivalent to the range functionality of [`[T]::get_unchecked_mut`].
    ///
    /// Note that `UniqueEntitySlice::get_unchecked_mut` cannot be called with an index.
    ///
    /// # Safety
    ///
    /// `index` must be safe to use with [`[T]::get_unchecked_mut`]
    ///
    /// [`[T]::get_unchecked_mut`]: `slice::get_unchecked_mut`
    pub unsafe fn get_unchecked_mut<I>(&mut self, index: I) -> &mut Self
    where
        Self: Index<I>,
        I: SliceIndex<[T], Output = [T]>,
    {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.get_unchecked_mut(index)) }
    }

    /// Returns an unsafe mutable pointer to the slice's buffer.
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr()
    }

    /// Returns the two unsafe mutable pointers spanning the slice.
    pub const fn as_mut_ptr_range(&mut self) -> Range<*mut T> {
        self.0.as_mut_ptr_range()
    }

    /// Swaps two elements in the slice.
    pub fn swap(&mut self, a: usize, b: usize) {
        self.0.swap(a, b);
    }

    /// Reverses the order of elements in the slice, in place.
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Returns an iterator over the slice.
    pub fn iter(&self) -> Iter<'_, T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }

    /// Returns an iterator over all contiguous windows of length
    /// `size`.
    ///
    /// Equivalent to [`[T]::windows`].
    ///
    /// [`[T]::windows`]: `slice::windows`
    pub fn windows(&self, size: usize) -> Windows<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.windows(size)) }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the
    /// beginning of the slice.
    ///
    /// Equivalent to [`[T]::chunks`].
    ///
    /// [`[T]::chunks`]: `slice::chunks`
    pub fn chunks(&self, chunk_size: usize) -> Chunks<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.chunks(chunk_size)) }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the
    /// beginning of the slice.
    ///
    /// Equivalent to [`[T]::chunks_mut`].
    ///
    /// [`[T]::chunks_mut`]: `slice::chunks_mut`
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(
                self.0.chunks_mut(chunk_size),
            )
        }
    }

    ///
    ///
    /// Equivalent to [`[T]::chunks_exact`].
    ///
    /// [`[T]::chunks_exact`]: `slice::chunks_exact`
    pub fn chunks_exact(&self, chunk_size: usize) -> ChunksExact<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.chunks_exact(chunk_size))
        }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the
    /// beginning of the slice.
    ///
    /// Equivalent to [`[T]::chunks_exact_mut`].
    ///
    /// [`[T]::chunks_exact_mut`]: `slice::chunks_exact_mut`
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(
                self.0.chunks_exact_mut(chunk_size),
            )
        }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the end
    /// of the slice.
    ///
    /// Equivalent to [`[T]::rchunks`].
    ///
    /// [`[T]::rchunks`]: `slice::rchunks`
    pub fn rchunks(&self, chunk_size: usize) -> RChunks<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.rchunks(chunk_size)) }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the end
    /// of the slice.
    ///
    /// Equivalent to [`[T]::rchunks_mut`].
    ///
    /// [`[T]::rchunks_mut`]: `slice::rchunks_mut`
    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(
                self.0.rchunks_mut(chunk_size),
            )
        }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the
    /// end of the slice.
    ///
    /// Equivalent to [`[T]::rchunks_exact`].
    ///
    /// [`[T]::rchunks_exact`]: `slice::rchunks_exact`
    pub fn rchunks_exact(&self, chunk_size: usize) -> RChunksExact<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.rchunks_exact(chunk_size))
        }
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the end
    /// of the slice.
    ///
    /// Equivalent to [`[T]::rchunks_exact_mut`].
    ///
    /// [`[T]::rchunks_exact_mut`]: `slice::rchunks_exact_mut`
    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, T> {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(
                self.0.rchunks_exact_mut(chunk_size),
            )
        }
    }

    /// Returns an iterator over the slice producing non-overlapping runs
    /// of elements using the predicate to separate them.
    ///
    /// Equivalent to [`[T]::chunk_by`].
    ///
    /// [`[T]::chunk_by`]: `slice::chunk_by`
    pub fn chunk_by<F>(&self, pred: F) -> ChunkBy<'_, T, F>
    where
        F: FnMut(&T, &T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.chunk_by(pred)) }
    }

    /// Returns an iterator over the slice producing non-overlapping mutable
    /// runs of elements using the predicate to separate them.
    ///
    /// Equivalent to [`[T]::chunk_by_mut`].
    ///
    /// [`[T]::chunk_by_mut`]: `slice::chunk_by_mut`
    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, T, F>
    where
        F: FnMut(&T, &T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(self.0.chunk_by_mut(pred))
        }
    }

    /// Divides one slice into two at an index.
    ///
    /// Equivalent to [`[T]::split_at`](slice::split_at).
    pub const fn split_at(&self, mid: usize) -> (&Self, &Self) {
        let (left, right) = self.0.split_at(mid);
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            (
                Self::from_slice_unchecked(left),
                Self::from_slice_unchecked(right),
            )
        }
    }

    /// Divides one mutable slice into two at an index.
    ///
    /// Equivalent to [`[T]::split_at_mut`](slice::split_at_mut).
    pub const fn split_at_mut(&mut self, mid: usize) -> (&mut Self, &mut Self) {
        let (left, right) = self.0.split_at_mut(mid);
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            (
                Self::from_slice_unchecked_mut(left),
                Self::from_slice_unchecked_mut(right),
            )
        }
    }

    /// Divides one slice into two at an index, without doing bounds checking.
    ///
    /// Equivalent to [`[T]::split_at_unchecked`](slice::split_at_unchecked).
    ///
    /// # Safety
    ///
    /// `mid` must be safe to use in [`[T]::split_at_unchecked`].
    ///
    /// [`[T]::split_at_unchecked`]: `slice::split_at_unchecked`
    pub const unsafe fn split_at_unchecked(&self, mid: usize) -> (&Self, &Self) {
        // SAFETY: The safety contract is upheld by the caller.
        let (left, right) = unsafe { self.0.split_at_unchecked(mid) };
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            (
                Self::from_slice_unchecked(left),
                Self::from_slice_unchecked(right),
            )
        }
    }

    /// Divides one mutable slice into two at an index, without doing bounds checking.
    ///
    /// Equivalent to [`[T]::split_at_mut_unchecked`](slice::split_at_mut_unchecked).
    ///
    /// # Safety
    ///
    /// `mid` must be safe to use in [`[T]::split_at_mut_unchecked`].
    ///
    /// [`[T]::split_at_mut_unchecked`]: `slice::split_at_mut_unchecked`
    pub const unsafe fn split_at_mut_unchecked(&mut self, mid: usize) -> (&mut Self, &mut Self) {
        // SAFETY: The safety contract is upheld by the caller.
        let (left, right) = unsafe { self.0.split_at_mut_unchecked(mid) };
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            (
                Self::from_slice_unchecked_mut(left),
                Self::from_slice_unchecked_mut(right),
            )
        }
    }

    /// Divides one slice into two at an index, returning `None` if the slice is
    /// too short.
    ///
    /// Equivalent to [`[T]::split_at_checked`](slice::split_at_checked).
    pub const fn split_at_checked(&self, mid: usize) -> Option<(&Self, &Self)> {
        let Some((left, right)) = self.0.split_at_checked(mid) else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            Some((
                Self::from_slice_unchecked(left),
                Self::from_slice_unchecked(right),
            ))
        }
    }

    /// Divides one mutable slice into two at an index, returning `None` if the
    /// slice is too short.
    ///
    /// Equivalent to [`[T]::split_at_mut_checked`](slice::split_at_mut_checked).
    pub const fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut Self, &mut Self)> {
        let Some((left, right)) = self.0.split_at_mut_checked(mid) else {
            return None;
        };
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            Some((
                Self::from_slice_unchecked_mut(left),
                Self::from_slice_unchecked_mut(right),
            ))
        }
    }

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`.
    ///
    /// Equivalent to [`[T]::split`].
    ///
    /// [`[T]::split`]: `slice::split`
    pub fn split<F>(&self, pred: F) -> Split<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.split(pred)) }
    }

    /// Returns an iterator over mutable subslices separated by elements that
    /// match `pred`.
    ///
    /// Equivalent to [`[T]::split_mut`].
    ///
    /// [`[T]::split_mut`]: `slice::split_mut`
    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(self.0.split_mut(pred))
        }
    }

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`.
    ///
    /// Equivalent to [`[T]::split_inclusive`].
    ///
    /// [`[T]::split_inclusive`]: `slice::split_inclusive`
    pub fn split_inclusive<F>(&self, pred: F) -> SplitInclusive<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.split_inclusive(pred))
        }
    }

    /// Returns an iterator over mutable subslices separated by elements that
    /// match `pred`.
    ///
    /// Equivalent to [`[T]::split_inclusive_mut`].
    ///
    /// [`[T]::split_inclusive_mut`]: `slice::split_inclusive_mut`
    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(
                self.0.split_inclusive_mut(pred),
            )
        }
    }

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`, starting at the end of the slice and working backwards.
    ///
    /// Equivalent to [`[T]::rsplit`].
    ///
    /// [`[T]::rsplit`]: `slice::rsplit`
    pub fn rsplit<F>(&self, pred: F) -> RSplit<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.rsplit(pred)) }
    }

    /// Returns an iterator over mutable subslices separated by elements that
    /// match `pred`, starting at the end of the slice and working
    /// backwards.
    ///
    /// Equivalent to [`[T]::rsplit_mut`].
    ///
    /// [`[T]::rsplit_mut`]: `slice::rsplit_mut`
    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(self.0.rsplit_mut(pred))
        }
    }

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`, limited to returning at most `n` items.
    ///
    /// Equivalent to [`[T]::splitn`].
    ///
    /// [`[T]::splitn`]: `slice::splitn`
    pub fn splitn<F>(&self, n: usize, pred: F) -> SplitN<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.splitn(n, pred)) }
    }

    /// Returns an iterator over mutable subslices separated by elements that match
    /// `pred`, limited to returning at most `n` items.
    ///
    /// Equivalent to [`[T]::splitn_mut`].
    ///
    /// [`[T]::splitn_mut`]: `slice::splitn_mut`
    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(self.0.splitn_mut(n, pred))
        }
    }

    /// Returns an iterator over subslices separated by elements that match
    /// `pred` limited to returning at most `n` items.
    ///
    /// Equivalent to [`[T]::rsplitn`].
    ///
    /// [`[T]::rsplitn`]: `slice::rsplitn`
    pub fn rsplitn<F>(&self, n: usize, pred: F) -> RSplitN<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe { UniqueEntitySliceIter::from_slice_iterator_unchecked(self.0.rsplitn(n, pred)) }
    }

    /// Returns an iterator over subslices separated by elements that match
    /// `pred` limited to returning at most `n` items.
    ///
    /// Equivalent to [`[T]::rsplitn_mut`].
    ///
    /// [`[T]::rsplitn_mut`]: `slice::rsplitn_mut`
    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        // SAFETY: Any subslice of a unique slice is also unique.
        unsafe {
            UniqueEntitySliceIterMut::from_mut_slice_iterator_unchecked(self.0.rsplitn_mut(n, pred))
        }
    }

    /// Sorts the slice **without** preserving the initial order of equal elements.
    ///
    /// Equivalent to [`[T]::sort_unstable`](slice::sort_unstable).
    pub fn sort_unstable(&mut self)
    where
        T: Ord,
    {
        self.0.sort_unstable();
    }

    /// Sorts the slice with a comparison function, **without** preserving the initial order of
    /// equal elements.
    ///
    /// Equivalent to [`[T]::sort_unstable_by`](slice::sort_unstable_by).
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.0.sort_unstable_by(compare);
    }

    /// Sorts the slice with a key extraction function, **without** preserving the initial order of
    /// equal elements.
    ///
    /// Equivalent to [`[T]::sort_unstable_by_key`](slice::sort_unstable_by_key).
    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.0.sort_unstable_by_key(f);
    }

    /// Rotates the slice in-place such that the first `mid` elements of the
    /// slice move to the end while the last `self.len() - mid` elements move to
    /// the front.
    ///
    /// Equivalent to [`[T]::rotate_left`](slice::rotate_left).
    pub fn rotate_left(&mut self, mid: usize) {
        self.0.rotate_left(mid);
    }

    /// Rotates the slice in-place such that the first `self.len() - k`
    /// elements of the slice move to the end while the last `k` elements move
    /// to the front.
    ///
    /// Equivalent to [`[T]::rotate_right`](slice::rotate_right).
    pub fn rotate_right(&mut self, mid: usize) {
        self.0.rotate_right(mid);
    }

    /// Sorts the slice, preserving initial order of equal elements.
    ///
    /// Equivalent to [`[T]::sort`](slice::sort()).
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.0.sort();
    }

    /// Sorts the slice with a comparison function, preserving initial order of equal elements.
    ///
    /// Equivalent to [`[T]::sort_by`](slice::sort_by).
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.0.sort_by(compare);
    }

    /// Sorts the slice with a key extraction function, preserving initial order of equal elements.
    ///
    /// Equivalent to [`[T]::sort_by_key`](slice::sort_by_key).
    pub fn sort_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.0.sort_by_key(f);
    }

    // Sorts the slice with a key extraction function, preserving initial order of equal elements.
    ///
    /// Equivalent to [`[T]::sort_by_cached_key`](slice::sort_by_cached_key).
    pub fn sort_by_cached_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.0.sort_by_cached_key(f);
    }

    /// Copies self into a new `UniqueEntityVec`.
    pub fn to_vec(&self) -> UniqueEntityVec<T>
    where
        T: Clone,
    {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityVec::from_vec_unchecked(self.0.to_vec()) }
    }

    /// Converts `self` into a vector without clones or allocation.
    ///
    /// Equivalent to [`[T]::into_vec`](slice::into_vec).
    pub fn into_vec(self: Box<Self>) -> UniqueEntityVec<T> {
        // SAFETY:
        // This matches the implementation of `slice::into_vec`.
        // All elements in the original slice are unique.
        unsafe {
            let len = self.len();
            let vec = Vec::from_raw_parts(Box::into_raw(self).cast::<T>(), len, len);
            UniqueEntityVec::from_vec_unchecked(vec)
        }
    }
}

/// Converts a reference to T into a slice of length 1 (without copying).
pub const fn from_ref<T: TrustedEntityBorrow>(s: &T) -> &UniqueEntitySlice<T> {
    // SAFETY: A slice with a length of 1 is always unique.
    unsafe { UniqueEntitySlice::from_slice_unchecked(slice::from_ref(s)) }
}

/// Converts a reference to T into a slice of length 1 (without copying).
pub const fn from_mut<T: TrustedEntityBorrow>(s: &mut T) -> &mut UniqueEntitySlice<T> {
    // SAFETY: A slice with a length of 1 is always unique.
    unsafe { UniqueEntitySlice::from_slice_unchecked_mut(slice::from_mut(s)) }
}

/// Forms a slice from a pointer and a length.
///
/// Equivalent to [`slice::from_raw_parts`].
///
/// # Safety
///
/// [`slice::from_raw_parts`] must be safe to call with `data` and `len`.
/// Additionally, all elements in the resulting slice must be unique.
pub const unsafe fn from_raw_parts<'a, T: TrustedEntityBorrow>(
    data: *const T,
    len: usize,
) -> &'a UniqueEntitySlice<T> {
    // SAFETY: The safety contract is upheld by the caller.
    unsafe { UniqueEntitySlice::from_slice_unchecked(slice::from_raw_parts(data, len)) }
}

/// Performs the same functionality as [`from_raw_parts`], except that a mutable slice is returned.
///
/// Equivalent to [`slice::from_raw_parts_mut`].
///
/// # Safety
///
/// [`slice::from_raw_parts_mut`] must be safe to call with `data` and `len`.
/// Additionally, all elements in the resulting slice must be unique.
pub const unsafe fn from_raw_parts_mut<'a, T: TrustedEntityBorrow>(
    data: *mut T,
    len: usize,
) -> &'a mut UniqueEntitySlice<T> {
    // SAFETY: The safety contract is upheld by the caller.
    unsafe { UniqueEntitySlice::from_slice_unchecked_mut(slice::from_raw_parts_mut(data, len)) }
}

/// Casts a slice of entity slices to a slice of [`UniqueEntitySlice`]s.
///
/// # Safety
///
/// All elements in each of the casted slices must be unique.
pub unsafe fn cast_slice_of_unique_entity_slice<'a, 'b, T: TrustedEntityBorrow + 'a>(
    slice: &'b [&'a [T]],
) -> &'b [&'a UniqueEntitySlice<T>] {
    // SAFETY: All elements in the original iterator are unique slices.
    unsafe { &*(ptr::from_ref(slice) as *const [&UniqueEntitySlice<T>]) }
}

/// Casts a mutable slice of entity slices to a slice of [`UniqueEntitySlice`]s.
///
/// # Safety
///
/// All elements in each of the casted slices must be unique.
pub unsafe fn cast_slice_of_unique_entity_slice_mut<'a, 'b, T: TrustedEntityBorrow + 'a>(
    slice: &'b mut [&'a [T]],
) -> &'b mut [&'a UniqueEntitySlice<T>] {
    // SAFETY: All elements in the original iterator are unique slices.
    unsafe { &mut *(ptr::from_mut(slice) as *mut [&UniqueEntitySlice<T>]) }
}

/// Casts a mutable slice of mutable entity slices to a slice of mutable [`UniqueEntitySlice`]s.
///
/// # Safety
///
/// All elements in each of the casted slices must be unique.
pub unsafe fn cast_slice_of_mut_unique_entity_slice_mut<'a, 'b, T: TrustedEntityBorrow + 'a>(
    slice: &'b mut [&'a mut [T]],
) -> &'b mut [&'a mut UniqueEntitySlice<T>] {
    // SAFETY: All elements in the original iterator are unique slices.
    unsafe { &mut *(ptr::from_mut(slice) as *mut [&mut UniqueEntitySlice<T>]) }
}

impl<'a, T: TrustedEntityBorrow> IntoIterator for &'a UniqueEntitySlice<T> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: TrustedEntityBorrow> IntoIterator for &'a Box<UniqueEntitySlice<T>> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: TrustedEntityBorrow> IntoIterator for Box<UniqueEntitySlice<T>> {
    type Item = T;

    type IntoIter = unique_vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<T: TrustedEntityBorrow> Deref for UniqueEntitySlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TrustedEntityBorrow> AsRef<[T]> for UniqueEntitySlice<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T: TrustedEntityBorrow> AsRef<Self> for UniqueEntitySlice<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T: TrustedEntityBorrow> AsMut<Self> for UniqueEntitySlice<T> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<T: TrustedEntityBorrow> Borrow<[T]> for UniqueEntitySlice<T> {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T: TrustedEntityBorrow + Clone> Clone for Box<UniqueEntitySlice<T>> {
    fn clone(&self) -> Self {
        self.to_vec().into_boxed_slice()
    }
}

impl<T: TrustedEntityBorrow> Default for &UniqueEntitySlice<T> {
    fn default() -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(Default::default()) }
    }
}

impl<T: TrustedEntityBorrow> Default for &mut UniqueEntitySlice<T> {
    fn default() -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(Default::default()) }
    }
}

impl<T: TrustedEntityBorrow> Default for Box<UniqueEntitySlice<T>> {
    fn default() -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_boxed_slice_unchecked(Default::default()) }
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&UniqueEntitySlice<T>> for Box<UniqueEntitySlice<T>> {
    fn from(value: &UniqueEntitySlice<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_boxed_slice_unchecked(value.0.into()) }
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&UniqueEntitySlice<T>> for Arc<UniqueEntitySlice<T>> {
    fn from(value: &UniqueEntitySlice<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_arc_slice_unchecked(value.0.into()) }
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&UniqueEntitySlice<T>> for Rc<UniqueEntitySlice<T>> {
    fn from(value: &UniqueEntitySlice<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_rc_slice_unchecked(value.0.into()) }
    }
}

impl<'a, T: TrustedEntityBorrow + Clone> From<&'a UniqueEntitySlice<T>>
    for Cow<'a, UniqueEntitySlice<T>>
{
    fn from(value: &'a UniqueEntitySlice<T>) -> Self {
        Cow::Borrowed(value)
    }
}

impl<T: TrustedEntityBorrow + Clone, const N: usize> From<UniqueEntityArray<T, N>>
    for Box<UniqueEntitySlice<T>>
{
    fn from(value: UniqueEntityArray<T, N>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_boxed_slice_unchecked(Box::new(value.into_inner())) }
    }
}

impl<'a, T: TrustedEntityBorrow + Clone> From<Cow<'a, UniqueEntitySlice<T>>>
    for Box<UniqueEntitySlice<T>>
{
    fn from(value: Cow<'a, UniqueEntitySlice<T>>) -> Self {
        match value {
            Cow::Borrowed(slice) => Box::from(slice),
            Cow::Owned(slice) => Box::from(slice),
        }
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Box<UniqueEntitySlice<T>> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        value.into_boxed_slice()
    }
}

impl<T: TrustedEntityBorrow> FromIterator<T> for Box<UniqueEntitySlice<T>> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter()
            .collect::<UniqueEntityVec<T>>()
            .into_boxed_slice()
    }
}

impl<T: TrustedEntityBorrow> FromEntitySetIterator<T> for Box<UniqueEntitySlice<T>> {
    fn from_entity_set_iter<I: EntitySet<Item = T>>(iter: I) -> Self {
        iter.into_iter()
            .collect_set::<UniqueEntityVec<T>>()
            .into_boxed_slice()
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>>
    for &UniqueEntitySlice<T>
{
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>>
    for &mut UniqueEntitySlice<T>
{
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>>
    for UniqueEntitySlice<T>
{
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<&UniqueEntitySlice<U>>
    for [T; N]
{
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: TrustedEntityBorrow> PartialEq<&UniqueEntitySlice<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.eq(&&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U> + Clone, U: TrustedEntityBorrow>
    PartialEq<&UniqueEntitySlice<U>> for Cow<'_, UniqueEntitySlice<T>>
{
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<&UniqueEntitySlice<U>> for Vec<T> {
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<&UniqueEntitySlice<U>> for VecDeque<T> {
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.eq(&&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<&mut UniqueEntitySlice<U>>
    for [T; N]
{
    fn eq(&self, other: &&mut UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: TrustedEntityBorrow> PartialEq<&mut UniqueEntitySlice<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &&mut UniqueEntitySlice<U>) -> bool {
        self.eq(&&**other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U> + Clone, U: TrustedEntityBorrow>
    PartialEq<&mut UniqueEntitySlice<U>> for Cow<'_, UniqueEntitySlice<T>>
{
    fn eq(&self, other: &&mut UniqueEntitySlice<U>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U> + Clone, U: TrustedEntityBorrow>
    PartialEq<UniqueEntityVec<U>> for Cow<'_, UniqueEntitySlice<T>>
{
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<&mut UniqueEntitySlice<U>> for Vec<T> {
    fn eq(&self, other: &&mut UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<&mut UniqueEntitySlice<U>> for VecDeque<T> {
    fn eq(&self, other: &&mut UniqueEntitySlice<U>) -> bool {
        self.eq(&&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntitySlice<U>>
    for [T]
{
    fn eq(&self, other: &UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<UniqueEntitySlice<U>>
    for [T; N]
{
    fn eq(&self, other: &UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntitySlice<U>>
    for Vec<T>
{
    fn eq(&self, other: &UniqueEntitySlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for &UniqueEntitySlice<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for &mut UniqueEntitySlice<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for UniqueEntitySlice<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow, const N: usize>
    PartialEq<UniqueEntityArray<U, N>> for &UniqueEntitySlice<T>
{
    fn eq(&self, other: &UniqueEntityArray<U, N>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow, const N: usize>
    PartialEq<UniqueEntityArray<U, N>> for &mut UniqueEntitySlice<T>
{
    fn eq(&self, other: &UniqueEntityArray<U, N>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow, const N: usize>
    PartialEq<UniqueEntityArray<U, N>> for UniqueEntitySlice<T>
{
    fn eq(&self, other: &UniqueEntityArray<U, N>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<Vec<U>> for &UniqueEntitySlice<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<Vec<U>> for &mut UniqueEntitySlice<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<Vec<U>> for UniqueEntitySlice<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + Clone> ToOwned for UniqueEntitySlice<T> {
    type Owned = UniqueEntityVec<T>;

    fn to_owned(&self) -> Self::Owned {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityVec::from_vec_unchecked(self.0.to_owned()) }
    }
}

impl<'a, T: TrustedEntityBorrow + Copy, const N: usize> TryFrom<&'a UniqueEntitySlice<T>>
    for &'a UniqueEntityArray<T, N>
{
    type Error = TryFromSliceError;

    fn try_from(value: &'a UniqueEntitySlice<T>) -> Result<Self, Self::Error> {
        <&[T; N]>::try_from(&value.0).map(|array|
                // SAFETY: All elements in the original slice are unique.
                unsafe { UniqueEntityArray::from_array_ref_unchecked(array) })
    }
}

impl<T: TrustedEntityBorrow + Copy, const N: usize> TryFrom<&UniqueEntitySlice<T>>
    for UniqueEntityArray<T, N>
{
    type Error = TryFromSliceError;

    fn try_from(value: &UniqueEntitySlice<T>) -> Result<Self, Self::Error> {
        <&Self>::try_from(value).copied()
    }
}

impl<T: TrustedEntityBorrow + Copy, const N: usize> TryFrom<&mut UniqueEntitySlice<T>>
    for UniqueEntityArray<T, N>
{
    type Error = TryFromSliceError;

    fn try_from(value: &mut UniqueEntitySlice<T>) -> Result<Self, Self::Error> {
        <Self>::try_from(&*value)
    }
}

impl<T: TrustedEntityBorrow> Index<(Bound<usize>, Bound<usize>)> for UniqueEntitySlice<T> {
    type Output = Self;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<Range<usize>> for UniqueEntitySlice<T> {
    type Output = Self;
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeFrom<usize>> for UniqueEntitySlice<T> {
    type Output = Self;
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeFull> for UniqueEntitySlice<T> {
    type Output = Self;
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeInclusive<usize>> for UniqueEntitySlice<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeTo<usize>> for UniqueEntitySlice<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeToInclusive<usize>> for UniqueEntitySlice<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<usize> for UniqueEntitySlice<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.0[index]
    }
}

impl<T: TrustedEntityBorrow> IndexMut<(Bound<usize>, Bound<usize>)> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<Range<usize>> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeFrom<usize>> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeFull> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeInclusive<usize>> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeTo<usize>> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeToInclusive<usize>> for UniqueEntitySlice<T> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

/// Immutable slice iterator.
///
/// This struct is created by [`iter`] method on [`UniqueEntitySlice`] and
/// the [`IntoIterator`] impls on it and [`UniqueEntityVec`].
///
/// [`iter`]: `UniqueEntitySlice::iter`
pub type Iter<'a, T> = UniqueEntityIter<slice::Iter<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntityIter<slice::Iter<'a, T>> {
    /// Views the underlying data as a subslice of the original data.
    ///
    /// Equivalent to [`slice::Iter::as_slice`].
    pub fn as_slice(&self) -> &'a UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }
}

/// Mutable slice iterator.
pub type IterMut<'a, T> = UniqueEntityIter<slice::IterMut<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntityIter<slice::IterMut<'a, T>> {
    /// Views the underlying data as a mutable subslice of the original data.
    ///
    /// Equivalent to [`slice::IterMut::into_slice`].
    pub fn into_slice(self) -> &'a mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.into_inner().into_slice()) }
    }

    /// Views the underlying data as a subslice of the original data.
    ///
    /// Equivalent to [`slice::IterMut::as_slice`].
    pub fn as_slice(&self) -> &UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }
}

/// An iterator that yields `&UniqueEntitySlice`. Note that an entity may appear
/// in multiple slices, depending on the wrapped iterator.
#[derive(Debug)]
pub struct UniqueEntitySliceIter<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a [T]>> {
    pub(crate) iter: I,
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a [T]>> UniqueEntitySliceIter<'a, T, I> {
    /// Constructs a [`UniqueEntitySliceIter`] from a slice iterator unsafely.
    ///
    /// # Safety
    ///
    /// All elements in each of the slices must be unique.
    pub unsafe fn from_slice_iterator_unchecked(iter: I) -> Self {
        Self { iter }
    }

    /// Returns the inner `I`.
    pub fn into_inner(self) -> I {
        self.iter
    }

    /// Returns a reference to the inner `I`.
    pub fn as_inner(&self) -> &I {
        &self.iter
    }

    /// Returns a mutable reference to the inner `I`.
    ///
    /// # Safety
    ///
    /// `self` must always contain an iterator that yields unique elements,
    /// even while this reference is live.
    pub unsafe fn as_mut_inner(&mut self) -> &mut I {
        &mut self.iter
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a [T]>> Iterator
    for UniqueEntitySliceIter<'a, T, I>
{
    type Item = &'a UniqueEntitySlice<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|slice|
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { UniqueEntitySlice::from_slice_unchecked(slice) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: ExactSizeIterator<Item = &'a [T]>> ExactSizeIterator
    for UniqueEntitySliceIter<'a, T, I>
{
}

impl<'a, T: TrustedEntityBorrow + 'a, I: DoubleEndedIterator<Item = &'a [T]>> DoubleEndedIterator
    for UniqueEntitySliceIter<'a, T, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|slice|
            // SAFETY: All elements in the original iterator are unique slices.
            unsafe { UniqueEntitySlice::from_slice_unchecked(slice) })
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: FusedIterator<Item = &'a [T]>> FusedIterator
    for UniqueEntitySliceIter<'a, T, I>
{
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a [T]> + AsRef<[&'a [T]]>>
    AsRef<[&'a UniqueEntitySlice<T>]> for UniqueEntitySliceIter<'a, T, I>
{
    fn as_ref(&self) -> &[&'a UniqueEntitySlice<T>] {
        // SAFETY:
        unsafe { cast_slice_of_unique_entity_slice(self.iter.as_ref()) }
    }
}

/// An iterator over overlapping subslices of length `size`.
///
/// This struct is created by [`UniqueEntitySlice::windows`].
pub type Windows<'a, T> = UniqueEntitySliceIter<'a, T, slice::Windows<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the beginning of the slice.
///
/// This struct is created by [`UniqueEntitySlice::chunks`].
pub type Chunks<'a, T> = UniqueEntitySliceIter<'a, T, slice::Chunks<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the beginning of the slice.
///
/// This struct is created by [`UniqueEntitySlice::chunks_exact`].
pub type ChunksExact<'a, T> = UniqueEntitySliceIter<'a, T, slice::ChunksExact<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntitySliceIter<'a, T, slice::ChunksExact<'a, T>> {
    /// Returns the remainder of the original slice that is not going to be
    /// returned by the iterator.
    ///
    /// Equivalent to [`slice::ChunksExact::remainder`].
    pub fn remainder(&self) -> &'a UniqueEntitySlice<T> {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.iter.remainder()) }
    }
}

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rchunks`].
pub type RChunks<'a, T> = UniqueEntitySliceIter<'a, T, slice::RChunks<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rchunks_exact`].
pub type RChunksExact<'a, T> = UniqueEntitySliceIter<'a, T, slice::RChunksExact<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntitySliceIter<'a, T, slice::RChunksExact<'a, T>> {
    /// Returns the remainder of the original slice that is not going to be
    /// returned by the iterator.
    ///
    /// Equivalent to [`slice::RChunksExact::remainder`].
    pub fn remainder(&self) -> &'a UniqueEntitySlice<T> {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.iter.remainder()) }
    }
}

/// An iterator over slice in (non-overlapping) chunks separated by a predicate.
///
/// This struct is created by [`UniqueEntitySlice::chunk_by`].
pub type ChunkBy<'a, T, P> = UniqueEntitySliceIter<'a, T, slice::ChunkBy<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function.
///
/// This struct is created by [`UniqueEntitySlice::split`].
pub type Split<'a, T, P> = UniqueEntitySliceIter<'a, T, slice::Split<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function.
///
/// This struct is created by [`UniqueEntitySlice::split_inclusive`].
pub type SplitInclusive<'a, T, P> = UniqueEntitySliceIter<'a, T, slice::SplitInclusive<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function, starting from the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rsplit`].
pub type RSplit<'a, T, P> = UniqueEntitySliceIter<'a, T, slice::RSplit<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function, limited to a given number of splits.
///
/// This struct is created by [`UniqueEntitySlice::splitn`].
pub type SplitN<'a, T, P> = UniqueEntitySliceIter<'a, T, slice::SplitN<'a, T, P>>;

/// An iterator over subslices separated by elements that match a
/// predicate function, limited to a given number of splits, starting
/// from the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rsplitn`].
pub type RSplitN<'a, T, P> = UniqueEntitySliceIter<'a, T, slice::RSplitN<'a, T, P>>;

/// An iterator that yields `&mut UniqueEntitySlice`. Note that an entity may appear
/// in multiple slices, depending on the wrapped iterator.
#[derive(Debug)]
pub struct UniqueEntitySliceIterMut<
    'a,
    T: TrustedEntityBorrow + 'a,
    I: Iterator<Item = &'a mut [T]>,
> {
    pub(crate) iter: I,
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a mut [T]>>
    UniqueEntitySliceIterMut<'a, T, I>
{
    /// Constructs a [`UniqueEntitySliceIterMut`] from a mutable slice iterator unsafely.
    ///
    /// # Safety
    ///
    /// All elements in each of the slices must be unique.
    pub unsafe fn from_mut_slice_iterator_unchecked(iter: I) -> Self {
        Self { iter }
    }

    /// Returns the inner `I`.
    pub fn into_inner(self) -> I {
        self.iter
    }

    /// Returns a reference to the inner `I`.
    pub fn as_inner(&self) -> &I {
        &self.iter
    }

    /// Returns a mutable reference to the inner `I`.
    ///
    /// # Safety
    ///
    /// `self` must always contain an iterator that yields unique elements,
    /// even while this reference is live.
    pub unsafe fn as_mut_inner(&mut self) -> &mut I {
        &mut self.iter
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a mut [T]>> Iterator
    for UniqueEntitySliceIterMut<'a, T, I>
{
    type Item = &'a mut UniqueEntitySlice<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|slice|
            // SAFETY: All elements in the original iterator are unique slices.
            unsafe { UniqueEntitySlice::from_slice_unchecked_mut(slice) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: ExactSizeIterator<Item = &'a mut [T]>> ExactSizeIterator
    for UniqueEntitySliceIterMut<'a, T, I>
{
}

impl<'a, T: TrustedEntityBorrow + 'a, I: DoubleEndedIterator<Item = &'a mut [T]>>
    DoubleEndedIterator for UniqueEntitySliceIterMut<'a, T, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|slice|
            // SAFETY: All elements in the original iterator are unique slices.
            unsafe { UniqueEntitySlice::from_slice_unchecked_mut(slice) })
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: FusedIterator<Item = &'a mut [T]>> FusedIterator
    for UniqueEntitySliceIterMut<'a, T, I>
{
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a mut [T]> + AsRef<[&'a [T]]>>
    AsRef<[&'a UniqueEntitySlice<T>]> for UniqueEntitySliceIterMut<'a, T, I>
{
    fn as_ref(&self) -> &[&'a UniqueEntitySlice<T>] {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { cast_slice_of_unique_entity_slice(self.iter.as_ref()) }
    }
}

impl<'a, T: TrustedEntityBorrow + 'a, I: Iterator<Item = &'a mut [T]> + AsMut<[&'a mut [T]]>>
    AsMut<[&'a mut UniqueEntitySlice<T>]> for UniqueEntitySliceIterMut<'a, T, I>
{
    fn as_mut(&mut self) -> &mut [&'a mut UniqueEntitySlice<T>] {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { cast_slice_of_mut_unique_entity_slice_mut(self.iter.as_mut()) }
    }
}

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the beginning of the slice.
///
/// This struct is created by [`UniqueEntitySlice::chunks_mut`].
pub type ChunksMut<'a, T> = UniqueEntitySliceIterMut<'a, T, slice::ChunksMut<'a, T>>;

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the beginning of the slice.
///
/// This struct is created by [`UniqueEntitySlice::chunks_exact_mut`].
pub type ChunksExactMut<'a, T> = UniqueEntitySliceIterMut<'a, T, slice::ChunksExactMut<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntitySliceIterMut<'a, T, slice::ChunksExactMut<'a, T>> {
    /// Returns the remainder of the original slice that is not going to be
    /// returned by the iterator.
    ///
    /// Equivalent to [`slice::ChunksExactMut::into_remainder`].
    pub fn into_remainder(self) -> &'a mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.iter.into_remainder()) }
    }
}

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rchunks_mut`].
pub type RChunksMut<'a, T> = UniqueEntitySliceIterMut<'a, T, slice::RChunksMut<'a, T>>;

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rchunks_exact_mut`].
pub type RChunksExactMut<'a, T> = UniqueEntitySliceIterMut<'a, T, slice::RChunksExactMut<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntitySliceIterMut<'a, T, slice::RChunksExactMut<'a, T>> {
    /// Returns the remainder of the original slice that is not going to be
    /// returned by the iterator.
    ///
    /// Equivalent to [`slice::RChunksExactMut::into_remainder`].
    pub fn into_remainder(self) -> &'a mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.iter.into_remainder()) }
    }
}

/// An iterator over slice in (non-overlapping) mutable chunks separated
/// by a predicate.
///
/// This struct is created by [`UniqueEntitySlice::chunk_by_mut`].
pub type ChunkByMut<'a, T, P> = UniqueEntitySliceIterMut<'a, T, slice::ChunkByMut<'a, T, P>>;

/// An iterator over the mutable subslices of the vector which are separated
/// by elements that match `pred`.
///
/// This struct is created by [`UniqueEntitySlice::split_mut`].
pub type SplitMut<'a, T, P> = UniqueEntitySliceIterMut<'a, T, slice::SplitMut<'a, T, P>>;

/// An iterator over the mutable subslices of the vector which are separated
/// by elements that match `pred`. Unlike `SplitMut`, it contains the matched
/// parts in the ends of the subslices.
///
/// This struct is created by [`UniqueEntitySlice::split_inclusive_mut`].
pub type SplitInclusiveMut<'a, T, P> =
    UniqueEntitySliceIterMut<'a, T, slice::SplitInclusiveMut<'a, T, P>>;

/// An iterator over the subslices of the vector which are separated
/// by elements that match `pred`, starting from the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rsplit_mut`].
pub type RSplitMut<'a, T, P> = UniqueEntitySliceIterMut<'a, T, slice::RSplitMut<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function, limited to a given number of splits.
///
/// This struct is created by [`UniqueEntitySlice::splitn_mut`].
pub type SplitNMut<'a, T, P> = UniqueEntitySliceIterMut<'a, T, slice::SplitNMut<'a, T, P>>;

/// An iterator over subslices separated by elements that match a
/// predicate function, limited to a given number of splits, starting
/// from the end of the slice.
///
/// This struct is created by [`UniqueEntitySlice::rsplitn_mut`].
pub type RSplitNMut<'a, T, P> = UniqueEntitySliceIterMut<'a, T, slice::RSplitNMut<'a, T, P>>;
