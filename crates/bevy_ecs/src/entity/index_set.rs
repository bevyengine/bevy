//! Contains the [`EntityIndexSet`] type, a [`IndexSet`] pre-configured to use [`EntityHash`] hashing.
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
        BitAnd, BitOr, BitXor, Bound, Deref, DerefMut, Index, Range, RangeBounds, RangeFrom,
        RangeFull, RangeInclusive, RangeTo, RangeToInclusive, Sub,
    },
    ptr,
};

use indexmap::set::{self, IndexSet};

use super::{Entity, EntityHash, EntitySetIterator};

use bevy_platform::prelude::Box;

/// An [`IndexSet`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Default)]
pub struct EntityIndexSet(pub(crate) IndexSet<Entity, EntityHash>);

impl EntityIndexSet {
    /// Creates an empty `EntityIndexSet`.
    ///
    /// Equivalent to [`IndexSet::with_hasher(EntityHash)`].
    ///
    /// [`IndexSet::with_hasher(EntityHash)`]: IndexSet::with_hasher
    pub const fn new() -> Self {
        Self(IndexSet::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityIndexSet` with the specified capacity.
    ///
    /// Equivalent to [`IndexSet::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`IndexSet::with_capacity_and_hasher(n, EntityHash)`]: IndexSet::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(IndexSet::with_capacity_and_hasher(n, EntityHash))
    }

    /// Returns the inner [`IndexSet`].
    pub fn into_inner(self) -> IndexSet<Entity, EntityHash> {
        self.0
    }

    /// Returns a slice of all the values in the set.
    ///
    /// Equivalent to [`IndexSet::as_slice`].
    pub fn as_slice(&self) -> &Slice {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Clears the `IndexSet` in the given index range, returning those values
    /// as a drain iterator.
    ///
    /// Equivalent to [`IndexSet::drain`].
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_> {
        Drain(self.0.drain(range), PhantomData)
    }

    /// Returns a slice of values in the given range of indices.
    ///
    /// Equivalent to [`IndexSet::get_range`].
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Slice> {
        self.0.get_range(range).map(|slice|
            // SAFETY: The source IndexSet uses EntityHash.
            unsafe { Slice::from_slice_unchecked(slice) })
    }

    /// Return an iterator over the values of the set, in their order.
    ///
    /// Equivalent to [`IndexSet::iter`].
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Converts into a boxed slice of all the values in the set.
    ///
    /// Equivalent to [`IndexSet::into_boxed_slice`].
    pub fn into_boxed_slice(self) -> Box<Slice> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.0.into_boxed_slice()) }
    }
}

impl Deref for EntityIndexSet {
    type Target = IndexSet<Entity, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EntityIndexSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a EntityIndexSet {
    type Item = &'a Entity;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter((&self.0).into_iter(), PhantomData)
    }
}

impl IntoIterator for EntityIndexSet {
    type Item = Entity;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl BitAnd for &EntityIndexSet {
    type Output = EntityIndexSet;

    fn bitand(self, rhs: Self) -> Self::Output {
        EntityIndexSet(self.0.bitand(&rhs.0))
    }
}

impl BitOr for &EntityIndexSet {
    type Output = EntityIndexSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        EntityIndexSet(self.0.bitor(&rhs.0))
    }
}

impl BitXor for &EntityIndexSet {
    type Output = EntityIndexSet;

    fn bitxor(self, rhs: Self) -> Self::Output {
        EntityIndexSet(self.0.bitxor(&rhs.0))
    }
}

impl Sub for &EntityIndexSet {
    type Output = EntityIndexSet;

    fn sub(self, rhs: Self) -> Self::Output {
        EntityIndexSet(self.0.sub(&rhs.0))
    }
}

impl<'a> Extend<&'a Entity> for EntityIndexSet {
    fn extend<T: IntoIterator<Item = &'a Entity>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl Extend<Entity> for EntityIndexSet {
    fn extend<T: IntoIterator<Item = Entity>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<const N: usize> From<[Entity; N]> for EntityIndexSet {
    fn from(value: [Entity; N]) -> Self {
        Self(IndexSet::from_iter(value))
    }
}

impl FromIterator<Entity> for EntityIndexSet {
    fn from_iter<I: IntoIterator<Item = Entity>>(iterable: I) -> Self {
        Self(IndexSet::from_iter(iterable))
    }
}

impl<S2> PartialEq<IndexSet<Entity, S2>> for EntityIndexSet
where
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexSet<Entity, S2>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq for EntityIndexSet {
    fn eq(&self, other: &EntityIndexSet) -> bool {
        self.0.eq(other)
    }
}

impl Eq for EntityIndexSet {}

impl Index<(Bound<usize>, Bound<usize>)> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<Range<usize>> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<RangeFrom<usize>> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<RangeFull> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<RangeInclusive<usize>> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<RangeTo<usize>> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<RangeToInclusive<usize>> for EntityIndexSet {
    type Output = Slice;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl Index<usize> for EntityIndexSet {
    type Output = Entity;
    fn index(&self, key: usize) -> &Entity {
        self.0.index(key)
    }
}

/// A dynamically-sized slice of values in an [`EntityIndexSet`].
///
/// Equivalent to an [`indexmap::set::Slice<V>`] whose source [`IndexSet`]
/// uses [`EntityHash`].
#[repr(transparent)]
pub struct Slice<S = EntityHash>(PhantomData<S>, set::Slice<Entity>);

impl Slice {
    /// Returns an empty slice.
    ///
    /// Equivalent to [`set::Slice::new`].
    pub const fn new<'a>() -> &'a Self {
        // SAFETY: The source slice is empty.
        unsafe { Self::from_slice_unchecked(set::Slice::new()) }
    }

    /// Constructs a [`entity::index_set::Slice`] from a [`indexmap::set::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexSet`] using [`EntityHash`].
    ///
    /// [`entity::index_set::Slice`]: `crate::entity::index_set::Slice`
    pub const unsafe fn from_slice_unchecked(slice: &set::Slice<Entity>) -> &Self {
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
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut set::Slice<Entity>) -> &mut Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    pub const fn as_inner(&self) -> &set::Slice<Entity> {
        &self.1
    }

    /// Constructs a boxed [`entity::index_set::Slice`] from a boxed [`indexmap::set::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexSet`] using [`EntityHash`].
    ///
    /// [`entity::index_set::Slice`]: `crate::entity::index_set::Slice`
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<set::Slice<Entity>>) -> Box<Self> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Casts a reference to `self` to the inner slice.
    #[expect(
        clippy::borrowed_box,
        reason = "We wish to access the Box API of the inner type, without consuming it."
    )]
    pub fn as_boxed_inner(self: &Box<Self>) -> &Box<set::Slice<Entity>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { &*(ptr::from_ref(self).cast::<Box<set::Slice<Entity>>>()) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_boxed_inner(self: Box<Self>) -> Box<set::Slice<Entity>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::set::Slice.
        unsafe { Box::from_raw(Box::into_raw(self) as *mut set::Slice<Entity>) }
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
    pub fn split_first(&self) -> Option<(&Entity, &Self)> {
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
    pub fn split_last(&self) -> Option<(&Entity, &Self)> {
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
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.1.iter(), PhantomData)
    }
}

impl Deref for Slice {
    type Target = set::Slice<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a> IntoIterator for &'a Slice {
    type IntoIter = Iter<'a>;
    type Item = &'a Entity;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for Box<Slice> {
    type IntoIter = IntoIter;
    type Item = Entity;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.into_boxed_inner().into_iter(), PhantomData)
    }
}

impl Clone for Box<Slice> {
    fn clone(&self) -> Self {
        // SAFETY: This is a clone of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.as_boxed_inner().clone()) }
    }
}

impl Default for &Slice {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked(<&set::Slice<Entity>>::default()) }
    }
}

impl Default for Box<Slice> {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_boxed_slice_unchecked(<Box<set::Slice<Entity>>>::default()) }
    }
}

impl<V: Debug> Debug for Slice<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Slice")
            .field(&self.0)
            .field(&&self.1)
            .finish()
    }
}

impl From<&Slice> for Box<Slice> {
    fn from(value: &Slice) -> Self {
        // SAFETY: This slice is a copy of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(value.1.into()) }
    }
}

impl Hash for Slice {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl PartialOrd for Slice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Slice {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(other)
    }
}

impl PartialEq for Slice {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for Slice {}

impl Index<(Bound<usize>, Bound<usize>)> for Slice {
    type Output = Self;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<Range<usize>> for Slice {
    type Output = Self;
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<RangeFrom<usize>> for Slice {
    type Output = Slice;
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<RangeFull> for Slice {
    type Output = Self;
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<RangeInclusive<usize>> for Slice {
    type Output = Self;
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<RangeTo<usize>> for Slice {
    type Output = Self;
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<RangeToInclusive<usize>> for Slice {
    type Output = Self;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl Index<usize> for Slice {
    type Output = Entity;
    fn index(&self, key: usize) -> &Entity {
        self.1.index(key)
    }
}

/// An iterator over the items of an [`EntityIndexSet`].
///
/// This struct is created by the [`iter`] method on [`EntityIndexSet`]. See its documentation for more.
///
/// [`iter`]: EntityIndexSet::iter
pub struct Iter<'a, S = EntityHash>(set::Iter<'a, Entity>, PhantomData<S>);

impl<'a> Iter<'a> {
    /// Returns the inner [`Iter`](set::Iter).
    pub fn into_inner(self) -> set::Iter<'a, Entity> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`set::Iter::as_slice`].
    pub fn as_slice(&self) -> &Slice {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a> Deref for Iter<'a> {
    type Target = set::Iter<'a, Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl FusedIterator for Iter<'_> {}

impl Clone for Iter<'_> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl Debug for Iter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl Default for Iter<'_> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: Iter stems from a correctly behaving `IndexSet<Entity, EntityHash>`.
unsafe impl EntitySetIterator for Iter<'_> {}

/// Owning iterator over the items of an [`EntityIndexSet`].
///
/// This struct is created by the [`into_iter`] method on [`EntityIndexSet`] (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: EntityIndexSet::into_iter
pub struct IntoIter<S = EntityHash>(set::IntoIter<Entity>, PhantomData<S>);

impl IntoIter {
    /// Returns the inner [`IntoIter`](set::IntoIter).
    pub fn into_inner(self) -> set::IntoIter<Entity> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`set::IntoIter::as_slice`].
    pub fn as_slice(&self) -> &Slice {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl Deref for IntoIter {
    type Target = set::IntoIter<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Iterator for IntoIter {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl DoubleEndedIterator for IntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for IntoIter {}

impl FusedIterator for IntoIter {}

impl Clone for IntoIter {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl Debug for IntoIter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl Default for IntoIter {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: IntoIter stems from a correctly behaving `IndexSet<Entity, EntityHash>`.
unsafe impl EntitySetIterator for IntoIter {}

/// A draining iterator over the items of an [`EntityIndexSet`].
///
/// This struct is created by the [`drain`] method on [`EntityIndexSet`]. See its documentation for more.
///
/// [`drain`]: EntityIndexSet::drain
pub struct Drain<'a, S = EntityHash>(set::Drain<'a, Entity>, PhantomData<S>);

impl<'a> Drain<'a> {
    /// Returns the inner [`Drain`](set::Drain).
    pub fn into_inner(self) -> set::Drain<'a, Entity> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.$
    ///
    /// Equivalent to [`set::Drain::as_slice`].
    pub fn as_slice(&self) -> &Slice {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a> Deref for Drain<'a> {
    type Target = set::Drain<'a, Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Iterator for Drain<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl DoubleEndedIterator for Drain<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for Drain<'_> {}

impl FusedIterator for Drain<'_> {}

impl Debug for Drain<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

// SAFETY: Drain stems from a correctly behaving `IndexSet<Entity, EntityHash>`.
unsafe impl EntitySetIterator for Drain<'_> {}

// SAFETY: Difference stems from two correctly behaving `IndexSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for set::Difference<'_, Entity, EntityHash> {}

// SAFETY: Intersection stems from two correctly behaving `IndexSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for set::Intersection<'_, Entity, EntityHash> {}

// SAFETY: SymmetricDifference stems from two correctly behaving `IndexSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for set::SymmetricDifference<'_, Entity, EntityHash, EntityHash> {}

// SAFETY: Union stems from two correctly behaving `IndexSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for set::Union<'_, Entity, EntityHash> {}

// SAFETY: Splice stems from a correctly behaving `IndexSet<Entity, EntityHash>`s.
unsafe impl<I: Iterator<Item = Entity>> EntitySetIterator
    for set::Splice<'_, I, Entity, EntityHash>
{
}
