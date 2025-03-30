//! Contains the [`EntityEquivalentIndexMap`] type, an [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around `indexmap`'s [`IndexMap`] that is more performant for [`Entity`] keys.

use core::{
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::{
        Bound, Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull,
        RangeInclusive, RangeTo, RangeToInclusive,
    },
    ptr,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use indexmap::map::{self, IndexMap, IntoValues, ValuesMut};

use super::{Entity, EntityHash, EntitySetIterator, TrustedBuildHasher, TrustedEntityBorrow};

use bevy_platform_support::prelude::Box;

/// A [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct EntityEquivalentIndexMap<K: TrustedEntityBorrow + Hash, V>(
    pub(crate) IndexMap<K, V, EntityHash>,
)
where
    EntityHash: TrustedBuildHasher<K>;

/// An [`IndexMap`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityIndexMap<V> = EntityEquivalentIndexMap<Entity, V>;

impl<K: TrustedEntityBorrow + Hash, V> EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Creates an empty `EntityEquivalentIndexMap`.
    ///
    /// Equivalent to [`IndexMap::with_hasher(EntityHash)`].
    ///
    /// [`IndexMap::with_hasher(EntityHash)`]: IndexMap::with_hasher
    pub const fn new() -> Self {
        Self(IndexMap::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityEquivalentIndexMap` with the specified capacity.
    ///
    /// Equivalent to [`IndexMap::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`IndexMap:with_capacity_and_hasher(n, EntityHash)`]: IndexMap::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(IndexMap::with_capacity_and_hasher(n, EntityHash))
    }

    /// Returns the inner [`IndexMap`].
    pub fn into_inner(self) -> IndexMap<K, V, EntityHash> {
        self.0
    }

    /// Returns a slice of all the key-value pairs in the map.
    ///
    /// Equivalent to [`IndexMap::as_slice`].
    pub fn as_slice(&self) -> &Slice<K, V> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice of all the key-value pairs in the map.
    ///
    /// Equivalent to [`IndexMap::as_mut_slice`].
    pub fn as_mut_slice(&mut self) -> &mut Slice<K, V> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Slice::from_slice_unchecked_mut(self.0.as_mut_slice()) }
    }

    /// Converts into a boxed slice of all the key-value pairs in the map.
    ///
    /// Equivalent to [`IndexMap::into_boxed_slice`].
    pub fn into_boxed_slice(self) -> Box<Slice<K, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.0.into_boxed_slice()) }
    }

    /// Returns a slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`IndexMap::get_range`].
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Slice<K, V>> {
        self.0.get_range(range).map(|slice|
            // SAFETY: EntityIndexSetSlice is a transparent wrapper around indexmap::set::Slice.
            unsafe { Slice::from_slice_unchecked(slice) })
    }

    /// Returns a mutable slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`IndexMap::get_range_mut`].
    pub fn get_range_mut<R: RangeBounds<usize>>(&mut self, range: R) -> Option<&mut Slice<K, V>> {
        self.0.get_range_mut(range).map(|slice|
            // SAFETY: EntityIndexSetSlice is a transparent wrapper around indexmap::set::Slice.
            unsafe { Slice::from_slice_unchecked_mut(slice) })
    }

    /// Return an iterator over the key-value pairs of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::iter`].
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Return a mutable iterator over the key-value pairs of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::iter_mut`].
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(self.0.iter_mut(), PhantomData)
    }

    /// Clears the `IndexMap` in the given index range, returning those
    /// key-value pairs as a drain iterator.
    ///
    /// Equivalent to [`IndexMap::drain`].
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, K, V> {
        Drain(self.0.drain(range), PhantomData)
    }

    /// Return an iterator over the keys of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::keys`].
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.0.keys(), PhantomData)
    }

    /// Return an owning iterator over the keys of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::into_keys`].
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys(self.0.into_keys(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Deref for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = IndexMap<K, V, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DerefMut for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash + Copy, V: Copy> Extend<(&'a K, &'a V)>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Extend<(K, V)> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash, V, const N: usize> From<[(K, V); N]>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from(value: [(K, V); N]) -> Self {
        Self(IndexMap::from_iter(value))
    }
}

impl<K: TrustedEntityBorrow + Hash, V> FromIterator<(K, V)> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        Self(IndexMap::from_iter(iterable))
    }
}

// `TrustedEntityBorrow` does not guarantee maintained equality on conversions from one implementer to another,
// so we restrict this impl to only keys of type `Entity`.
impl<V, Q: TrustedEntityBorrow + ?Sized> Index<&Q> for EntityIndexMap<V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<(Bound<usize>, Bound<usize>)>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<Range<usize>> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeFrom<usize>> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeFull> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeInclusive<usize>>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeTo<usize>> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeToInclusive<usize>>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Slice<K, V>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<usize> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = V;
    fn index(&self, key: usize) -> &V {
        self.0.index(key)
    }
}

impl<V, Q: TrustedEntityBorrow + ?Sized> IndexMut<&Q> for EntityEquivalentIndexMap<Entity, V> {
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.0.index_mut(&key.entity())
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<(Bound<usize>, Bound<usize>)>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<Range<usize>> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeFrom<usize>> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeFull> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeInclusive<usize>>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeTo<usize>> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeToInclusive<usize>>
    for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<usize> for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.0.index_mut(key)
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> IntoIterator for &'a EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter(), PhantomData)
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> IntoIterator for &'a mut EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.0.iter_mut(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IntoIterator for EntityEquivalentIndexMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash, V1, V2, S2> PartialEq<IndexMap<K, V2, S2>>
    for EntityEquivalentIndexMap<K, V1>
where
    EntityHash: TrustedBuildHasher<K>,
    V1: PartialEq<V2>,
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexMap<K, V2, S2>) -> bool {
        self.0.eq(other)
    }
}

impl<K: TrustedEntityBorrow + Hash, V1, V2> PartialEq<EntityEquivalentIndexMap<K, V2>>
    for EntityEquivalentIndexMap<K, V1>
where
    EntityHash: TrustedBuildHasher<K>,
    V1: PartialEq<V2>,
{
    fn eq(&self, other: &EntityEquivalentIndexMap<K, V2>) -> bool {
        self.0.eq(other)
    }
}

impl<K: TrustedEntityBorrow + Hash, V: Eq> Eq for EntityEquivalentIndexMap<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// A dynamically-sized slice of key-value pairs in an [`EntityEquivalentIndexMap`].
///
/// Equivalent to an [`indexmap::map::Slice<K, V>`] whose source [`IndexMap`]
/// uses [`EntityHash`].
#[repr(transparent)]
pub struct Slice<K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    PhantomData<S>,
    map::Slice<K, V>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<K: TrustedEntityBorrow + Hash, V> Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns an empty slice.    
    ///
    /// Equivalent to [`map::Slice::new`].
    pub const fn new<'a>() -> &'a Self {
        // SAFETY: The source slice is empty.
        unsafe { Self::from_slice_unchecked(map::Slice::new()) }
    }

    /// Returns an empty mutable slice.
    ///
    /// Equivalent to [`map::Slice::new_mut`].
    pub fn new_mut<'a>() -> &'a mut Self {
        // SAFETY: The source slice is empty.
        unsafe { Self::from_slice_unchecked_mut(map::Slice::new_mut()) }
    }

    /// Constructs a [`entity::index_map::Slice`] from a [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    pub const unsafe fn from_slice_unchecked(slice: &map::Slice<K, V>) -> &Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &*(ptr::from_ref(slice) as *const Self) }
    }

    /// Constructs a [`entity::index_map::Slice`] from a [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut map::Slice<K, V>) -> &mut Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    pub const fn as_inner(&self) -> &map::Slice<K, V> {
        &self.1
    }

    /// Constructs a boxed [`entity::index_map::Slice`] from a boxed [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<map::Slice<K, V>>) -> Box<Self> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Casts a reference to `self` to the inner slice.
    #[expect(
        clippy::borrowed_box,
        reason = "We wish to access the Box API of the inner type, without consuming it."
    )]
    pub fn as_boxed_inner(self: &Box<Self>) -> &Box<map::Slice<K, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &*(ptr::from_ref(self).cast::<Box<map::Slice<K, V>>>()) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_boxed_inner(self: Box<Self>) -> Box<map::Slice<K, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Box::from_raw(Box::into_raw(self) as *mut map::Slice<K, V>) }
    }

    /// Get a key-value pair by index, with mutable access to the value.
    ///
    /// Equivalent to [`map::Slice::get_index_mut`].
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
        self.1.get_index_mut(index)
    }

    /// Returns a slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`map::Slice::get_range`].
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Self> {
        self.1.get_range(range).map(|slice|
            // SAFETY: This a subslice of a valid slice.
            unsafe { Self::from_slice_unchecked(slice) })
    }

    /// Returns a mutable slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`map::Slice::get_range_mut`].
    pub fn get_range_mut<R: RangeBounds<usize>>(&mut self, range: R) -> Option<&mut Self> {
        self.1.get_range_mut(range).map(|slice|
            // SAFETY: This a subslice of a valid slice.
            unsafe { Self::from_slice_unchecked_mut(slice) })
    }

    /// Get the first key-value pair, with mutable access to the value.
    ///
    /// Equivalent to [`map::Slice::first_mut`].
    pub fn first_mut(&mut self) -> Option<(&K, &mut V)> {
        self.1.first_mut()
    }

    /// Get the last key-value pair, with mutable access to the value.
    ///
    /// Equivalent to [`map::Slice::last_mut`].
    pub fn last_mut(&mut self) -> Option<(&K, &mut V)> {
        self.1.last_mut()
    }

    /// Divides one slice into two at an index.
    ///
    /// Equivalent to [`map::Slice::split_at`].
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

    /// Divides one mutable slice into two at an index.
    ///
    /// Equivalent to [`map::Slice::split_at_mut`].
    pub fn split_at_mut(&mut self, index: usize) -> (&mut Self, &mut Self) {
        let (slice_1, slice_2) = self.1.split_at_mut(index);
        // SAFETY: These are subslices of a valid slice.
        unsafe {
            (
                Self::from_slice_unchecked_mut(slice_1),
                Self::from_slice_unchecked_mut(slice_2),
            )
        }
    }

    /// Returns the first key-value pair and the rest of the slice,
    /// or `None` if it is empty.
    ///
    /// Equivalent to [`map::Slice::split_first`].
    pub fn split_first(&self) -> Option<((&K, &V), &Self)> {
        self.1.split_first().map(|(first, rest)| {
            (
                first,
                // SAFETY: This a subslice of a valid slice.
                unsafe { Self::from_slice_unchecked(rest) },
            )
        })
    }

    /// Returns the first key-value pair and the rest of the slice,
    /// with mutable access to the value, or `None` if it is empty.
    ///
    /// Equivalent to [`map::Slice::split_first_mut`].
    pub fn split_first_mut(&mut self) -> Option<((&K, &mut V), &mut Self)> {
        self.1.split_first_mut().map(|(first, rest)| {
            (
                first,
                // SAFETY: This a subslice of a valid slice.
                unsafe { Self::from_slice_unchecked_mut(rest) },
            )
        })
    }

    /// Returns the last key-value pair and the rest of the slice,
    /// or `None` if it is empty.
    ///
    /// Equivalent to [`map::Slice::split_last`].
    pub fn split_last(&self) -> Option<((&K, &V), &Self)> {
        self.1.split_last().map(|(last, rest)| {
            (
                last,
                // SAFETY: This a subslice of a valid slice.
                unsafe { Self::from_slice_unchecked(rest) },
            )
        })
    }

    /// Returns the last key-value pair and the rest of the slice,
    /// with mutable access to the value, or `None` if it is empty.
    ///
    /// Equivalent to [`map::Slice::split_last_mut`].
    pub fn split_last_mut(&mut self) -> Option<((&K, &mut V), &mut Self)> {
        self.1.split_last_mut().map(|(last, rest)| {
            (
                last,
                // SAFETY: This a subslice of a valid slice.
                unsafe { Self::from_slice_unchecked_mut(rest) },
            )
        })
    }

    /// Return an iterator over the key-value pairs of the map slice.
    ///
    /// Equivalent to [`map::Slice::iter`].
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.1.iter(), PhantomData)
    }

    /// Return an iterator over the key-value pairs of the map slice.
    ///
    /// Equivalent to [`map::Slice::iter_mut`].
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(self.1.iter_mut(), PhantomData)
    }

    /// Return an iterator over the keys of the map slice.
    ///
    /// Equivalent to [`map::Slice::keys`].
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.1.keys(), PhantomData)
    }

    /// Return an owning iterator over the keys of the map slice.
    ///
    /// Equivalent to [`map::Slice::into_keys`].
    pub fn into_keys(self: Box<Self>) -> IntoKeys<K, V> {
        IntoKeys(self.into_boxed_inner().into_keys(), PhantomData)
    }

    /// Return an iterator over mutable references to the the values of the map slice.
    ///
    /// Equivalent to [`map::Slice::values_mut`].
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        self.1.values_mut()
    }

    /// Return an owning iterator over the values of the map slice.
    ///
    /// Equivalent to [`map::Slice::into_values`].
    pub fn into_values(self: Box<Self>) -> IntoValues<K, V> {
        self.into_boxed_inner().into_values()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Deref for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::Slice<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for Slice<K, V>
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

impl<K: TrustedEntityBorrow + Hash + Clone, V: Clone> Clone for Box<Slice<K, V>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        // SAFETY: This a clone of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.as_boxed_inner().clone()) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for &Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked(<&map::Slice<K, V>>::default()) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for &mut Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked_mut(<&mut map::Slice<K, V>>::default()) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for Box<Slice<K, V>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_boxed_slice_unchecked(<Box<map::Slice<K, V>>>::default()) }
    }
}

impl<K: TrustedEntityBorrow + Hash + Copy, V: Copy> From<&Slice<K, V>> for Box<Slice<K, V>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from(value: &Slice<K, V>) -> Self {
        // SAFETY: This slice is a copy of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(value.1.into()) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V: Hash> Hash for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> IntoIterator for &'a Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.1.iter(), PhantomData)
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> IntoIterator for &'a mut Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.1.iter_mut(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IntoIterator for Box<Slice<K, V>>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.into_boxed_inner().into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + PartialOrd, V: PartialOrd> PartialOrd for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl<K: TrustedEntityBorrow + Hash + Ord, V: Ord> Ord for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(other)
    }
}

impl<K: TrustedEntityBorrow + Hash, V: PartialEq> PartialEq for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<K: TrustedEntityBorrow + Hash, V: Eq> Eq for Slice<K, V> where EntityHash: TrustedBuildHasher<K>
{}

impl<K: TrustedEntityBorrow + Hash, V> Index<(Bound<usize>, Bound<usize>)> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<Range<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeFrom<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeFull> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeInclusive<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeTo<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<RangeToInclusive<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = Self;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Index<usize> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = V;
    fn index(&self, key: usize) -> &V {
        self.1.index(key)
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<(Bound<usize>, Bound<usize>)> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<Range<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeFrom<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeFull> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeFull) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeInclusive<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeTo<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<RangeToInclusive<usize>> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IndexMut<usize> for Slice<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.1.index_mut(key)
    }
}

/// An iterator over the entries of an [`EntityEquivalentIndexMap`].
///
/// This `struct` is created by the [`EntityEquivalentIndexMap::iter`] method.
/// See its documentation for more.
pub struct Iter<'a, K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    map::Iter<'a, K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash, V> Iter<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`Iter`](map::Iter).
    pub fn into_inner(self) -> map::Iter<'a, K, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::Iter::as_slice`].
    pub fn as_slice(&self) -> &Slice<K, V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Deref for Iter<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::Iter<'a, K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Iterator for Iter<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DoubleEndedIterator for Iter<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for Iter<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for Iter<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> Clone for Iter<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for Iter<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for Iter<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// A mutable iterator over the entries of an [`EntityEquivalentIndexMap`].
///
/// This `struct` is created by the [`EntityEquivalentIndexMap::iter_mut`] method.
/// See its documentation for more.
pub struct IterMut<'a, K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    map::IterMut<'a, K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash, V> IterMut<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`IterMut`](map::IterMut).
    pub fn into_inner(self) -> map::IterMut<'a, K, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IterMut::as_slice`].
    pub fn as_slice(&self) -> &Slice<K, V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IterMut::into_slice`].
    pub fn into_slice(self) -> &'a mut Slice<K, V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.into_slice()) }
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Deref for IterMut<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::IterMut<'a, K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Iterator for IterMut<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DoubleEndedIterator for IterMut<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for IterMut<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for IterMut<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for IterMut<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IterMut")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for IterMut<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// An owning iterator over the entries of an [`IndexMap`].
///
/// This `struct` is created by the [`IndexMap::into_iter`] method
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
pub struct IntoIter<K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    map::IntoIter<K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<K: TrustedEntityBorrow + Hash, V> IntoIter<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`IntoIter`](map::IntoIter).
    pub fn into_inner(self) -> map::IntoIter<K, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IntoIter::as_slice`].
    pub fn as_slice(&self) -> &Slice<K, V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IntoIter::as_mut_slice`].
    pub fn as_mut_slice(&mut self) -> &mut Slice<K, V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.as_mut_slice()) }
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Deref for IntoIter<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::IntoIter<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Iterator for IntoIter<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DoubleEndedIterator for IntoIter<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for IntoIter<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for IntoIter<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash + Clone, V: Clone> Clone for IntoIter<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for IntoIter<K, V>
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

impl<K: TrustedEntityBorrow + Hash, V> Default for IntoIter<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// A draining iterator over the entries of an [`EntityEquivalentIndexMap`].
///
/// This `struct` is created by the [`EntityEquivalentIndexMap::drain`] method.
/// See its documentation for more.
pub struct Drain<'a, K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    map::Drain<'a, K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash, V> Drain<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`Drain`](map::Drain).
    pub fn into_inner(self) -> map::Drain<'a, K, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::Drain::as_slice`].
    pub fn as_slice(&self) -> &Slice<K, V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Deref for Drain<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::Drain<'a, K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Iterator for Drain<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DoubleEndedIterator for Drain<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for Drain<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for Drain<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for Drain<'_, K, V>
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

/// An iterator over the keys of an [`EntityEquivalentIndexMap`].
///
/// This `struct` is created by the [`EntityEquivalentIndexMap::keys`] method.
/// See its documentation for more.
pub struct Keys<'a, K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    map::Keys<'a, K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash, V> Keys<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`Keys`](map::Keys).
    pub fn into_inner(self) -> map::Keys<'a, K, V> {
        self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V, S> Deref for Keys<'a, K, V, S>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::Keys<'a, K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Iterator for Keys<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DoubleEndedIterator for Keys<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for Keys<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for Keys<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> Index<usize> for Keys<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = K;

    fn index(&self, index: usize) -> &K {
        self.0.index(index)
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone, V> Clone for Keys<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for Keys<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Keys").field(&self.0).field(&self.1).finish()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for Keys<'_, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: Keys stems from a correctly behaving `IndexMap<K, V, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash, V> EntitySetIterator for Keys<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// An owning iterator over the keys of an [`EntityEquivalentIndexMap`].
///
/// This `struct` is created by the [`EntityEquivalentIndexMap::into_keys`] method.
/// See its documentation for more.
pub struct IntoKeys<K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    map::IntoKeys<K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<K: TrustedEntityBorrow + Hash, V> IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`IntoKeys`](map::IntoKeys).
    pub fn into_inner(self) -> map::IntoKeys<K, V> {
        self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Deref for IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = map::IntoKeys<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Iterator for IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DoubleEndedIterator for IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for IntoKeys<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for IntoKeys<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash + Debug, V: Debug> Debug for IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoKeys")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Default for IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: IntoKeys stems from a correctly behaving `IndexMap<K, V, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash, V> EntitySetIterator for IntoKeys<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}
