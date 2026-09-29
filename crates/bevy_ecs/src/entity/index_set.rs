//! Contains the [`EntityEquivalentIndexSet`] type, a [`IndexSet`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around `indexmap`'ss [`IndexSet`] that is more performant for [`Entity`] keys.

use core::{
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::BuildHasher,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::{
        BitAnd, BitOr, BitXor, Bound, Deref, Index, Range, RangeBounds, RangeFrom, RangeFull,
        RangeInclusive, RangeTo, RangeToInclusive, Sub,
    },
    ptr,
};

use indexmap::{self, set, IndexSet};

use super::{Entity, EntityEquivalent, EntityHash, EntitySetIterator};

use bevy_platform::prelude::Box;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// An [`IndexSet`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct EntityEquivalentIndexSet<K: EntityEquivalent + Hash>(IndexSet<K, EntityHash>);

/// An [`IndexSet`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityIndexSet = EntityEquivalentIndexSet<Entity>;

impl<K: EntityEquivalent + Hash> EntityEquivalentIndexSet<K> {
    /// Creates an empty `EntityEquivalentIndexSet`.
    ///
    /// Equivalent to [`IndexSet::with_hasher(EntityHash)`].
    ///
    /// [`IndexSet::with_hasher(EntityHash)`]: IndexSet::with_hasher
    pub const fn new() -> Self {
        Self(IndexSet::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityEquivalentIndexSet` with the specified capacity.
    ///
    /// Equivalent to [`IndexSet::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`IndexSet::with_capacity_and_hasher(n, EntityHash)`]: IndexSet::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(IndexSet::with_capacity_and_hasher(n, EntityHash))
    }

    /// Constructs an `EntityIndexSet` from an [`IndexSet`].
    ///
    /// # Safety
    ///
    /// The given set cannot contain duplicates, for example by using
    /// [`MutableValues`](indexmap::set::MutableValues).
    pub const unsafe fn from_index_set_unchecked(set: IndexSet<K, EntityHash>) -> Self {
        Self(set)
    }

    /// Returns a mutable reference to the inner [`IndexSet`].
    ///
    /// # Safety
    ///
    /// The returned reference cannot be used to introduce duplicates in the set, for example
    /// by using [`MutableValues`](indexmap::set::MutableValues).
    pub const unsafe fn as_index_set_unchecked(&mut self) -> &mut IndexSet<K, EntityHash> {
        &mut self.0
    }

    /// Returns the inner [`IndexSet`].
    pub fn into_inner(self) -> IndexSet<K, EntityHash> {
        self.0
    }

    /// Returns a slice of all the values in the set.
    ///
    /// Equivalent to [`IndexSet::as_slice`].
    pub fn as_slice(&self) -> &Slice<K> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Clears the `IndexSet` in the given index range, returning those values
    /// as a drain iterator.
    ///
    /// Equivalent to [`IndexSet::drain`].
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, K> {
        Drain(self.0.drain(range), PhantomData)
    }

    /// Returns a slice of values in the given range of indices.
    ///
    /// Equivalent to [`IndexSet::get_range`].
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Slice<K>> {
        self.0.get_range(range).map(|slice|
            // SAFETY: The source IndexSet uses EntityHash.
            unsafe { Slice::from_slice_unchecked(slice) })
    }

    /// Return an iterator over the values of the set, in their order.
    ///
    /// Equivalent to [`IndexSet::iter`].
    pub fn iter(&self) -> Iter<'_, K> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Converts into a boxed slice of all the values in the set.
    ///
    /// Equivalent to [`IndexSet::into_boxed_slice`].
    pub fn into_boxed_slice(self) -> Box<Slice<K>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.0.into_boxed_slice()) }
    }

    /// Moves all values from `other` into `self`, leaving `other` empty.
    ///
    /// Equivalent to [`IndexSet::append`].
    pub fn append(&mut self, other: &mut EntityEquivalentIndexSet<K>) {
        self.0.append(&mut other.0);
    }

    /// Remove all elements in the set, while preserving its capacity.
    ///
    /// Equivalent to [`IndexSet::clear`].
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Creates an iterator which uses a closure to determine if a value should be removed,
    /// for all values in the given range.
    ///
    /// Equivalent to [`IndexSet::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> set::ExtractIf<'_, K, F>
    where
        F: FnMut(&K) -> bool,
        R: RangeBounds<usize>,
    {
        self.0.extract_if(range, pred)
    }

    /// Insert the value into the set.
    ///
    /// Equivalent to [`IndexSet::insert`].
    pub fn insert(&mut self, value: K) -> bool {
        self.0.insert(value)
    }

    /// Insert the value into the set before the value at the given index, or at the end.
    ///
    /// Equivalent to [`IndexSet::insert_before`].
    pub fn insert_before(&mut self, index: usize, value: K) -> (usize, bool) {
        self.0.insert_before(index, value)
    }

    /// Insert the value into the set, and get its index.
    ///
    /// Equivalent to [`IndexSet::insert_full`].
    pub fn insert_full(&mut self, value: K) -> (usize, bool) {
        self.0.insert_full(value)
    }

    /// Insert the value into the set at its ordered position among sorted values.
    ///
    /// Equivalent to [`IndexSet::insert_sorted`].
    pub fn insert_sorted(&mut self, value: K) -> (usize, bool)
    where
        K: Ord,
    {
        self.0.insert_sorted(value)
    }

    /// Insert the value into the set at its ordered position among values sorted by `cmp`.
    ///
    /// Equivalent to [`IndexSet::insert_sorted_by`].
    pub fn insert_sorted_by<F>(&mut self, value: K, cmp: F) -> (usize, bool)
    where
        F: FnMut(&K, &K) -> Ordering,
    {
        self.0.insert_sorted_by(value, cmp)
    }

    /// Insert the value into the set at its ordered position among values using a sort-key extraction function.
    ///
    /// Equivalent to [`IndexSet::insert_sorted_by_key`].
    pub fn insert_sorted_by_key<B, F>(&mut self, value: K, sort_key: F) -> (usize, bool)
    where
        B: Ord,
        F: FnMut(&K) -> B,
    {
        self.0.insert_sorted_by_key(value, sort_key)
    }

    /// Moves the position of a value from one index to another by shifting all other values in-between.
    ///
    /// Equivalent to [`IndexSet::move_index`].
    pub fn move_index(&mut self, from: usize, to: usize) {
        self.0.move_index(from, to);
    }

    /// Remove the last value
    ///
    /// Equivalent to [`IndexSet::pop`].
    pub fn pop(&mut self) -> Option<K> {
        self.0.pop()
    }

    /// Removes and returns the last value from a set if the predicate returns `true`,
    /// or [`None`] if the predicate returns `false` or the set is empty
    /// (the predicate will not be called in that case).
    ///
    /// Equivalent to [`IndexSet::pop_if`].
    pub fn pop_if(&mut self, predicate: impl FnOnce(&K) -> bool) -> Option<K> {
        self.0.pop_if(predicate)
    }

    /// Adds a value to the set, replacing the existing value, if any, that is equal to
    /// the given one, without altering its insertion order. Returns the replaced value.
    ///
    /// Equivalent to [`IndexSet::replace`].
    pub fn replace(&mut self, value: K) -> Option<K> {
        self.0.replace(value)
    }

    /// Adds a value to the set, replacing the existing value, if any, that is equal to
    /// the given one, without altering its insertion order. Returns the index of the item
    /// and its replaced value.
    ///
    /// Equivalent to [`IndexSet::replace_full`].
    pub fn replace_full(&mut self, value: K) -> (usize, Option<K>) {
        self.0.replace_full(value)
    }

    /// Replaces the value at the given index. The new value does not need to be equivalent
    /// to the one it is replacing, but it must be unique to the rest of the set.
    ///
    /// Equivalent to [`IndexSet::replace_index`].
    pub fn replace_index(&mut self, index: usize, value: K) -> Result<K, (usize, K)> {
        self.0.replace_index(index, value)
    }

    /// Reserve capacity for `additional` more values.
    ///
    /// Equivalent to [`IndexSet::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Reserve capacity for `additional` more values, without over-allocating.
    ///
    /// Equivalent to [`IndexSet::reserve_exact`].
    pub fn reserve_exact(&mut self, additional: usize) {
        self.0.reserve_exact(additional);
    }

    /// Scan through each value in the set and keep those where the closure `keep` returns `true`.
    ///
    /// Equivalent to [`IndexSet::retain`].
    pub fn retain<F>(&mut self, keep: F)
    where
        F: FnMut(&K) -> bool,
    {
        self.0.retain(keep);
    }

    /// Reverses the order of the set’s values in place.
    ///
    /// Equivalent to [`IndexSet::reverse`].
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Insert the value into the set at the given index.
    ///
    /// Equivalent to [`IndexSet::shift_insert`].
    pub fn shift_insert(&mut self, index: usize, value: K) -> bool {
        self.0.shift_insert(index, value)
    }

    /// Remove the value from the set, and return `true` if it was present.
    ///
    /// Equivalent to [`IndexSet::shift_remove`].
    pub fn shift_remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: ?Sized + Hash + indexmap::Equivalent<K>,
    {
        self.0.shift_remove(value)
    }

    /// Remove the value from the set return it and the index it had.
    ///
    /// Equivalent to [`IndexSet::shift_remove_full`].
    pub fn shift_remove_full<Q>(&mut self, value: &Q) -> Option<(usize, K)>
    where
        Q: ?Sized + Hash + indexmap::Equivalent<K>,
    {
        self.0.shift_remove_full(value)
    }

    /// Remove the value by index
    ///
    /// Equivalent to [`IndexSet::shift_remove_index`].
    pub fn shift_remove_index(&mut self, index: usize) -> Option<K> {
        self.0.shift_remove_index(index)
    }

    /// Removes and returns the value in the set, if any, that is equal to the given one.
    ///
    /// Equivalent to [`IndexSet::shift_take`].
    pub fn shift_take<Q>(&mut self, value: &Q) -> Option<K>
    where
        Q: ?Sized + Hash + indexmap::Equivalent<K>,
    {
        self.0.shift_take(value)
    }

    /// Shrink the capacity of the set with a lower limit.
    ///
    /// Equivalent to [`IndexSet::shrink_to`].
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.0.shrink_to(min_capacity);
    }

    /// Shrink the capacity of the set as much as possible.
    ///
    /// Equivalent to [`IndexSet::shrink_to_fit`].
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Sort the set’s values by their default ordering.
    ///
    /// Equivalent to [`IndexSet::sort`].
    pub fn sort(&mut self)
    where
        K: Ord,
    {
        self.0.sort();
    }

    /// Sort the set’s values in place using the comparison function `cmp`.
    ///
    /// Equivalent to [`IndexSet::sort_by`].
    pub fn sort_by<F>(&mut self, cmp: F)
    where
        F: FnMut(&K, &K) -> Ordering,
    {
        self.0.sort_by(cmp);
    }

    /// Sort the set’s values in place using a key extraction function.
    ///
    /// Equivalent to [`IndexSet::sort_by_cached_key`].
    pub fn sort_by_cached_key<Q, F>(&mut self, sort_key: F)
    where
        Q: Ord,
        F: FnMut(&K) -> Q,
    {
        self.0.sort_by_cached_key(sort_key);
    }

    /// Sort the set’s values in place using a key extraction function.
    ///
    /// Equivalent to [`IndexSet::sort_by_key`].
    pub fn sort_by_key<Q, F>(&mut self, sort_key: F)
    where
        Q: Ord,
        F: FnMut(&K) -> Q,
    {
        self.0.sort_by_key(sort_key);
    }

    /// Sort the set’s values by their default ordering.
    ///
    /// Equivalent to [`IndexSet::sort_unstable`].
    pub fn sort_unstable(&mut self)
    where
        K: Ord,
    {
        self.0.sort_unstable();
    }

    /// Sort the set’s values in place using the comparison function `cmp`.
    ///
    /// Equivalent to [`IndexSet::sort_unstable_by`].
    pub fn sort_unstable_by<F>(&mut self, cmp: F)
    where
        F: FnMut(&K, &K) -> Ordering,
    {
        self.0.sort_unstable_by(cmp);
    }

    /// Sort the set’s values in place using a key extraction function.
    ///
    /// Equivalent to [`IndexSet::sort_unstable_by_key`].
    pub fn sort_unstable_by_key<Q, F>(&mut self, sort_key: F)
    where
        Q: Ord,
        F: FnMut(&K) -> Q,
    {
        self.0.sort_unstable_by_key(sort_key);
    }

    /// Creates a splicing iterator that replaces the specified range in the set with the given
    /// `replace_with` iterator and yields the removed items. `replace_with` does not need to be
    /// the same length as `range`.
    ///
    /// Equivalent to [`IndexSet::splice`].
    pub fn splice<R, I>(
        &mut self,
        range: R,
        replace_with: I,
    ) -> set::Splice<'_, I::IntoIter, K, EntityHash>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = K>,
    {
        self.0.splice(range, replace_with)
    }

    /// Splits the collection into two at the given index.
    ///
    /// Equivalent to [`IndexSet::split_off`].
    pub fn split_off(&mut self, at: usize) -> Self {
        let splitted_off = self.0.split_off(at);
        // SAFETY: `self` didn't contain duplicates, so the splitted off part also doesn't contain duplicated.
        unsafe { Self::from_index_set_unchecked(splitted_off) }
    }

    /// Swaps the position of two values in the set.
    ///
    /// Equivalent to [`IndexSet::swap_indices`].
    pub fn swap_indices(&mut self, a: usize, b: usize) {
        self.0.swap_indices(a, b);
    }

    /// Remove the value from the set, and return `true` if it was present.
    ///
    /// Equivalent to [`IndexSet::swap_remove`].
    pub fn swap_remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: ?Sized + Hash + indexmap::Equivalent<K>,
    {
        self.0.swap_remove(value)
    }

    /// Remove the value from the set return it and the index it had.
    ///
    /// Equivalent to [`IndexSet::swap_remove_full`].
    pub fn swap_remove_full<Q>(&mut self, value: &Q) -> Option<(usize, K)>
    where
        Q: ?Sized + Hash + indexmap::Equivalent<K>,
    {
        self.0.swap_remove_full(value)
    }

    /// Remove the value by index
    ///
    /// Equivalent to [`IndexSet::swap_remove_index`].
    pub fn swap_remove_index(&mut self, index: usize) -> Option<K> {
        self.0.swap_remove_index(index)
    }

    /// Removes and returns the value in the set, if any, that is equal to the given one.
    ///
    /// Equivalent to [`IndexSet::swap_take`].
    pub fn swap_take<Q>(&mut self, value: &Q) -> Option<K>
    where
        Q: ?Sized + Hash + indexmap::Equivalent<K>,
    {
        self.0.swap_take(value)
    }

    /// Shortens the set, keeping the first `len` elements and dropping the rest.
    ///
    /// Equivalent to [`IndexSet::truncate`].
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    /// Try to reserve capacity for `additional` more values.
    ///
    /// Equivalent to [`IndexSet::try_reserve`].
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), indexmap::TryReserveError> {
        self.0.try_reserve(additional)
    }

    /// Try to reserve capacity for `additional` more values, without over-allocating.
    ///
    /// Equivalent to [`IndexSet::try_reserve`].
    pub fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), indexmap::TryReserveError> {
        self.0.try_reserve_exact(additional)
    }
}

impl<K: EntityEquivalent + Hash> Default for EntityEquivalentIndexSet<K> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: EntityEquivalent + Hash> Deref for EntityEquivalentIndexSet<K> {
    type Target = IndexSet<K, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: EntityEquivalent + Hash> IntoIterator for &'a EntityEquivalentIndexSet<K> {
    type Item = &'a K;

    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        Iter((&self.0).into_iter(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash> IntoIterator for EntityEquivalentIndexSet<K> {
    type Item = K;

    type IntoIter = IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitAnd for &EntityEquivalentIndexSet<K> {
    type Output = EntityEquivalentIndexSet<K>;

    fn bitand(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.bitand(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitOr for &EntityEquivalentIndexSet<K> {
    type Output = EntityEquivalentIndexSet<K>;

    fn bitor(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.bitor(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitXor for &EntityEquivalentIndexSet<K> {
    type Output = EntityEquivalentIndexSet<K>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.bitxor(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> Sub for &EntityEquivalentIndexSet<K> {
    type Output = EntityEquivalentIndexSet<K>;

    fn sub(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.sub(&rhs.0))
    }
}

impl<'a, K: EntityEquivalent + Hash + Copy> Extend<&'a K> for EntityEquivalentIndexSet<K> {
    fn extend<I: IntoIterator<Item = &'a K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: EntityEquivalent + Hash> Extend<K> for EntityEquivalentIndexSet<K> {
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: EntityEquivalent + Hash, const N: usize> From<[K; N]> for EntityEquivalentIndexSet<K> {
    fn from(value: [K; N]) -> Self {
        Self(IndexSet::from_iter(value))
    }
}

impl<K: EntityEquivalent + Hash> FromIterator<K> for EntityEquivalentIndexSet<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iterable: I) -> Self {
        Self(IndexSet::from_iter(iterable))
    }
}

impl<K: EntityEquivalent + Hash, S2> PartialEq<IndexSet<K, S2>> for EntityEquivalentIndexSet<K>
where
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexSet<K, S2>) -> bool {
        self.0.eq(other)
    }
}

impl<K: EntityEquivalent + Hash> PartialEq for EntityEquivalentIndexSet<K> {
    fn eq(&self, other: &EntityEquivalentIndexSet<K>) -> bool {
        self.0.eq(other)
    }
}

impl<K: EntityEquivalent + Hash> Eq for EntityEquivalentIndexSet<K> {}

impl<K: EntityEquivalent + Hash> Index<(Bound<usize>, Bound<usize>)>
    for EntityEquivalentIndexSet<K>
{
    type Output = Slice<K>;

    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<Range<usize>> for EntityEquivalentIndexSet<K> {
    type Output = Slice<K>;

    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeFrom<usize>> for EntityEquivalentIndexSet<K> {
    type Output = Slice<K>;

    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeFull> for EntityEquivalentIndexSet<K> {
    type Output = Slice<K>;

    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeInclusive<usize>> for EntityEquivalentIndexSet<K> {
    type Output = Slice<K>;

    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeTo<usize>> for EntityEquivalentIndexSet<K> {
    type Output = Slice<K>;

    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeToInclusive<usize>> for EntityEquivalentIndexSet<K> {
    type Output = Slice<K>;

    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<usize> for EntityEquivalentIndexSet<K> {
    type Output = K;

    fn index(&self, key: usize) -> &K {
        self.0.index(key)
    }
}

/// A dynamically-sized slice of values in an [`EntityEquivalentIndexSet`].
///
/// Equivalent to an [`indexmap::set::Slice<V>`] whose source [`IndexSet`]
/// uses [`EntityHash`].
#[repr(transparent)]
pub struct Slice<K: EntityEquivalent + Hash, S = EntityHash>(PhantomData<S>, set::Slice<K>);

impl<K: EntityEquivalent + Hash> Slice<K> {
    /// Returns an empty slice.
    ///
    /// Equivalent to [`set::Slice::new`].
    pub const fn new<'a>() -> &'a Self {
        // SAFETY: The source slice is empty.
        unsafe { Self::from_slice_unchecked(set::Slice::<K>::new()) }
    }

    /// Constructs a [`entity::index_set::Slice`] from a [`indexmap::set::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexSet`] using [`EntityHash`].
    ///
    /// [`entity::index_set::Slice`]: `crate::entity::index_set::Slice`
    pub const unsafe fn from_slice_unchecked(slice: &set::Slice<K>) -> &Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { &*(ptr::from_ref(slice) as *const Self) }
    }

    /// Constructs a [`entity::index_set::Slice`] from a [`indexmap::set::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexSet`] using [`EntityHash`].
    ///
    /// [`entity::index_set::Slice`]: `crate::entity::index_set::Slice`
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut set::Slice<K>) -> &mut Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    pub const fn as_inner(&self) -> &set::Slice<K> {
        &self.1
    }

    /// Constructs a boxed [`entity::index_set::Slice`] from a boxed [`indexmap::set::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexSet`] using [`EntityHash`].
    ///
    /// [`entity::index_set::Slice`]: `crate::entity::index_set::Slice`
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<set::Slice<K>>) -> Box<Self> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Casts a reference to `self` to the inner slice.
    #[expect(
        clippy::borrowed_box,
        reason = "We wish to access the Box API of the inner type, without consuming it."
    )]
    pub const fn as_boxed_inner(self: &Box<Self>) -> &Box<set::Slice<K>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { &*(ptr::from_ref(self).cast::<Box<set::Slice<K>>>()) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_boxed_inner(self: Box<Self>) -> Box<set::Slice<K>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Box::from_raw(Box::into_raw(self) as *mut set::Slice<K>) }
    }

    /// Returns a slice of values in the given range of indices.
    ///
    /// Equivalent to [`set::Slice::get_range`].
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Self> {
        self.1.get_range(range).map(|slice|
            // SAFETY: This a subslice of a valid slice.
            unsafe { Self::from_slice_unchecked(slice) })
    }

    /// Divides one slice into two at an index.
    ///
    /// Equivalent to [`set::Slice::split_at`].
    pub fn split_at(&self, index: usize) -> (&Self, &Self) {
        let (slice_1, slice_2) = self.1.split_at(index);
        // SAFETY: These are subslices of a valid slice.
        unsafe {
            (
                Self::from_slice_unchecked(slice_1),
                Self::from_slice_unchecked(slice_2),
            )
        }
    }

    /// Returns the first value and the rest of the slice,
    /// or `None` if it is empty.
    ///
    /// Equivalent to [`set::Slice::split_first`].
    pub fn split_first(&self) -> Option<(&K, &Self)> {
        self.1.split_first().map(|(first, rest)| {
            (
                first,
                // SAFETY: This a subslice of a valid slice.
                unsafe { Self::from_slice_unchecked(rest) },
            )
        })
    }

    /// Returns the last value and the rest of the slice,
    /// or `None` if it is empty.
    ///
    /// Equivalent to [`set::Slice::split_last`].
    pub fn split_last(&self) -> Option<(&K, &Self)> {
        self.1.split_last().map(|(last, rest)| {
            (
                last,
                // SAFETY: This a subslice of a valid slice.
                unsafe { Self::from_slice_unchecked(rest) },
            )
        })
    }

    /// Return an iterator over the values of the set slice.
    ///
    /// Equivalent to [`set::Slice::iter`].
    pub fn iter(&self) -> Iter<'_, K> {
        Iter(self.1.iter(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash> Deref for Slice<K> {
    type Target = set::Slice<K>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a, K: EntityEquivalent + Hash> IntoIterator for &'a Slice<K> {
    type IntoIter = Iter<'a, K>;
    type Item = &'a K;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: EntityEquivalent + Hash> IntoIterator for Box<Slice<K>> {
    type IntoIter = IntoIter<K>;
    type Item = K;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.into_boxed_inner().into_iter(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash + Clone> Clone for Box<Slice<K>> {
    fn clone(&self) -> Self {
        // SAFETY: This is a clone of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.as_boxed_inner().clone()) }
    }
}

impl<K: EntityEquivalent + Hash> Default for &Slice<K> {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked(<&set::Slice<K>>::default()) }
    }
}

impl<K: EntityEquivalent + Hash> Default for Box<Slice<K>> {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_boxed_slice_unchecked(<Box<set::Slice<K>>>::default()) }
    }
}

impl<K: EntityEquivalent + Hash + Debug> Debug for Slice<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Slice")
            .field(&self.0)
            .field(&&self.1)
            .finish()
    }
}

impl<K: EntityEquivalent + Hash + Copy> From<&Slice<K>> for Box<Slice<K>> {
    fn from(value: &Slice<K>) -> Self {
        // SAFETY: This slice is a copy of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(value.1.into()) }
    }
}

impl<K: EntityEquivalent + Hash> Hash for Slice<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl<K: EntityEquivalent + Hash + PartialOrd> PartialOrd for Slice<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(other)
    }
}

impl<K: EntityEquivalent + Hash + Ord> Ord for Slice<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(other)
    }
}

impl<K: EntityEquivalent + Hash> PartialEq for Slice<K> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<K: EntityEquivalent + Hash> Eq for Slice<K> {}

impl<K: EntityEquivalent + Hash> Index<(Bound<usize>, Bound<usize>)> for Slice<K> {
    type Output = Self;

    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<Range<usize>> for Slice<K> {
    type Output = Self;

    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeFrom<usize>> for Slice<K> {
    type Output = Slice<K>;

    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeFull> for Slice<K> {
    type Output = Self;

    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeInclusive<usize>> for Slice<K> {
    type Output = Self;

    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeTo<usize>> for Slice<K> {
    type Output = Self;

    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<RangeToInclusive<usize>> for Slice<K> {
    type Output = Self;

    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: EntityEquivalent + Hash> Index<usize> for Slice<K> {
    type Output = K;

    fn index(&self, key: usize) -> &K {
        self.1.index(key)
    }
}

/// An iterator over the items of an [`EntityEquivalentIndexSet`].
///
/// This struct is created by the [`iter`] method on [`EntityEquivalentIndexSet`]. See its documentation for more.
///
/// [`iter`]: EntityEquivalentIndexSet::iter
pub struct Iter<'a, K: EntityEquivalent + Hash, S = EntityHash>(set::Iter<'a, K>, PhantomData<S>);

impl<'a, K: EntityEquivalent + Hash> Iter<'a, K> {
    /// Constructs a [`Iter<'a, S>`] from a [`set::Iter<'a>`] unsafely.
    ///
    /// # Safety
    ///
    /// `iter` must either be empty, or have been obtained from a
    /// [`IndexSet`] using the `S` hasher.
    pub const unsafe fn from_iter_unchecked<S>(iter: set::Iter<'a, K>) -> Iter<'a, K, S> {
        Iter(iter, PhantomData)
    }

    /// Returns the inner [`Iter`](set::Iter).
    pub const fn into_inner(self) -> set::Iter<'a, K> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`set::Iter::as_slice`].
    pub fn as_slice(&self) -> &Slice<K> {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a, K: EntityEquivalent + Hash> Deref for Iter<'a, K> {
    type Target = set::Iter<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: EntityEquivalent + Hash> Iterator for Iter<'a, K> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    fn last(self) -> Option<Self::Item> {
        self.0.last()
    }

    fn collect<C>(self) -> C
    where
        C: FromIterator<Self::Item>,
    {
        self.0.collect()
    }
}

impl<K: EntityEquivalent + Hash> DoubleEndedIterator for Iter<'_, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }
}

impl<K: EntityEquivalent + Hash> ExactSizeIterator for Iter<'_, K> {}

impl<K: EntityEquivalent + Hash> FusedIterator for Iter<'_, K> {}

impl<K: EntityEquivalent + Hash> Clone for Iter<'_, K> {
    fn clone(&self) -> Self {
        // SAFETY: We are cloning an already valid `Iter`.
        unsafe { Self::from_iter_unchecked(self.0.clone()) }
    }
}

impl<K: EntityEquivalent + Hash + Debug> Debug for Iter<'_, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<K: EntityEquivalent + Hash> Default for Iter<'_, K> {
    fn default() -> Self {
        // SAFETY: `Iter` is empty.
        unsafe { Self::from_iter_unchecked(Default::default()) }
    }
}

// SAFETY: Iter stems from a correctly behaving `IndexSet<K, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for Iter<'_, K> {}

/// Owning iterator over the items of an [`EntityEquivalentIndexSet`].
///
/// This struct is created by the [`into_iter`] method on [`EntityEquivalentIndexSet`] (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: EntityEquivalentIndexSet::into_iter
pub struct IntoIter<K: EntityEquivalent + Hash, S = EntityHash>(set::IntoIter<K>, PhantomData<S>);

impl<K: EntityEquivalent + Hash> IntoIter<K> {
    /// Constructs a [`IntoIter<S>`] from a [`set::IntoIter`] unsafely.
    ///
    /// # Safety
    ///
    /// `into_iter` must either be empty, or have been obtained from a
    /// [`IndexSet`] using the `S` hasher.
    pub const unsafe fn from_into_iter_unchecked<S>(into_iter: set::IntoIter<K>) -> IntoIter<K, S> {
        IntoIter(into_iter, PhantomData)
    }

    /// Returns the inner [`IntoIter`](set::IntoIter).
    pub fn into_inner(self) -> set::IntoIter<K> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`set::IntoIter::as_slice`].
    pub fn as_slice(&self) -> &Slice<K> {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<K: EntityEquivalent + Hash> Deref for IntoIter<K> {
    type Target = set::IntoIter<K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: EntityEquivalent + Hash> Iterator for IntoIter<K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    fn last(self) -> Option<Self::Item> {
        self.0.last()
    }

    fn collect<C>(self) -> C
    where
        C: FromIterator<Self::Item>,
    {
        self.0.collect()
    }
}

impl<K: EntityEquivalent + Hash> DoubleEndedIterator for IntoIter<K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }
}

impl<K: EntityEquivalent + Hash> ExactSizeIterator for IntoIter<K> {}

impl<K: EntityEquivalent + Hash> FusedIterator for IntoIter<K> {}

impl<K: EntityEquivalent + Hash + Clone> Clone for IntoIter<K> {
    fn clone(&self) -> Self {
        // SAFETY: We are cloning an already valid `IntoIter`.
        unsafe { Self::from_into_iter_unchecked(self.0.clone()) }
    }
}

impl<K: EntityEquivalent + Hash + Debug> Debug for IntoIter<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<K: EntityEquivalent + Hash> Default for IntoIter<K> {
    fn default() -> Self {
        // SAFETY: `IntoIter` is empty.
        unsafe { Self::from_into_iter_unchecked(Default::default()) }
    }
}

// SAFETY: IntoIter stems from a correctly behaving `IndexSet<K, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for IntoIter<K> {}

/// A draining iterator over the items of an [`EntityEquivalentIndexSet`].
///
/// This struct is created by the [`drain`] method on [`EntityEquivalentIndexSet`]. See its documentation for more.
///
/// [`drain`]: EntityEquivalentIndexSet::drain
pub struct Drain<'a, K: EntityEquivalent + Hash, S = EntityHash>(set::Drain<'a, K>, PhantomData<S>);

impl<'a, K: EntityEquivalent + Hash> Drain<'a, K> {
    /// Constructs a [`Drain<'a, S>`] from a [`set::Drain<'a>`] unsafely.
    ///
    /// # Safety
    ///
    /// `drain` must either be empty, or have been obtained from a
    /// [`IndexSet`] using the `S` hasher.
    pub const unsafe fn from_drain_unchecked<S>(drain: set::Drain<'a, K>) -> Drain<'a, K, S> {
        Drain(drain, PhantomData)
    }

    /// Returns the inner [`Drain`](set::Drain).
    pub fn into_inner(self) -> set::Drain<'a, K> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.$
    ///
    /// Equivalent to [`set::Drain::as_slice`].
    pub fn as_slice(&self) -> &Slice<K> {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a, K: EntityEquivalent + Hash> Deref for Drain<'a, K> {
    type Target = set::Drain<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: EntityEquivalent + Hash> Iterator for Drain<'a, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    fn last(self) -> Option<Self::Item> {
        self.0.last()
    }

    fn collect<C>(self) -> C
    where
        C: FromIterator<Self::Item>,
    {
        self.0.collect()
    }
}

impl<K: EntityEquivalent + Hash> DoubleEndedIterator for Drain<'_, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }
}

impl<K: EntityEquivalent + Hash> ExactSizeIterator for Drain<'_, K> {}

impl<K: EntityEquivalent + Hash> FusedIterator for Drain<'_, K> {}

impl<K: EntityEquivalent + Hash + Debug> Debug for Drain<'_, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

// SAFETY: Drain stems from a correctly behaving `IndexSet<K, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for Drain<'_, K> {}

// SAFETY: Difference stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for set::Difference<'_, K, EntityHash> {}

// SAFETY: Intersection stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for set::Intersection<'_, K, EntityHash> {}

// SAFETY: SymmetricDifference stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator
    for set::SymmetricDifference<'_, K, EntityHash, EntityHash>
{
}

// SAFETY: Union stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for set::Union<'_, K, EntityHash> {}

// SAFETY: Splice stems from a correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash, I: Iterator<Item = K>> EntitySetIterator
    for set::Splice<'_, I, K, EntityHash>
{
}
