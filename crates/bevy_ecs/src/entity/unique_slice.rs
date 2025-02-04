use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::Debug,
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
    UniqueEntityIter, UniqueEntityVec,
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

    /// Divides one slice into two at an index.
    ///
    /// Equivalent to [`[T]::split_at`](slice::split_at)
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
    /// Equivalent to [`[T]::split_at_mut`](slice::split_at_mut)
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
    /// Equivalent to [`[T]::split_at_unchecked`](slice::split_at_unchecked)
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
/// [`into_iter`]: UniqueEntitySlice::into_iter
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
