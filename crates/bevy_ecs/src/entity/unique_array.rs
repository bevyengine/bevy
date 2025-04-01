//! A wrapper around entity arrays with a uniqueness invariant.

use core::{
    array,
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    ops::{
        Bound, Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive,
        RangeTo, RangeToInclusive,
    },
    ptr,
};

use alloc::{
    boxed::Box,
    collections::{BTreeSet, BinaryHeap, LinkedList, VecDeque},
    rc::Rc,
    vec::Vec,
};

use bevy_platform_support::sync::Arc;

use super::{
    Entity, EntityEquivalent, UniqueEntityIter,
    unique_slice::{self, UniqueEntitySlice},
};

/// An array that contains only unique entities.
///
/// It can be obtained through certain methods on [`UniqueEntitySlice`],
/// and some [`TryFrom`] implementations.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntityArray<const N: usize, T: EntityEquivalent = Entity>([T; N]);

impl<T: EntityEquivalent, const N: usize> UniqueEntityArray<N, T> {
    /// Constructs a `UniqueEntityArray` from a [`[T; N]`] unsafely.
    ///
    /// # Safety
    ///
    /// `array` must contain only unique elements.
    pub const unsafe fn from_array_unchecked(array: [T; N]) -> Self {
        Self(array)
    }

    /// Constructs a `&UniqueEntityArray` from a [`&[T; N]`] unsafely.
    ///
    /// # Safety
    ///
    /// `array` must contain only unique elements.
    pub const unsafe fn from_array_ref_unchecked(array: &[T; N]) -> &Self {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { &*(ptr::from_ref(array).cast()) }
    }

    /// Constructs a `Box<UniqueEntityArray>` from a [`Box<[T; N]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `array` must contain only unique elements.
    pub unsafe fn from_boxed_array_unchecked(array: Box<[T; N]>) -> Box<Self> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Box::from_raw(Box::into_raw(array).cast()) }
    }

    /// Casts `self` into the inner array.
    pub fn into_boxed_inner(self: Box<Self>) -> Box<[T; N]> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Box::from_raw(Box::into_raw(self).cast()) }
    }

    /// Constructs a `Arc<UniqueEntityArray>` from a [`Arc<[T; N]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_arc_array_unchecked(slice: Arc<[T; N]>) -> Arc<Self> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Arc::from_raw(Arc::into_raw(slice).cast()) }
    }

    /// Casts `self` to the inner array.
    pub fn into_arc_inner(this: Arc<Self>) -> Arc<[T; N]> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Arc::from_raw(Arc::into_raw(this).cast()) }
    }

    // Constructs a `Rc<UniqueEntityArray>` from a [`Rc<[T; N]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_rc_array_unchecked(slice: Rc<[T; N]>) -> Rc<Self> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Rc::from_raw(Rc::into_raw(slice).cast()) }
    }

    /// Casts `self` to the inner array.
    pub fn into_rc_inner(self: Rc<Self>) -> Rc<[T; N]> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Rc::from_raw(Rc::into_raw(self).cast()) }
    }

    /// Return the inner array.
    pub fn into_inner(self) -> [T; N] {
        self.0
    }

    /// Returns a reference to the inner array.
    pub fn as_inner(&self) -> &[T; N] {
        &self.0
    }

    /// Returns a slice containing the entire array. Equivalent to `&s[..]`.
    pub const fn as_slice(&self) -> &UniqueEntitySlice<T> {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice containing the entire array. Equivalent to
    /// `&mut s[..]`.
    pub fn as_mut_slice(&mut self) -> &mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.as_mut_slice()) }
    }

    /// Borrows each element and returns an array of references with the same
    /// size as `self`.
    ///
    /// Equivalent to [`[T; N]::as_ref`](array::each_ref).
    pub fn each_ref(&self) -> UniqueEntityArray<N, &T> {
        UniqueEntityArray(self.0.each_ref())
    }
}

impl<T: EntityEquivalent, const N: usize> Deref for UniqueEntityArray<N, T> {
    type Target = UniqueEntitySlice<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(&self.0) }
    }
}

impl<T: EntityEquivalent, const N: usize> DerefMut for UniqueEntityArray<N, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(&mut self.0) }
    }
}
impl<T: EntityEquivalent> Default for UniqueEntityArray<0, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'a, T: EntityEquivalent, const N: usize> IntoIterator for &'a UniqueEntityArray<N, T> {
    type Item = &'a T;

    type IntoIter = unique_slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }
}

impl<T: EntityEquivalent, const N: usize> IntoIterator for UniqueEntityArray<N, T> {
    type Item = T;

    type IntoIter = IntoIter<N, T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.into_iter()) }
    }
}

impl<T: EntityEquivalent, const N: usize> AsRef<UniqueEntitySlice<T>> for UniqueEntityArray<N, T> {
    fn as_ref(&self) -> &UniqueEntitySlice<T> {
        self
    }
}

impl<T: EntityEquivalent, const N: usize> AsMut<UniqueEntitySlice<T>> for UniqueEntityArray<N, T> {
    fn as_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        self
    }
}

impl<T: EntityEquivalent, const N: usize> Borrow<UniqueEntitySlice<T>> for UniqueEntityArray<N, T> {
    fn borrow(&self) -> &UniqueEntitySlice<T> {
        self
    }
}

impl<T: EntityEquivalent, const N: usize> BorrowMut<UniqueEntitySlice<T>>
    for UniqueEntityArray<N, T>
{
    fn borrow_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        self
    }
}

impl<T: EntityEquivalent, const N: usize> Index<(Bound<usize>, Bound<usize>)>
    for UniqueEntityArray<N, T>
{
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<Range<usize>> for UniqueEntityArray<N, T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<RangeFrom<usize>> for UniqueEntityArray<N, T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<RangeFull> for UniqueEntityArray<N, T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<RangeInclusive<usize>> for UniqueEntityArray<N, T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<RangeTo<usize>> for UniqueEntityArray<N, T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<RangeToInclusive<usize>>
    for UniqueEntityArray<N, T>
{
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> Index<usize> for UniqueEntityArray<N, T> {
    type Output = T;
    fn index(&self, key: usize) -> &T {
        self.0.index(key)
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<(Bound<usize>, Bound<usize>)>
    for UniqueEntityArray<N, T>
{
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<Range<usize>> for UniqueEntityArray<N, T> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<RangeFrom<usize>> for UniqueEntityArray<N, T> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<RangeFull> for UniqueEntityArray<N, T> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<RangeInclusive<usize>>
    for UniqueEntityArray<N, T>
{
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<RangeTo<usize>> for UniqueEntityArray<N, T> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent, const N: usize> IndexMut<RangeToInclusive<usize>>
    for UniqueEntityArray<N, T>
{
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent + Clone> From<&[T; 1]> for UniqueEntityArray<1, T> {
    fn from(value: &[T; 1]) -> Self {
        Self(value.clone())
    }
}

impl<T: EntityEquivalent + Clone> From<&[T; 0]> for UniqueEntityArray<0, T> {
    fn from(value: &[T; 0]) -> Self {
        Self(value.clone())
    }
}

impl<T: EntityEquivalent + Clone> From<&mut [T; 1]> for UniqueEntityArray<1, T> {
    fn from(value: &mut [T; 1]) -> Self {
        Self(value.clone())
    }
}

impl<T: EntityEquivalent + Clone> From<&mut [T; 0]> for UniqueEntityArray<0, T> {
    fn from(value: &mut [T; 0]) -> Self {
        Self(value.clone())
    }
}

impl<T: EntityEquivalent> From<[T; 1]> for UniqueEntityArray<1, T> {
    fn from(value: [T; 1]) -> Self {
        Self(value)
    }
}

impl<T: EntityEquivalent> From<[T; 0]> for UniqueEntityArray<0, T> {
    fn from(value: [T; 0]) -> Self {
        Self(value)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<1, T>> for (T,) {
    fn from(array: UniqueEntityArray<1, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<2, T>> for (T, T) {
    fn from(array: UniqueEntityArray<2, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<3, T>> for (T, T, T) {
    fn from(array: UniqueEntityArray<3, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<4, T>> for (T, T, T, T) {
    fn from(array: UniqueEntityArray<4, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<5, T>> for (T, T, T, T, T) {
    fn from(array: UniqueEntityArray<5, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<6, T>> for (T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<6, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<7, T>> for (T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<7, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<8, T>> for (T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<8, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<9, T>> for (T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<9, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<10, T>> for (T, T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<10, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<11, T>> for (T, T, T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<11, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent> From<UniqueEntityArray<12, T>> for (T, T, T, T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<12, T>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: EntityEquivalent + Ord, const N: usize> From<UniqueEntityArray<N, T>> for BTreeSet<T> {
    fn from(value: UniqueEntityArray<N, T>) -> Self {
        BTreeSet::from(value.0)
    }
}

impl<T: EntityEquivalent + Ord, const N: usize> From<UniqueEntityArray<N, T>> for BinaryHeap<T> {
    fn from(value: UniqueEntityArray<N, T>) -> Self {
        BinaryHeap::from(value.0)
    }
}

impl<T: EntityEquivalent, const N: usize> From<UniqueEntityArray<N, T>> for LinkedList<T> {
    fn from(value: UniqueEntityArray<N, T>) -> Self {
        LinkedList::from(value.0)
    }
}

impl<T: EntityEquivalent, const N: usize> From<UniqueEntityArray<N, T>> for Vec<T> {
    fn from(value: UniqueEntityArray<N, T>) -> Self {
        Vec::from(value.0)
    }
}

impl<T: EntityEquivalent, const N: usize> From<UniqueEntityArray<N, T>> for VecDeque<T> {
    fn from(value: UniqueEntityArray<N, T>) -> Self {
        VecDeque::from(value.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&UniqueEntitySlice<U>> for UniqueEntityArray<N, T>
{
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.0.eq(&other.as_inner())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<UniqueEntitySlice<U>> for UniqueEntityArray<N, T>
{
    fn eq(&self, other: &UniqueEntitySlice<U>) -> bool {
        self.0.eq(other.as_inner())
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize> PartialEq<&UniqueEntityArray<N, U>>
    for Vec<T>
{
    fn eq(&self, other: &&UniqueEntityArray<N, U>) -> bool {
        self.eq(&other.0)
    }
}
impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize> PartialEq<&UniqueEntityArray<N, U>>
    for VecDeque<T>
{
    fn eq(&self, other: &&UniqueEntityArray<N, U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize> PartialEq<&mut UniqueEntityArray<N, U>>
    for VecDeque<T>
{
    fn eq(&self, other: &&mut UniqueEntityArray<N, U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize> PartialEq<UniqueEntityArray<N, U>>
    for Vec<T>
{
    fn eq(&self, other: &UniqueEntityArray<N, U>) -> bool {
        self.eq(&other.0)
    }
}
impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize> PartialEq<UniqueEntityArray<N, U>>
    for VecDeque<T>
{
    fn eq(&self, other: &UniqueEntityArray<N, U>) -> bool {
        self.eq(&other.0)
    }
}

/// A by-value array iterator.
///
/// Equivalent to [`array::IntoIter`].
pub type IntoIter<const N: usize, T = Entity> = UniqueEntityIter<array::IntoIter<T, N>>;

impl<T: EntityEquivalent, const N: usize> UniqueEntityIter<array::IntoIter<T, N>> {
    /// Returns an immutable slice of all elements that have not been yielded
    /// yet.
    ///
    /// Equivalent to [`array::IntoIter::as_slice`].
    pub fn as_slice(&self) -> &UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }

    /// Returns a mutable slice of all elements that have not been yielded yet.
    ///
    /// Equivalent to [`array::IntoIter::as_mut_slice`].
    pub fn as_mut_slice(&mut self) -> &mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.as_mut_inner().as_mut_slice()) }
    }
}
