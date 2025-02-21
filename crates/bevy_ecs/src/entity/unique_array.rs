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
    sync::Arc,
    vec::Vec,
};

use super::{unique_slice, TrustedEntityBorrow, UniqueEntityIter, UniqueEntitySlice};

/// An array that contains only unique entities.
///
/// It can be obtained through certain methods on [`UniqueEntitySlice`],
/// and some [`TryFrom`] implementations.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntityArray<T: TrustedEntityBorrow, const N: usize>([T; N]);

impl<T: TrustedEntityBorrow, const N: usize> UniqueEntityArray<T, N> {
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
    pub fn into_arc_inner(self: Arc<Self>) -> Arc<[T; N]> {
        // SAFETY: UniqueEntityArray is a transparent wrapper around [T; N].
        unsafe { Arc::from_raw(Arc::into_raw(self).cast()) }
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
    pub fn each_ref(&self) -> UniqueEntityArray<&T, N> {
        UniqueEntityArray(self.0.each_ref())
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Deref for UniqueEntityArray<T, N> {
    type Target = UniqueEntitySlice<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(&self.0) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> DerefMut for UniqueEntityArray<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(&mut self.0) }
    }
}
impl<T: TrustedEntityBorrow> Default for UniqueEntityArray<T, 0> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'a, T: TrustedEntityBorrow, const N: usize> IntoIterator for &'a UniqueEntityArray<T, N> {
    type Item = &'a T;

    type IntoIter = unique_slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IntoIterator for UniqueEntityArray<T, N> {
    type Item = T;

    type IntoIter = IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: All elements in the original array are unique.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.into_iter()) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> AsRef<UniqueEntitySlice<T>>
    for UniqueEntityArray<T, N>
{
    fn as_ref(&self) -> &UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow, const N: usize> AsMut<UniqueEntitySlice<T>>
    for UniqueEntityArray<T, N>
{
    fn as_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Borrow<UniqueEntitySlice<T>>
    for UniqueEntityArray<T, N>
{
    fn borrow(&self) -> &UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow, const N: usize> BorrowMut<UniqueEntitySlice<T>>
    for UniqueEntityArray<T, N>
{
    fn borrow_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<(Bound<usize>, Bound<usize>)>
    for UniqueEntityArray<T, N>
{
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<Range<usize>> for UniqueEntityArray<T, N> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<RangeFrom<usize>> for UniqueEntityArray<T, N> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<RangeFull> for UniqueEntityArray<T, N> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<RangeInclusive<usize>>
    for UniqueEntityArray<T, N>
{
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<RangeTo<usize>> for UniqueEntityArray<T, N> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<RangeToInclusive<usize>>
    for UniqueEntityArray<T, N>
{
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> Index<usize> for UniqueEntityArray<T, N> {
    type Output = T;
    fn index(&self, key: usize) -> &T {
        self.0.index(key)
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<(Bound<usize>, Bound<usize>)>
    for UniqueEntityArray<T, N>
{
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<Range<usize>> for UniqueEntityArray<T, N> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<RangeFrom<usize>>
    for UniqueEntityArray<T, N>
{
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<RangeFull> for UniqueEntityArray<T, N> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<RangeInclusive<usize>>
    for UniqueEntityArray<T, N>
{
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<RangeTo<usize>> for UniqueEntityArray<T, N> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow, const N: usize> IndexMut<RangeToInclusive<usize>>
    for UniqueEntityArray<T, N>
{
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&[T; 1]> for UniqueEntityArray<T, 1> {
    fn from(value: &[T; 1]) -> Self {
        Self(value.clone())
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&[T; 0]> for UniqueEntityArray<T, 0> {
    fn from(value: &[T; 0]) -> Self {
        Self(value.clone())
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&mut [T; 1]> for UniqueEntityArray<T, 1> {
    fn from(value: &mut [T; 1]) -> Self {
        Self(value.clone())
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&mut [T; 0]> for UniqueEntityArray<T, 0> {
    fn from(value: &mut [T; 0]) -> Self {
        Self(value.clone())
    }
}

impl<T: TrustedEntityBorrow> From<[T; 1]> for UniqueEntityArray<T, 1> {
    fn from(value: [T; 1]) -> Self {
        Self(value)
    }
}

impl<T: TrustedEntityBorrow> From<[T; 0]> for UniqueEntityArray<T, 0> {
    fn from(value: [T; 0]) -> Self {
        Self(value)
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 1>> for (T,) {
    fn from(array: UniqueEntityArray<T, 1>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 2>> for (T, T) {
    fn from(array: UniqueEntityArray<T, 2>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 3>> for (T, T, T) {
    fn from(array: UniqueEntityArray<T, 3>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 4>> for (T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 4>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 5>> for (T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 5>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 6>> for (T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 6>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 7>> for (T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 7>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 8>> for (T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 8>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 9>> for (T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 9>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 10>> for (T, T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 10>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 11>> for (T, T, T, T, T, T, T, T, T, T, T) {
    fn from(array: UniqueEntityArray<T, 11>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityArray<T, 12>>
    for (T, T, T, T, T, T, T, T, T, T, T, T)
{
    fn from(array: UniqueEntityArray<T, 12>) -> Self {
        Self::from(array.into_inner())
    }
}

impl<T: TrustedEntityBorrow + Ord, const N: usize> From<UniqueEntityArray<T, N>> for BTreeSet<T> {
    fn from(value: UniqueEntityArray<T, N>) -> Self {
        BTreeSet::from(value.0)
    }
}

impl<T: TrustedEntityBorrow + Ord, const N: usize> From<UniqueEntityArray<T, N>> for BinaryHeap<T> {
    fn from(value: UniqueEntityArray<T, N>) -> Self {
        BinaryHeap::from(value.0)
    }
}

impl<T: TrustedEntityBorrow, const N: usize> From<UniqueEntityArray<T, N>> for LinkedList<T> {
    fn from(value: UniqueEntityArray<T, N>) -> Self {
        LinkedList::from(value.0)
    }
}

impl<T: TrustedEntityBorrow, const N: usize> From<UniqueEntityArray<T, N>> for Vec<T> {
    fn from(value: UniqueEntityArray<T, N>) -> Self {
        Vec::from(value.0)
    }
}

impl<T: TrustedEntityBorrow, const N: usize> From<UniqueEntityArray<T, N>> for VecDeque<T> {
    fn from(value: UniqueEntityArray<T, N>) -> Self {
        VecDeque::from(value.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow, const N: usize>
    PartialEq<&UniqueEntitySlice<U>> for UniqueEntityArray<T, N>
{
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.0.eq(&other.as_inner())
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow, const N: usize>
    PartialEq<UniqueEntitySlice<U>> for UniqueEntityArray<T, N>
{
    fn eq(&self, other: &UniqueEntitySlice<U>) -> bool {
        self.0.eq(other.as_inner())
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<&UniqueEntityArray<U, N>>
    for Vec<T>
{
    fn eq(&self, other: &&UniqueEntityArray<U, N>) -> bool {
        self.eq(&other.0)
    }
}
impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<&UniqueEntityArray<U, N>>
    for VecDeque<T>
{
    fn eq(&self, other: &&UniqueEntityArray<U, N>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize>
    PartialEq<&mut UniqueEntityArray<U, N>> for VecDeque<T>
{
    fn eq(&self, other: &&mut UniqueEntityArray<U, N>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<UniqueEntityArray<U, N>>
    for Vec<T>
{
    fn eq(&self, other: &UniqueEntityArray<U, N>) -> bool {
        self.eq(&other.0)
    }
}
impl<T: PartialEq<U>, U: TrustedEntityBorrow, const N: usize> PartialEq<UniqueEntityArray<U, N>>
    for VecDeque<T>
{
    fn eq(&self, other: &UniqueEntityArray<U, N>) -> bool {
        self.eq(&other.0)
    }
}

pub type IntoIter<T, const N: usize> = UniqueEntityIter<array::IntoIter<T, N>>;

impl<T: TrustedEntityBorrow, const N: usize> UniqueEntityIter<array::IntoIter<T, N>> {
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
