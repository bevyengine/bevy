//! A wrapper around entity [`Vec`]s with a uniqueness invariant.

use core::{
    borrow::{Borrow, BorrowMut},
    mem::MaybeUninit,
    ops::{
        Bound, Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull,
        RangeInclusive, RangeTo, RangeToInclusive,
    },
};

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::{BTreeSet, BinaryHeap, TryReserveError, VecDeque},
    rc::Rc,
    vec::{self, Vec},
};

use bevy_platform::sync::Arc;

use super::{
    unique_slice::{self, UniqueEntityEquivalentSlice},
    Entity, EntityEquivalent, EntitySet, FromEntitySetIterator, UniqueEntityEquivalentArray,
    UniqueEntityIter,
};

/// A `Vec` that contains only unique entities.
///
/// "Unique" means that `x != y` holds for any 2 entities in this collection.
/// This is always true when less than 2 entities are present.
///
/// This type is best obtained by its `FromEntitySetIterator` impl, via either
/// `EntityIterator::collect_set` or `UniqueEntityEquivalentVec::from_entity_iter`.
///
/// While this type can be constructed via `Iterator::collect`, doing so is inefficient,
/// and not recommended.
///
/// When `T` is [`Entity`], use the [`UniqueEntityVec`] alias.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntityEquivalentVec<T: EntityEquivalent>(Vec<T>);

/// A `Vec` that contains only unique [`Entity`].
///
/// This is the default case of a [`UniqueEntityEquivalentVec`].
pub type UniqueEntityVec = UniqueEntityEquivalentVec<Entity>;

impl<T: EntityEquivalent> UniqueEntityEquivalentVec<T> {
    /// Constructs a new, empty `UniqueEntityEquivalentVec<T>`.
    ///
    /// Equivalent to [`Vec::new`].
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Constructs a new, empty `UniqueEntityEquivalentVec<T>` with at least the specified capacity.
    ///
    /// Equivalent to [`Vec::with_capacity`]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Creates a `UniqueEntityEquivalentVec<T>` directly from a pointer, a length, and a capacity.
    ///
    /// Equivalent to [`Vec::from_raw_parts`].
    ///
    /// # Safety
    ///
    /// It must be safe to call [`Vec::from_raw_parts`] with these inputs,
    /// and the resulting [`Vec`] must only contain unique elements.
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Self {
        // SAFETY: Caller ensures it's safe to call `Vec::from_raw_parts`
        Self(unsafe { Vec::from_raw_parts(ptr, length, capacity) })
    }

    /// Constructs a `UniqueEntityEquivalentVec` from a [`Vec<T>`] unsafely.
    ///
    /// # Safety
    ///
    /// `vec` must contain only unique elements.
    pub unsafe fn from_vec_unchecked(vec: Vec<T>) -> Self {
        Self(vec)
    }

    /// Returns the inner [`Vec<T>`].
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }

    /// Returns a reference to the inner [`Vec<T>`].
    pub fn as_vec(&self) -> &Vec<T> {
        &self.0
    }

    /// Returns a mutable reference to the inner [`Vec<T>`].
    ///
    /// # Safety
    ///
    /// The elements of this `Vec` must always remain unique, even while
    /// this mutable reference is live.
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<T> {
        &mut self.0
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// Equivalent to [`Vec::capacity`].
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `Vec<T>`.
    ///
    /// Equivalent to [`Vec::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Reserves the minimum capacity for at least `additional` more elements to
    /// be inserted in the given `UniqueEntityEquivalentVec<T>`.
    ///
    /// Equivalent to [`Vec::reserve_exact`].
    pub fn reserve_exact(&mut self, additional: usize) {
        self.0.reserve_exact(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements to be inserted
    /// in the given `Vec<T>`.
    ///
    /// Equivalent to [`Vec::try_reserve`].
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.0.try_reserve(additional)
    }

    /// Tries to reserve the minimum capacity for at least `additional`
    /// elements to be inserted in the given `Vec<T>`.
    ///
    /// Equivalent to [`Vec::try_reserve_exact`].
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.0.try_reserve_exact(additional)
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// Equivalent to [`Vec::shrink_to_fit`].
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Shrinks the capacity of the vector with a lower bound.
    ///
    /// Equivalent to [`Vec::shrink_to`].
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.0.shrink_to(min_capacity);
    }

    /// Converts the vector into `Box<UniqueEntityEquivalentSlice<T>>`.
    pub fn into_boxed_slice(self) -> Box<UniqueEntityEquivalentSlice<T>> {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe {
            UniqueEntityEquivalentSlice::from_boxed_slice_unchecked(self.0.into_boxed_slice())
        }
    }

    /// Extracts a slice containing the entire vector.
    pub fn as_slice(&self) -> &UniqueEntityEquivalentSlice<T> {
        self
    }

    /// Extracts a mutable slice of the entire vector.
    pub fn as_mut_slice(&mut self) -> &mut UniqueEntityEquivalentSlice<T> {
        self
    }

    /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// Equivalent to [`Vec::truncate`].
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    /// Returns a raw pointer to the vector's buffer, or a dangling raw pointer
    /// valid for zero sized reads if the vector didn't allocate.
    ///
    /// Equivalent to [`Vec::as_ptr`].
    pub fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
    /// Returns a raw mutable pointer to the vector's buffer, or a dangling
    /// raw pointer valid for zero sized reads if the vector didn't allocate.
    ///
    /// Equivalent to [`Vec::as_mut_ptr`].
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr()
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// Equivalent to [`Vec::set_len`].
    ///
    /// # Safety
    ///
    /// It must be safe to call [`Vec::set_len`] with these inputs,
    /// and the resulting [`Vec`] must only contain unique elements.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        // SAFETY: Caller ensures it's safe to call `Vec::set_len`
        unsafe { self.0.set_len(new_len) };
    }

    /// Removes an element from the vector and returns it.
    ///
    /// Equivalent to [`Vec::swap_remove`].
    pub fn swap_remove(&mut self, index: usize) -> T {
        self.0.swap_remove(index)
    }

    /// Inserts an element at position `index` within the vector, shifting all
    /// elements after it to the right.
    ///
    /// Equivalent to [`Vec::insert`].
    ///
    /// # Safety
    ///
    /// No `T` contained by `self` may equal `element`.
    pub unsafe fn insert(&mut self, index: usize, element: T) {
        self.0.insert(index, element);
    }

    /// Removes and returns the element at position `index` within the vector,
    /// shifting all elements after it to the left.
    ///
    /// Equivalent to [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        self.0.remove(index)
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Equivalent to [`Vec::retain`].
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.0.retain(f);
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// Equivalent to [`Vec::retain_mut`].
    ///
    /// # Safety
    ///
    /// `self` must only contain unique elements after each individual execution of `f`.
    pub unsafe fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        self.0.retain_mut(f);
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same
    /// key.
    ///
    /// Equivalent to [`Vec::dedup_by_key`].
    ///
    /// # Safety
    ///
    /// `self` must only contain unique elements after each individual execution of `key`.
    pub unsafe fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.0.dedup_by_key(key);
    }

    /// Removes all but the first of consecutive elements in the vector satisfying a given equality
    /// relation.
    ///
    /// Equivalent to [`Vec::dedup_by`].
    ///
    /// # Safety
    ///
    /// `self` must only contain unique elements after each individual execution of `same_bucket`.
    pub unsafe fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        self.0.dedup_by(same_bucket);
    }

    /// Appends an element to the back of a collection.
    ///
    /// Equivalent to [`Vec::push`].
    ///
    /// # Safety
    ///
    /// No `T` contained by `self` may equal `element`.
    pub unsafe fn push(&mut self, value: T) {
        self.0.push(value);
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// Equivalent to [`Vec::append`].
    ///
    /// # Safety
    ///
    /// `other` must contain no elements that equal any element in `self`.
    pub unsafe fn append(&mut self, other: &mut UniqueEntityEquivalentVec<T>) {
        self.0.append(&mut other.0);
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// Equivalent to [`Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Removes the specified range from the vector in bulk, returning all
    /// removed elements as an iterator.
    ///
    /// Equivalent to [`Vec::drain`].
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        // SAFETY: `self` and thus `range` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.drain(range)) }
    }

    /// Clears the vector, removing all values.
    ///
    /// Equivalent to [`Vec::clear`].
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns the number of elements in the vector, also referred to
    /// as its 'length'.
    ///
    /// Equivalent to [`Vec::len`].
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// Equivalent to [`Vec::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Splits the collection into two at the given index.
    ///
    /// Equivalent to [`Vec::split_off`].
    pub fn split_off(&mut self, at: usize) -> Self {
        Self(self.0.split_off(at))
    }

    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`.
    ///
    /// Equivalent to [`Vec::resize_with`].
    ///
    /// # Safety
    ///
    /// `f` must only produce unique `T`, and none of these may equal any `T` in `self`.
    pub unsafe fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        self.0.resize_with(new_len, f);
    }

    /// Consumes and leaks the Vec, returning a mutable reference to the contents, `&'a mut UniqueEntityEquivalentSlice<T>`.
    pub fn leak<'a>(self) -> &'a mut UniqueEntityEquivalentSlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.leak()) }
    }

    /// Returns the remaining spare capacity of the vector as a slice of
    /// [`MaybeUninit<T>`].
    ///
    /// Equivalent to [`Vec::spare_capacity_mut`].
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.0.spare_capacity_mut()
    }

    /// Creates a splicing iterator that replaces the specified range in the vector
    /// with the given `replace_with` iterator and yields the removed items.
    ///
    /// Equivalent to [`Vec::splice`].
    ///
    /// # Safety
    ///
    /// `replace_with` must not yield any elements that equal any elements in `self`,
    /// except for those in `range`.
    pub unsafe fn splice<R, I>(
        &mut self,
        range: R,
        replace_with: I,
    ) -> Splice<'_, <I as IntoIterator>::IntoIter>
    where
        R: RangeBounds<usize>,
        I: EntitySet<Item = T>,
    {
        // SAFETY: `self` and thus `range` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.splice(range, replace_with)) }
    }
}

impl<T: EntityEquivalent> Default for UniqueEntityEquivalentVec<T> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl<T: EntityEquivalent> Deref for UniqueEntityEquivalentVec<T> {
    type Target = UniqueEntityEquivalentSlice<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(&self.0) }
    }
}

impl<T: EntityEquivalent> DerefMut for UniqueEntityEquivalentVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(&mut self.0) }
    }
}

impl<'a, T: EntityEquivalent> IntoIterator for &'a UniqueEntityEquivalentVec<T>
where
    &'a T: EntityEquivalent,
{
    type Item = &'a T;

    type IntoIter = unique_slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: `self` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }
}

impl<T: EntityEquivalent> IntoIterator for UniqueEntityEquivalentVec<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: `self` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.into_iter()) }
    }
}

impl<T: EntityEquivalent> AsMut<Self> for UniqueEntityEquivalentVec<T> {
    fn as_mut(&mut self) -> &mut UniqueEntityEquivalentVec<T> {
        self
    }
}

impl<T: EntityEquivalent> AsMut<UniqueEntityEquivalentSlice<T>> for UniqueEntityEquivalentVec<T> {
    fn as_mut(&mut self) -> &mut UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent> AsRef<Self> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T: EntityEquivalent> AsRef<Vec<T>> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: EntityEquivalent> Borrow<Vec<T>> for UniqueEntityEquivalentVec<T> {
    fn borrow(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: EntityEquivalent> AsRef<[T]> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T: EntityEquivalent> AsRef<UniqueEntityEquivalentSlice<T>> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent> Borrow<[T]> for UniqueEntityEquivalentVec<T> {
    fn borrow(&self) -> &[T] {
        &self.0
    }
}

impl<T: EntityEquivalent> Borrow<UniqueEntityEquivalentSlice<T>> for UniqueEntityEquivalentVec<T> {
    fn borrow(&self) -> &UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent> BorrowMut<UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentVec<T>
{
    fn borrow_mut(&mut self) -> &mut UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<Vec<U>> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<&[U]> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &&[U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<&UniqueEntityEquivalentSlice<U>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<&mut [U]> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &&mut [U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<&mut UniqueEntityEquivalentSlice<U>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<&[U; N]>
    for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(&other.as_inner())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<&mut [U; N]>
    for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&mut [U; N]) -> bool {
        self.0.eq(&**other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&mut UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(other.as_inner())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<[U]> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentSlice<U>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(&**other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(other.as_inner())
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for Vec<T> {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for &[T] {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for &mut [T] {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentVec<U>> for [T]
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for VecDeque<T> {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: EntityEquivalent + Clone> From<&UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &UniqueEntityEquivalentSlice<T>) -> Self {
        value.to_vec()
    }
}

impl<T: EntityEquivalent + Clone> From<&mut UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &mut UniqueEntityEquivalentSlice<T>) -> Self {
        value.to_vec()
    }
}

impl<T: EntityEquivalent> From<Box<UniqueEntityEquivalentSlice<T>>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: Box<UniqueEntityEquivalentSlice<T>>) -> Self {
        value.into_vec()
    }
}

impl<T: EntityEquivalent> From<Cow<'_, UniqueEntityEquivalentSlice<T>>>
    for UniqueEntityEquivalentVec<T>
where
    UniqueEntityEquivalentSlice<T>: ToOwned<Owned = UniqueEntityEquivalentVec<T>>,
{
    fn from(value: Cow<UniqueEntityEquivalentSlice<T>>) -> Self {
        value.into_owned()
    }
}

impl<T: EntityEquivalent + Clone> From<&[T; 1]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &[T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone> From<&[T; 0]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &[T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone> From<&mut [T; 1]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &mut [T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone> From<&mut [T; 0]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &mut [T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent> From<[T; 1]> for UniqueEntityEquivalentVec<T> {
    fn from(value: [T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent> From<[T; 0]> for UniqueEntityEquivalentVec<T> {
    fn from(value: [T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone, const N: usize> From<&UniqueEntityEquivalentArray<T, N>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &UniqueEntityEquivalentArray<T, N>) -> Self {
        Self(Vec::from(value.as_inner().clone()))
    }
}

impl<T: EntityEquivalent + Clone, const N: usize> From<&mut UniqueEntityEquivalentArray<T, N>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &mut UniqueEntityEquivalentArray<T, N>) -> Self {
        Self(Vec::from(value.as_inner().clone()))
    }
}

impl<T: EntityEquivalent, const N: usize> From<UniqueEntityEquivalentArray<T, N>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: UniqueEntityEquivalentArray<T, N>) -> Self {
        Self(Vec::from(value.into_inner()))
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Vec<T> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        value.0
    }
}

impl<'a, T: EntityEquivalent + Clone> From<UniqueEntityEquivalentVec<T>> for Cow<'a, [T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Cow::from(value.0)
    }
}

impl<'a, T: EntityEquivalent + Clone> From<UniqueEntityEquivalentVec<T>>
    for Cow<'a, UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Cow::Owned(value)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Arc<[T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Arc::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>>
    for Arc<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_arc_slice_unchecked(Arc::from(value.0)) }
    }
}

impl<T: EntityEquivalent + Ord> From<UniqueEntityEquivalentVec<T>> for BinaryHeap<T> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        BinaryHeap::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Box<[T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Box::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Rc<[T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Rc::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>>
    for Rc<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_rc_slice_unchecked(Rc::from(value.0)) }
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for VecDeque<T> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        VecDeque::from(value.0)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>> for Box<[T; N]> {
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        Box::try_from(value.0).map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>>
    for Box<UniqueEntityEquivalentArray<T, N>>
{
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        Box::try_from(value.0)
            .map(|v|
                // SAFETY: All elements in the original Vec are unique.
                unsafe { UniqueEntityEquivalentArray::from_boxed_array_unchecked(v) })
            .map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>> for [T; N] {
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        <[T; N] as TryFrom<Vec<T>>>::try_from(value.0).map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>>
    for UniqueEntityEquivalentArray<T, N>
{
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        <[T; N] as TryFrom<Vec<T>>>::try_from(value.0)
            .map(|v|
            // SAFETY: All elements in the original Vec are unique.
            unsafe { UniqueEntityEquivalentArray::from_array_unchecked(v) })
            .map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent> From<BTreeSet<T>> for UniqueEntityEquivalentVec<T> {
    fn from(value: BTreeSet<T>) -> Self {
        Self(value.into_iter().collect::<Vec<T>>())
    }
}

impl<T: EntityEquivalent> FromIterator<T> for UniqueEntityEquivalentVec<T> {
    /// This impl only uses `Eq` to validate uniqueness, resulting in O(n^2) complexity.
    /// It can make sense for very low N, or if `T` implements neither `Ord` nor `Hash`.
    /// When possible, use `FromEntitySetIterator::from_entity_iter` instead.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        // Matches the `HashSet::from_iter` reservation logic.
        let iter = iter.into_iter();
        let unique_vec = Self::with_capacity(iter.size_hint().0);
        // Internal iteration (fold/for_each) is known to result in better code generation
        // over a for loop.
        iter.fold(unique_vec, |mut unique_vec, item| {
            if !unique_vec.0.contains(&item) {
                unique_vec.0.push(item);
            }
            unique_vec
        })
    }
}

impl<T: EntityEquivalent> FromEntitySetIterator<T> for UniqueEntityEquivalentVec<T> {
    fn from_entity_set_iter<I: EntitySet<Item = T>>(iter: I) -> Self {
        // SAFETY: `iter` is an `EntitySet`.
        unsafe { Self::from_vec_unchecked(Vec::from_iter(iter)) }
    }
}

impl<T: EntityEquivalent> Extend<T> for UniqueEntityEquivalentVec<T> {
    /// Use with caution, because this impl only uses `Eq` to validate uniqueness,
    /// resulting in O(n^2) complexity.
    /// It can make sense for very low N, or if `T` implements neither `Ord` nor `Hash`.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        // Matches the `HashSet::extend` reservation logic. Their reasoning:
        //  "Keys may be already present or show multiple times in the iterator.
        //  Reserve the entire hint lower bound if the map is empty.
        //  Otherwise reserve half the hint (rounded up), so the map
        //  will only resize twice in the worst case."
        let iter = iter.into_iter();
        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            iter.size_hint().0.div_ceil(2)
        };
        self.reserve(reserve);
        // Internal iteration (fold/for_each) is known to result in better code generation
        // over a for loop.
        iter.for_each(move |item| {
            if !self.0.contains(&item) {
                self.0.push(item);
            }
        });
    }
}

impl<'a, T: EntityEquivalent + Copy + 'a> Extend<&'a T> for UniqueEntityEquivalentVec<T> {
    /// Use with caution, because this impl only uses `Eq` to validate uniqueness,
    /// resulting in O(n^2) complexity.
    /// It can make sense for very low N, or if `T` implements neither `Ord` nor `Hash`.
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        // Matches the `HashSet::extend` reservation logic. Their reasoning:
        //  "Keys may be already present or show multiple times in the iterator.
        //  Reserve the entire hint lower bound if the map is empty.
        //  Otherwise reserve half the hint (rounded up), so the map
        //  will only resize twice in the worst case."
        let iter = iter.into_iter();
        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            iter.size_hint().0.div_ceil(2)
        };
        self.reserve(reserve);
        // Internal iteration (fold/for_each) is known to result in better code generation
        // over a for loop.
        iter.for_each(move |item| {
            if !self.0.contains(item) {
                self.0.push(*item);
            }
        });
    }
}

impl<T: EntityEquivalent> Index<(Bound<usize>, Bound<usize>)> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<Range<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeFrom<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeFull> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeTo<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeToInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<usize> for UniqueEntityEquivalentVec<T> {
    type Output = T;
    fn index(&self, key: usize) -> &T {
        self.0.index(key)
    }
}

impl<T: EntityEquivalent> IndexMut<(Bound<usize>, Bound<usize>)> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<Range<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeFrom<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeFull> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeTo<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeToInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the [`IntoIterator::into_iter`] trait
/// method on [`UniqueEntityEquivalentVec`].
pub type IntoIter<T = Entity> = UniqueEntityIter<vec::IntoIter<T>>;

impl<T: EntityEquivalent> UniqueEntityIter<vec::IntoIter<T>> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// Equivalent to [`vec::IntoIter::as_slice`].
    pub fn as_slice(&self) -> &UniqueEntityEquivalentSlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    ///
    /// Equivalent to [`vec::IntoIter::as_mut_slice`].
    pub fn as_mut_slice(&mut self) -> &mut UniqueEntityEquivalentSlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            UniqueEntityEquivalentSlice::from_slice_unchecked_mut(
                self.as_mut_inner().as_mut_slice(),
            )
        }
    }
}

/// A draining iterator for [`UniqueEntityEquivalentVec<T>`].
///
/// This struct is created by [`UniqueEntityEquivalentVec::drain`].
/// See its documentation for more.
pub type Drain<'a, T = Entity> = UniqueEntityIter<vec::Drain<'a, T>>;

impl<'a, T: EntityEquivalent> UniqueEntityIter<vec::Drain<'a, T>> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// Equivalent to [`vec::Drain::as_slice`].
    pub fn as_slice(&self) -> &UniqueEntityEquivalentSlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }
}

/// A splicing iterator for [`UniqueEntityEquivalentVec`].
///
/// This struct is created by [`UniqueEntityEquivalentVec::splice`].
/// See its documentation for more.
pub type Splice<'a, I> = UniqueEntityIter<vec::Splice<'a, I>>;
