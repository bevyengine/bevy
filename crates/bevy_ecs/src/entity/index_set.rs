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
        BitAnd, BitOr, BitXor, Bound, Deref, DerefMut, Index, Range, RangeBounds, RangeFrom,
        RangeFull, RangeInclusive, RangeTo, RangeToInclusive, Sub,
    },
    ptr,
};

use indexmap::set::{self, IndexSet};

use super::{Entity, EntityHash, EntitySetIterator, TrustedBuildHasher, TrustedEntityBorrow};

use bevy_platform_support::prelude::Box;

/// An [`IndexSet`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Default)]
pub struct EntityEquivalentIndexSet<K: TrustedEntityBorrow + Hash>(
    pub(crate) IndexSet<K, EntityHash>,
)
where
    EntityHash: TrustedBuildHasher<K>;

/// An [`IndexSet`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityIndexSet = EntityEquivalentIndexSet<Entity>;

impl<K: TrustedEntityBorrow + Hash> EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
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
}

impl<K: TrustedEntityBorrow + Hash> Deref for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = IndexSet<K, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash> DerefMut for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> IntoIterator for &'a EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = &'a K;

    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        Iter((&self.0).into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash> IntoIterator for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = K;

    type IntoIter = IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitAnd for &EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentIndexSet<K>;

    fn bitand(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.bitand(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitOr for &EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentIndexSet<K>;

    fn bitor(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.bitor(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitXor for &EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentIndexSet<K>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.bitxor(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> Sub for &EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentIndexSet<K>;

    fn sub(self, rhs: Self) -> Self::Output {
        EntityEquivalentIndexSet(self.0.sub(&rhs.0))
    }
}

impl<'a, K: TrustedEntityBorrow + Hash + Copy> Extend<&'a K> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = &'a K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash> Extend<K> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash, const N: usize> From<[K; N]> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from(value: [K; N]) -> Self {
        Self(IndexSet::from_iter(value))
    }
}

impl<K: TrustedEntityBorrow + Hash> FromIterator<K> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from_iter<I: IntoIterator<Item = K>>(iterable: I) -> Self {
        Self(IndexSet::from_iter(iterable))
    }
}

impl<K: TrustedEntityBorrow + Hash, S2> PartialEq<IndexSet<K, S2>> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexSet<K, S2>) -> bool {
        self.0.eq(other)
    }
}

impl<K: TrustedEntityBorrow + Hash> PartialEq for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn eq(&self, other: &EntityEquivalentIndexSet<K>) -> bool {
        self.0.eq(other)
    }
}

impl<K: TrustedEntityBorrow + Hash> Eq for EntityEquivalentIndexSet<K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash> Index<(Bound<usize>, Bound<usize>)>
    for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<Range<usize>> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeFrom<usize>> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeFull> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeInclusive<usize>> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeTo<usize>> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeToInclusive<usize>> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexSet uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<usize> for EntityEquivalentIndexSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
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
pub struct Slice<K: TrustedEntityBorrow + Hash, S = EntityHash>(PhantomData<S>, set::Slice<K>)
where
    EntityHash: TrustedBuildHasher<K>;

impl<K: TrustedEntityBorrow + Hash> Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
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
    pub fn as_boxed_inner(self: &Box<Self>) -> &Box<set::Slice<K>> {
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

impl<K: TrustedEntityBorrow + Hash> Deref for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = set::Slice<K>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> IntoIterator for &'a Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type IntoIter = Iter<'a, K>;
    type Item = &'a K;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: TrustedEntityBorrow + Hash> IntoIterator for Box<Slice<K>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type IntoIter = IntoIter<K>;
    type Item = K;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.into_boxed_inner().into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> Clone for Box<Slice<K>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        // SAFETY: This is a clone of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.as_boxed_inner().clone()) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Default for &Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked(<&set::Slice<K>>::default()) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Default for Box<Slice<K>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_boxed_slice_unchecked(<Box<set::Slice<K>>>::default()) }
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug> Debug for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Slice")
            .field(&self.0)
            .field(&&self.1)
            .finish()
    }
}

impl<K: TrustedEntityBorrow + Hash + Copy> From<&Slice<K>> for Box<Slice<K>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from(value: &Slice<K>) -> Self {
        // SAFETY: This slice is a copy of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(value.1.into()) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Hash for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl<K: TrustedEntityBorrow + Hash + PartialOrd> PartialOrd for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(other)
    }
}

impl<K: TrustedEntityBorrow + Hash + Ord> Ord for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(other)
    }
}

impl<K: TrustedEntityBorrow + Hash> PartialEq for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<K: TrustedEntityBorrow + Hash> Eq for Slice<K> where EntityHash: TrustedBuildHasher<K> {}

impl<K: TrustedEntityBorrow + Hash> Index<(Bound<usize>, Bound<usize>)> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<Range<usize>> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeFrom<usize>> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K>;
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeFull> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeInclusive<usize>> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeTo<usize>> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<RangeToInclusive<usize>> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash> Index<usize> for Slice<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
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
pub struct Iter<'a, K: TrustedEntityBorrow + Hash, S = EntityHash>(
    set::Iter<'a, K>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash> Iter<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`Iter`](set::Iter).
    pub fn into_inner(self) -> set::Iter<'a, K> {
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

impl<'a, K: TrustedEntityBorrow + Hash> Deref for Iter<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = set::Iter<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> Iterator for Iter<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash> DoubleEndedIterator for Iter<'_, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash> ExactSizeIterator for Iter<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash> FusedIterator for Iter<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash> Clone for Iter<'_, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug> Debug for Iter<'_, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<K: TrustedEntityBorrow + Hash> Default for Iter<'_, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: Iter stems from a correctly behaving `IndexSet<K, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for Iter<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// Owning iterator over the items of an [`EntityEquivalentIndexSet`].
///
/// This struct is created by the [`into_iter`] method on [`EntityEquivalentIndexSet`] (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: EntityEquivalentIndexSet::into_iter
pub struct IntoIter<K: TrustedEntityBorrow + Hash, S = EntityHash>(
    set::IntoIter<K>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<K: TrustedEntityBorrow + Hash> IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
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

impl<K: TrustedEntityBorrow + Hash> Deref for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = set::IntoIter<K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash> Iterator for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash> DoubleEndedIterator for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash> ExactSizeIterator for IntoIter<K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash> FusedIterator for IntoIter<K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash + Clone> Clone for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug> Debug for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<K: TrustedEntityBorrow + Hash> Default for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: IntoIter stems from a correctly behaving `IndexSet<K, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for IntoIter<K> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// A draining iterator over the items of an [`EntityEquivalentIndexSet`].
///
/// This struct is created by the [`drain`] method on [`EntityEquivalentIndexSet`]. See its documentation for more.
///
/// [`drain`]: EntityEquivalentIndexSet::drain
pub struct Drain<'a, K: TrustedEntityBorrow + Hash, S = EntityHash>(
    set::Drain<'a, K>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash> Drain<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
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

impl<'a, K: TrustedEntityBorrow + Hash> Deref for Drain<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = set::Drain<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> Iterator for Drain<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash> DoubleEndedIterator for Drain<'_, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash> ExactSizeIterator for Drain<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash> FusedIterator for Drain<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash + Debug> Debug for Drain<'_, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

// SAFETY: Drain stems from a correctly behaving `IndexSet<K, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for Drain<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

// SAFETY: Difference stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator
    for set::Difference<'_, K, EntityHash>
{
}

// SAFETY: Intersection stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator
    for set::Intersection<'_, K, EntityHash>
{
}

// SAFETY: SymmetricDifference stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator
    for set::SymmetricDifference<'_, K, EntityHash, EntityHash>
{
}

// SAFETY: Union stems from two correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for set::Union<'_, K, EntityHash> {}

// SAFETY: Splice stems from a correctly behaving `IndexSet<K, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash, I: Iterator<Item = K>> EntitySetIterator
    for set::Splice<'_, I, K, EntityHash>
{
}
