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
    sync::Arc,
    vec::{self, Vec},
};

use super::{
    unique_slice, EntitySet, FromEntitySetIterator, TrustedEntityBorrow, UniqueEntityIter,
    UniqueEntitySlice,
};

/// A `Vec` that contains only unique entities.
///
/// "Unique" means that `x != y` holds for any 2 entities in this collection.
/// This is always true when less than 2 entities are present.
///
/// This type is best obtained by its `FromEntitySetIterator` impl, via either
/// `EntityIterator::collect_set` or `UniqueEntityVec::from_entity_iter`.
///
/// While this type can be constructed via `Iterator::collect`, doing so is inefficient,
/// and not recommended.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntityVec<T: TrustedEntityBorrow>(Vec<T>);

impl<T: TrustedEntityBorrow> UniqueEntityVec<T> {
    /// Constructs a new, empty `UniqueEntityVec<T>`.
    ///
    /// Equivalent to [`Vec::new`].
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Constructs a new, empty `UniqueEntityVec<T>` with at least the specified capacity.
    ///
    /// Equivalent to [`Vec::with_capacity`]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Creates a `UniqueEntityVec<T>` directly from a pointer, a length, and a capacity.
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

    /// Constructs a `UniqueEntityVec` from a [`Vec<T>`] unsafely.
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
    /// be inserted in the given `UniqueEntityVec<T>`.
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

    /// Converts the vector into `Box<UniqueEntitySlice<T>>`.
    pub fn into_boxed_slice(self) -> Box<UniqueEntitySlice<T>> {
        // SAFETY: UniqueEntitySlice is a transparent wrapper around [T].
        unsafe { UniqueEntitySlice::from_boxed_slice_unchecked(self.0.into_boxed_slice()) }
    }

    /// Extracts a slice containing the entire vector.
    pub fn as_slice(&self) -> &UniqueEntitySlice<T> {
        self
    }

    /// Extracts a mutable slice of the entire vector.
    pub fn as_mut_slice(&mut self) -> &mut UniqueEntitySlice<T> {
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
    pub unsafe fn append(&mut self, other: &mut UniqueEntityVec<T>) {
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

    /// Consumes and leaks the Vec, returning a mutable reference to the contents, `&'a mut UniqueEntitySlice<T>`.
    pub fn leak<'a>(self) -> &'a mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.leak()) }
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

impl<T: TrustedEntityBorrow> Default for UniqueEntityVec<T> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl<T: TrustedEntityBorrow> Deref for UniqueEntityVec<T> {
    type Target = UniqueEntitySlice<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(&self.0) }
    }
}

impl<T: TrustedEntityBorrow> DerefMut for UniqueEntityVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(&mut self.0) }
    }
}

impl<'a, T: TrustedEntityBorrow> IntoIterator for &'a UniqueEntityVec<T>
where
    &'a T: TrustedEntityBorrow,
{
    type Item = &'a T;

    type IntoIter = unique_slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: `self` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }
}

impl<T: TrustedEntityBorrow> IntoIterator for UniqueEntityVec<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: `self` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.into_iter()) }
    }
}

impl<T: TrustedEntityBorrow> AsMut<Self> for UniqueEntityVec<T> {
    fn as_mut(&mut self) -> &mut UniqueEntityVec<T> {
        self
    }
}

impl<T: TrustedEntityBorrow> AsMut<UniqueEntitySlice<T>> for UniqueEntityVec<T> {
    fn as_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow> AsRef<Self> for UniqueEntityVec<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T: TrustedEntityBorrow> AsRef<Vec<T>> for UniqueEntityVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: TrustedEntityBorrow> Borrow<Vec<T>> for UniqueEntityVec<T> {
    fn borrow(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: TrustedEntityBorrow> AsRef<[T]> for UniqueEntityVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T: TrustedEntityBorrow> AsRef<UniqueEntitySlice<T>> for UniqueEntityVec<T> {
    fn as_ref(&self) -> &UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow> Borrow<[T]> for UniqueEntityVec<T> {
    fn borrow(&self) -> &[T] {
        &self.0
    }
}

impl<T: TrustedEntityBorrow> Borrow<UniqueEntitySlice<T>> for UniqueEntityVec<T> {
    fn borrow(&self) -> &UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow> BorrowMut<UniqueEntitySlice<T>> for UniqueEntityVec<T> {
    fn borrow_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        self
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<Vec<U>> for UniqueEntityVec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<&[U]> for UniqueEntityVec<T> {
    fn eq(&self, other: &&[U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<&UniqueEntitySlice<U>>
    for UniqueEntityVec<T>
{
    fn eq(&self, other: &&UniqueEntitySlice<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<&mut [U]> for UniqueEntityVec<T> {
    fn eq(&self, other: &&mut [U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow>
    PartialEq<&mut UniqueEntitySlice<U>> for UniqueEntityVec<T>
{
    fn eq(&self, other: &&mut UniqueEntitySlice<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U, const N: usize> PartialEq<&[U; N]>
    for UniqueEntityVec<T>
{
    fn eq(&self, other: &&[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U, const N: usize> PartialEq<&mut [U; N]>
    for UniqueEntityVec<T>
{
    fn eq(&self, other: &&mut [U; N]) -> bool {
        self.0.eq(&**other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U> PartialEq<[U]> for UniqueEntityVec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntitySlice<U>>
    for UniqueEntityVec<T>
{
    fn eq(&self, other: &UniqueEntitySlice<U>) -> bool {
        self.0.eq(&**other)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for UniqueEntityVec<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>> for Vec<T> {
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>> for &[T] {
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>> for &mut [T] {
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>>
    for [T]
{
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: TrustedEntityBorrow> PartialEq<UniqueEntityVec<U>> for VecDeque<T> {
    fn eq(&self, other: &UniqueEntityVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&UniqueEntitySlice<T>> for UniqueEntityVec<T> {
    fn from(value: &UniqueEntitySlice<T>) -> Self {
        value.to_vec()
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&mut UniqueEntitySlice<T>> for UniqueEntityVec<T> {
    fn from(value: &mut UniqueEntitySlice<T>) -> Self {
        value.to_vec()
    }
}

impl<T: TrustedEntityBorrow> From<Box<UniqueEntitySlice<T>>> for UniqueEntityVec<T> {
    fn from(value: Box<UniqueEntitySlice<T>>) -> Self {
        value.into_vec()
    }
}

impl<T: TrustedEntityBorrow> From<Cow<'_, UniqueEntitySlice<T>>> for UniqueEntityVec<T>
where
    UniqueEntitySlice<T>: ToOwned<Owned = UniqueEntityVec<T>>,
{
    fn from(value: Cow<UniqueEntitySlice<T>>) -> Self {
        value.into_owned()
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&[T; 1]> for UniqueEntityVec<T> {
    fn from(value: &[T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&[T; 0]> for UniqueEntityVec<T> {
    fn from(value: &[T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&mut [T; 1]> for UniqueEntityVec<T> {
    fn from(value: &mut [T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: TrustedEntityBorrow + Clone> From<&mut [T; 0]> for UniqueEntityVec<T> {
    fn from(value: &mut [T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: TrustedEntityBorrow> From<[T; 1]> for UniqueEntityVec<T> {
    fn from(value: [T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: TrustedEntityBorrow> From<[T; 0]> for UniqueEntityVec<T> {
    fn from(value: [T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Vec<T> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        value.0
    }
}

impl<'a, T: TrustedEntityBorrow + Clone> From<UniqueEntityVec<T>> for Cow<'a, [T]> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        Cow::from(value.0)
    }
}

impl<'a, T: TrustedEntityBorrow + Clone> From<UniqueEntityVec<T>>
    for Cow<'a, UniqueEntitySlice<T>>
{
    fn from(value: UniqueEntityVec<T>) -> Self {
        Cow::Owned(value)
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Arc<[T]> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        Arc::from(value.0)
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Arc<UniqueEntitySlice<T>> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_arc_slice_unchecked(Arc::from(value.0)) }
    }
}

impl<T: TrustedEntityBorrow + Ord> From<UniqueEntityVec<T>> for BinaryHeap<T> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        BinaryHeap::from(value.0)
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Box<[T]> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        Box::from(value.0)
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Rc<[T]> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        Rc::from(value.0)
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for Rc<UniqueEntitySlice<T>> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_rc_slice_unchecked(Rc::from(value.0)) }
    }
}

impl<T: TrustedEntityBorrow> From<UniqueEntityVec<T>> for VecDeque<T> {
    fn from(value: UniqueEntityVec<T>) -> Self {
        VecDeque::from(value.0)
    }
}

impl<T: TrustedEntityBorrow, const N: usize> TryFrom<UniqueEntityVec<T>> for Box<[T; N]> {
    type Error = UniqueEntityVec<T>;

    fn try_from(value: UniqueEntityVec<T>) -> Result<Self, Self::Error> {
        Box::try_from(value.0).map_err(UniqueEntityVec)
    }
}

impl<T: TrustedEntityBorrow, const N: usize> TryFrom<UniqueEntityVec<T>> for [T; N] {
    type Error = UniqueEntityVec<T>;

    fn try_from(value: UniqueEntityVec<T>) -> Result<Self, Self::Error> {
        <[T; N] as TryFrom<Vec<T>>>::try_from(value.0).map_err(UniqueEntityVec)
    }
}

impl<T: TrustedEntityBorrow> From<BTreeSet<T>> for UniqueEntityVec<T> {
    fn from(value: BTreeSet<T>) -> Self {
        Self(value.into_iter().collect::<Vec<T>>())
    }
}

impl<T: TrustedEntityBorrow> FromIterator<T> for UniqueEntityVec<T> {
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

impl<T: TrustedEntityBorrow> FromEntitySetIterator<T> for UniqueEntityVec<T> {
    fn from_entity_set_iter<I: EntitySet<Item = T>>(iter: I) -> Self {
        // SAFETY: `iter` is an `EntitySet`.
        unsafe { Self::from_vec_unchecked(Vec::from_iter(iter)) }
    }
}

impl<T: TrustedEntityBorrow> Extend<T> for UniqueEntityVec<T> {
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
            (iter.size_hint().0 + 1) / 2
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

impl<'a, T: TrustedEntityBorrow + Copy + 'a> Extend<&'a T> for UniqueEntityVec<T> {
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
            (iter.size_hint().0 + 1) / 2
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

impl<T: TrustedEntityBorrow> Index<(Bound<usize>, Bound<usize>)> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<Range<usize>> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeFrom<usize>> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeFull> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeInclusive<usize>> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeTo<usize>> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<RangeToInclusive<usize>> for UniqueEntityVec<T> {
    type Output = UniqueEntitySlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: TrustedEntityBorrow> Index<usize> for UniqueEntityVec<T> {
    type Output = T;
    fn index(&self, key: usize) -> &T {
        self.0.index(key)
    }
}

impl<T: TrustedEntityBorrow> IndexMut<(Bound<usize>, Bound<usize>)> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<Range<usize>> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeFrom<usize>> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeFull> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeInclusive<usize>> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeTo<usize>> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: TrustedEntityBorrow> IndexMut<RangeToInclusive<usize>> for UniqueEntityVec<T> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the [`IntoIterator::into_iter`] trait
/// method on [`UniqueEntityVec`].
pub type IntoIter<T> = UniqueEntityIter<vec::IntoIter<T>>;

impl<T: TrustedEntityBorrow> UniqueEntityIter<vec::IntoIter<T>> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// Equivalent to [`vec::IntoIter::as_slice`].
    pub fn as_slice(&self) -> &UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    ///
    /// Equivalent to [`vec::IntoIter::as_mut_slice`].
    pub fn as_mut_slice(&mut self) -> &mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.as_mut_inner().as_mut_slice()) }
    }
}

/// A draining iterator for [`UniqueEntityVec<T>`].
///
/// This struct is created by [`UniqueEntityVec::drain`].
/// See its documentation for more.
pub type Drain<'a, T> = UniqueEntityIter<vec::Drain<'a, T>>;

impl<'a, T: TrustedEntityBorrow> UniqueEntityIter<vec::Drain<'a, T>> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// Equivalent to [`vec::Drain::as_slice`].
    pub fn as_slice(&self) -> &UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.as_inner().as_slice()) }
    }
}

/// A splicing iterator for [`UniqueEntityVec`].
///
/// This struct is created by [`UniqueEntityVec::splice`].
/// See its documentation for more.
pub type Splice<'a, I> = UniqueEntityIter<vec::Splice<'a, I>>;
