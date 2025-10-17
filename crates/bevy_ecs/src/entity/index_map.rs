//! Contains the [`EntityIndexMap`] type, an [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
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
pub use indexmap::map::Entry;
use indexmap::{
    self,
    map::{self, IntoValues, ValuesMut},
    IndexMap,
};

use super::{Entity, EntityEquivalent, EntityHash, EntitySetIterator};

use bevy_platform::prelude::Box;

/// A [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct EntityIndexMap<V>(IndexMap<Entity, V, EntityHash>);

impl<V> EntityIndexMap<V> {
    /// Creates an empty `EntityIndexMap`.
    ///
    /// Equivalent to [`IndexMap::with_hasher(EntityHash)`].
    ///
    /// [`IndexMap::with_hasher(EntityHash)`]: indexmap::IndexMap::with_hasher
    #[inline]
    pub const fn new() -> Self {
        Self(IndexMap::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityIndexMap` with the specified capacity.
    ///
    /// Equivalent to [`IndexMap::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`IndexMap::with_capacity_and_hasher(n, EntityHash)`]: indexmap::IndexMap::with_capacity_and_hasher
    #[inline]
    pub fn with_capacity(n: usize) -> Self {
        Self(IndexMap::with_capacity_and_hasher(n, EntityHash))
    }

    /// Constructs an `EntityIndexMap` from an [`IndexMap`].
    #[inline]
    pub const fn from_index_map(set: IndexMap<Entity, V, EntityHash>) -> Self {
        Self(set)
    }

    /// Returns the inner [`IndexMap`].
    #[inline]
    pub fn into_inner(self) -> IndexMap<Entity, V, EntityHash> {
        self.0
    }

    /// Returns a slice of all the key-value pairs in the map.
    ///
    /// Equivalent to [`IndexMap::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &Slice<V> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice of all the key-value pairs in the map.
    ///
    /// Equivalent to [`IndexMap::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut Slice<V> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Slice::from_slice_unchecked_mut(self.0.as_mut_slice()) }
    }

    /// Converts into a boxed slice of all the key-value pairs in the map.
    ///
    /// Equivalent to [`IndexMap::into_boxed_slice`].
    #[inline]
    pub fn into_boxed_slice(self) -> Box<Slice<V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.0.into_boxed_slice()) }
    }

    /// Returns a slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`IndexMap::get_range`].
    #[inline]
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Slice<V>> {
        self.0.get_range(range).map(|slice|
            // SAFETY: EntityIndexSetSlice is a transparent wrapper around indexmap::set::Slice.
            unsafe { Slice::from_slice_unchecked(slice) })
    }

    /// Returns a mutable slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`IndexMap::get_range_mut`].
    #[inline]
    pub fn get_range_mut<R: RangeBounds<usize>>(&mut self, range: R) -> Option<&mut Slice<V>> {
        self.0.get_range_mut(range).map(|slice|
            // SAFETY: EntityIndexSetSlice is a transparent wrapper around indexmap::set::Slice.
            unsafe { Slice::from_slice_unchecked_mut(slice) })
    }

    /// Return an iterator over the key-value pairs of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::iter`].
    #[inline]
    pub fn iter(&self) -> Iter<'_, V> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Return a mutable iterator over the key-value pairs of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::iter_mut`].
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        IterMut(self.0.iter_mut(), PhantomData)
    }

    /// Clears the `IndexMap` in the given index range, returning those
    /// key-value pairs as a drain iterator.
    ///
    /// Equivalent to [`IndexMap::drain`].
    #[inline]
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, V> {
        Drain(self.0.drain(range), PhantomData)
    }

    /// Return an iterator over the keys of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::keys`].
    #[inline]
    pub fn keys(&self) -> Keys<'_, V> {
        Keys(self.0.keys(), PhantomData)
    }

    /// Return an owning iterator over the keys of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::into_keys`].
    #[inline]
    pub fn into_keys(self) -> IntoKeys<V> {
        IntoKeys(self.0.into_keys(), PhantomData)
    }
}

impl<V> Default for EntityIndexMap<V> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<V> Deref for EntityIndexMap<V> {
    type Target = IndexMap<Entity, V, EntityHash>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for EntityIndexMap<V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, V: Copy> Extend<(&'a Entity, &'a V)> for EntityIndexMap<V> {
    #[inline]
    fn extend<T: IntoIterator<Item = (&'a Entity, &'a V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V> Extend<(Entity, V)> for EntityIndexMap<V> {
    #[inline]
    fn extend<T: IntoIterator<Item = (Entity, V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V, const N: usize> From<[(Entity, V); N]> for EntityIndexMap<V> {
    #[inline]
    fn from(value: [(Entity, V); N]) -> Self {
        Self(IndexMap::from_iter(value))
    }
}

impl<V> FromIterator<(Entity, V)> for EntityIndexMap<V> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (Entity, V)>>(iterable: I) -> Self {
        Self(IndexMap::from_iter(iterable))
    }
}

impl<V, Q: EntityEquivalent + ?Sized> Index<&Q> for EntityIndexMap<V> {
    type Output = V;
    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<V> Index<(Bound<usize>, Bound<usize>)> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<Range<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeFrom<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeFull> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeInclusive<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeTo<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeToInclusive<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    #[inline]
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<usize> for EntityIndexMap<V> {
    type Output = V;
    #[inline]
    fn index(&self, key: usize) -> &V {
        self.0.index(key)
    }
}

impl<V, Q: EntityEquivalent + ?Sized> IndexMut<&Q> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.0.index_mut(&key.entity())
    }
}

impl<V> IndexMut<(Bound<usize>, Bound<usize>)> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<Range<usize>> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFrom<usize>> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFull> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeInclusive<usize>> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeTo<usize>> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeToInclusive<usize>> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<usize> for EntityIndexMap<V> {
    #[inline]
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.0.index_mut(key)
    }
}

impl<'a, V> IntoIterator for &'a EntityIndexMap<V> {
    type Item = (&'a Entity, &'a V);
    type IntoIter = Iter<'a, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter(), PhantomData)
    }
}

impl<'a, V> IntoIterator for &'a mut EntityIndexMap<V> {
    type Item = (&'a Entity, &'a mut V);
    type IntoIter = IterMut<'a, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.0.iter_mut(), PhantomData)
    }
}

impl<V> IntoIterator for EntityIndexMap<V> {
    type Item = (Entity, V);
    type IntoIter = IntoIter<V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<V1, V2, S2> PartialEq<IndexMap<Entity, V2, S2>> for EntityIndexMap<V1>
where
    V1: PartialEq<V2>,
    S2: BuildHasher,
{
    #[inline]
    fn eq(&self, other: &IndexMap<Entity, V2, S2>) -> bool {
        self.0.eq(other)
    }
}

impl<V1, V2> PartialEq<EntityIndexMap<V2>> for EntityIndexMap<V1>
where
    V1: PartialEq<V2>,
{
    #[inline]
    fn eq(&self, other: &EntityIndexMap<V2>) -> bool {
        self.0.eq(other)
    }
}

impl<V: Eq> Eq for EntityIndexMap<V> {}

/// A dynamically-sized slice of key-value pairs in an [`EntityIndexMap`].
///
/// Equivalent to an [`indexmap::map::Slice<V>`] whose source [`IndexMap`]
/// uses [`EntityHash`].
#[repr(transparent)]
pub struct Slice<V, S = EntityHash>(PhantomData<S>, map::Slice<Entity, V>);

impl<V> Slice<V> {
    /// Returns an empty slice.    
    ///
    /// Equivalent to [`map::Slice::new`].
    #[inline]
    pub const fn new<'a>() -> &'a Self {
        // SAFETY: The source slice is empty.
        unsafe { Self::from_slice_unchecked(map::Slice::new()) }
    }

    /// Returns an empty mutable slice.
    ///
    /// Equivalent to [`map::Slice::new_mut`].
    #[inline]
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
    #[inline]
    pub const unsafe fn from_slice_unchecked(slice: &map::Slice<Entity, V>) -> &Self {
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
    #[inline]
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut map::Slice<Entity, V>) -> &mut Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Casts `self` to the inner slice.
    #[inline]
    pub const fn as_inner(&self) -> &map::Slice<Entity, V> {
        &self.1
    }

    /// Constructs a boxed [`entity::index_map::Slice`] from a boxed [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    #[inline]
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<map::Slice<Entity, V>>) -> Box<Self> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Casts a reference to `self` to the inner slice.
    #[expect(
        clippy::borrowed_box,
        reason = "We wish to access the Box API of the inner type, without consuming it."
    )]
    #[inline]
    pub const fn as_boxed_inner(self: &Box<Self>) -> &Box<map::Slice<Entity, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &*(ptr::from_ref(self).cast::<Box<map::Slice<Entity, V>>>()) }
    }

    /// Casts `self` to the inner slice.
    #[inline]
    pub fn into_boxed_inner(self: Box<Self>) -> Box<map::Slice<Entity, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Box::from_raw(Box::into_raw(self) as *mut map::Slice<Entity, V>) }
    }

    /// Get a key-value pair by index, with mutable access to the value.
    ///
    /// Equivalent to [`map::Slice::get_index_mut`].
    #[inline]
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&Entity, &mut V)> {
        self.1.get_index_mut(index)
    }

    /// Returns a slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`map::Slice::get_range`].
    #[inline]
    pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Self> {
        self.1.get_range(range).map(|slice|
            // SAFETY: This a subslice of a valid slice.
            unsafe { Self::from_slice_unchecked(slice) })
    }

    /// Returns a mutable slice of key-value pairs in the given range of indices.
    ///
    /// Equivalent to [`map::Slice::get_range_mut`].
    #[inline]
    pub fn get_range_mut<R: RangeBounds<usize>>(&mut self, range: R) -> Option<&mut Self> {
        self.1.get_range_mut(range).map(|slice|
            // SAFETY: This a subslice of a valid slice.
            unsafe { Self::from_slice_unchecked_mut(slice) })
    }

    /// Get the first key-value pair, with mutable access to the value.
    ///
    /// Equivalent to [`map::Slice::first_mut`].
    #[inline]
    pub fn first_mut(&mut self) -> Option<(&Entity, &mut V)> {
        self.1.first_mut()
    }

    /// Get the last key-value pair, with mutable access to the value.
    ///
    /// Equivalent to [`map::Slice::last_mut`].
    #[inline]
    pub fn last_mut(&mut self) -> Option<(&Entity, &mut V)> {
        self.1.last_mut()
    }

    /// Divides one slice into two at an index.
    ///
    /// Equivalent to [`map::Slice::split_at`].
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn split_first(&self) -> Option<((&Entity, &V), &Self)> {
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
    #[inline]
    pub fn split_first_mut(&mut self) -> Option<((&Entity, &mut V), &mut Self)> {
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
    #[inline]
    pub fn split_last(&self) -> Option<((&Entity, &V), &Self)> {
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
    #[inline]
    pub fn split_last_mut(&mut self) -> Option<((&Entity, &mut V), &mut Self)> {
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
    #[inline]
    pub fn iter(&self) -> Iter<'_, V> {
        Iter(self.1.iter(), PhantomData)
    }

    /// Return an iterator over the key-value pairs of the map slice.
    ///
    /// Equivalent to [`map::Slice::iter_mut`].
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        IterMut(self.1.iter_mut(), PhantomData)
    }

    /// Return an iterator over the keys of the map slice.
    ///
    /// Equivalent to [`map::Slice::keys`].
    #[inline]
    pub fn keys(&self) -> Keys<'_, V> {
        Keys(self.1.keys(), PhantomData)
    }

    /// Return an owning iterator over the keys of the map slice.
    ///
    /// Equivalent to [`map::Slice::into_keys`].
    #[inline]
    pub fn into_keys(self: Box<Self>) -> IntoKeys<V> {
        IntoKeys(self.into_boxed_inner().into_keys(), PhantomData)
    }

    /// Return an iterator over mutable references to the values of the map slice.
    ///
    /// Equivalent to [`map::Slice::values_mut`].
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, Entity, V> {
        self.1.values_mut()
    }

    /// Return an owning iterator over the values of the map slice.
    ///
    /// Equivalent to [`map::Slice::into_values`].
    #[inline]
    pub fn into_values(self: Box<Self>) -> IntoValues<Entity, V> {
        self.into_boxed_inner().into_values()
    }
}

impl<V> Deref for Slice<V> {
    type Target = map::Slice<Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.1
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

impl<V: Clone> Clone for Box<Slice<V>> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: This a clone of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.as_boxed_inner().clone()) }
    }
}

impl<V> Default for &Slice<V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked(<&map::Slice<Entity, V>>::default()) }
    }
}

impl<V> Default for &mut Slice<V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked_mut(<&mut map::Slice<Entity, V>>::default()) }
    }
}

impl<V> Default for Box<Slice<V>> {
    #[inline]
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_boxed_slice_unchecked(<Box<map::Slice<Entity, V>>>::default()) }
    }
}

impl<V: Copy> From<&Slice<V>> for Box<Slice<V>> {
    #[inline]
    fn from(value: &Slice<V>) -> Self {
        // SAFETY: This slice is a copy of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(value.1.into()) }
    }
}

impl<V: Hash> Hash for Slice<V> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl<'a, V> IntoIterator for &'a Slice<V> {
    type Item = (&'a Entity, &'a V);
    type IntoIter = Iter<'a, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter(self.1.iter(), PhantomData)
    }
}

impl<'a, V> IntoIterator for &'a mut Slice<V> {
    type Item = (&'a Entity, &'a mut V);
    type IntoIter = IterMut<'a, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.1.iter_mut(), PhantomData)
    }
}

impl<V> IntoIterator for Box<Slice<V>> {
    type Item = (Entity, V);
    type IntoIter = IntoIter<V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.into_boxed_inner().into_iter(), PhantomData)
    }
}

impl<V: PartialOrd> PartialOrd for Slice<V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl<V: Ord> Ord for Slice<V> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(other)
    }
}

impl<V: PartialEq> PartialEq for Slice<V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<V: Eq> Eq for Slice<V> {}

impl<V> Index<(Bound<usize>, Bound<usize>)> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<Range<usize>> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeFrom<usize>> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeFull> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeInclusive<usize>> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeTo<usize>> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeToInclusive<usize>> for Slice<V> {
    type Output = Self;
    #[inline]
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<usize> for Slice<V> {
    type Output = V;
    #[inline]
    fn index(&self, key: usize) -> &V {
        self.1.index(key)
    }
}

impl<V> IndexMut<(Bound<usize>, Bound<usize>)> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<Range<usize>> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFrom<usize>> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFull> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeFull) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeInclusive<usize>> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeTo<usize>> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeToInclusive<usize>> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<usize> for Slice<V> {
    #[inline]
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.1.index_mut(key)
    }
}

/// An iterator over the entries of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::iter`] method.
/// See its documentation for more.
pub struct Iter<'a, V, S = EntityHash>(map::Iter<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Iter<'a, V> {
    /// Constructs a [`Iter<'a, V, S>`] from a [`map::Iter<'a, V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `iter` must either be empty, or have been obtained from a
    /// [`IndexMap`] using the `S` hasher.
    #[inline]
    pub const unsafe fn from_iter_unchecked<S>(iter: map::Iter<'a, Entity, V>) -> Iter<'a, V, S> {
        Iter::<'_, _, S>(iter, PhantomData)
    }

    /// Returns the inner [`Iter`](map::Iter).
    #[inline]
    pub const fn into_inner(self) -> map::Iter<'a, Entity, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::Iter::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &Slice<V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a, V> Deref for Iter<'a, V> {
    type Target = map::Iter<'a, Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a Entity, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for Iter<'_, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for Iter<'_, V> {}

impl<V> FusedIterator for Iter<'_, V> {}

impl<V> Clone for Iter<'_, V> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We are cloning an already valid `Iter`.
        unsafe { Self::from_iter_unchecked(self.0.clone()) }
    }
}

impl<V: Debug> Debug for Iter<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<V> Default for Iter<'_, V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: `Iter` is empty.
        unsafe { Self::from_iter_unchecked(Default::default()) }
    }
}

/// A mutable iterator over the entries of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::iter_mut`] method.
/// See its documentation for more.
pub struct IterMut<'a, V, S = EntityHash>(map::IterMut<'a, Entity, V>, PhantomData<S>);

impl<'a, V> IterMut<'a, V> {
    /// Constructs a [`IterMut<'a, V, S>`] from a [`map::IterMut<'a, V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `iter_mut` must either be empty, or have been obtained from a
    /// [`IndexMap`] using the `S` hasher.
    #[inline]
    pub const unsafe fn from_iter_mut_unchecked<S>(
        iter_mut: map::IterMut<'a, Entity, V>,
    ) -> IterMut<'a, V, S> {
        IterMut::<'_, _, S>(iter_mut, PhantomData)
    }

    /// Returns the inner [`IterMut`](map::IterMut).
    #[inline]
    pub const fn into_inner(self) -> map::IterMut<'a, Entity, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IterMut::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &Slice<V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IterMut::into_slice`].
    #[inline]
    pub fn into_slice(self) -> &'a mut Slice<V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.into_slice()) }
    }
}

impl<'a, V> Deref for IterMut<'a, V> {
    type Target = map::IterMut<'a, Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (&'a Entity, &'a mut V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for IterMut<'_, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for IterMut<'_, V> {}

impl<V> FusedIterator for IterMut<'_, V> {}

impl<V: Debug> Debug for IterMut<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IterMut")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<V> Default for IterMut<'_, V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: `IterMut` is empty.
        unsafe { Self::from_iter_mut_unchecked(Default::default()) }
    }
}

/// An owning iterator over the entries of an [`IndexMap`].
///
/// This `struct` is created by the [`IndexMap::into_iter`] method
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
pub struct IntoIter<V, S = EntityHash>(map::IntoIter<Entity, V>, PhantomData<S>);

impl<V> IntoIter<V> {
    /// Constructs a [`IntoIter<V, S>`] from a [`map::IntoIter<V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `into_iter` must either be empty, or have been obtained from a
    /// [`IndexMap`] using the `S` hasher.
    #[inline]
    pub const unsafe fn from_into_iter_unchecked<S>(
        into_iter: map::IntoIter<Entity, V>,
    ) -> IntoIter<V, S> {
        IntoIter::<_, S>(into_iter, PhantomData)
    }

    /// Returns the inner [`IntoIter`](map::IntoIter).
    #[inline]
    pub fn into_inner(self) -> map::IntoIter<Entity, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IntoIter::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &Slice<V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }

    /// Returns a mutable slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::IntoIter::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut Slice<V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.as_mut_slice()) }
    }
}

impl<V> Deref for IntoIter<V> {
    type Target = map::IntoIter<Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for IntoIter<V> {
    type Item = (Entity, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for IntoIter<V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for IntoIter<V> {}

impl<V> FusedIterator for IntoIter<V> {}

impl<V: Clone> Clone for IntoIter<V> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We are cloning an already valid `IntoIter`.
        unsafe { Self::from_into_iter_unchecked(self.0.clone()) }
    }
}

impl<V: Debug> Debug for IntoIter<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<V> Default for IntoIter<V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: `IntoIter` is empty.
        unsafe { Self::from_into_iter_unchecked(Default::default()) }
    }
}

/// A draining iterator over the entries of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::drain`] method.
/// See its documentation for more.
pub struct Drain<'a, V, S = EntityHash>(map::Drain<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Drain<'a, V> {
    /// Constructs a [`Drain<'a, V, S>`] from a [`map::Drain<'a, V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `drain` must either be empty, or have been obtained from a
    /// [`IndexMap`] using the `S` hasher.
    #[inline]
    pub const unsafe fn from_drain_unchecked<S>(
        drain: map::Drain<'a, Entity, V>,
    ) -> Drain<'a, V, S> {
        Drain::<'_, _, S>(drain, PhantomData)
    }

    /// Returns the inner [`Drain`](indexmap::map::Drain).
    #[inline]
    pub fn into_inner(self) -> map::Drain<'a, Entity, V> {
        self.0
    }

    /// Returns a slice of the remaining entries in the iterator.
    ///
    /// Equivalent to [`map::Drain::as_slice`](`indexmap::map::Drain::as_slice`).
    #[inline]
    pub fn as_slice(&self) -> &Slice<V> {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.as_slice()) }
    }
}

impl<'a, V> Deref for Drain<'a, V> {
    type Target = map::Drain<'a, Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for Drain<'_, V> {
    type Item = (Entity, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for Drain<'_, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for Drain<'_, V> {}

impl<V> FusedIterator for Drain<'_, V> {}

impl<V: Debug> Debug for Drain<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

/// An iterator over the keys of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::keys`] method.
/// See its documentation for more.
pub struct Keys<'a, V, S = EntityHash>(map::Keys<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Keys<'a, V> {
    /// Constructs a [`Keys<'a, V, S>`] from a [`map::Keys<'a, V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `keys` must either be empty, or have been obtained from a
    /// [`IndexMap`] using the `S` hasher.
    #[inline]
    pub const unsafe fn from_keys_unchecked<S>(keys: map::Keys<'a, Entity, V>) -> Keys<'a, V, S> {
        Keys::<'_, _, S>(keys, PhantomData)
    }

    /// Returns the inner [`Keys`](map::Keys).
    #[inline]
    pub const fn into_inner(self) -> map::Keys<'a, Entity, V> {
        self.0
    }
}

impl<'a, V, S> Deref for Keys<'a, V, S> {
    type Target = map::Keys<'a, Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for Keys<'a, V> {
    type Item = &'a Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for Keys<'_, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for Keys<'_, V> {}

impl<V> FusedIterator for Keys<'_, V> {}

impl<V> Index<usize> for Keys<'_, V> {
    type Output = Entity;

    #[inline]
    fn index(&self, index: usize) -> &Entity {
        self.0.index(index)
    }
}

impl<V> Clone for Keys<'_, V> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We are cloning an already valid `Keys`.
        unsafe { Self::from_keys_unchecked(self.0.clone()) }
    }
}

impl<V: Debug> Debug for Keys<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Keys").field(&self.0).field(&self.1).finish()
    }
}

impl<V> Default for Keys<'_, V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: `Keys` is empty.
        unsafe { Self::from_keys_unchecked(Default::default()) }
    }
}

// SAFETY: Keys stems from a correctly behaving `IndexMap<Entity, V, EntityHash>`.
unsafe impl<V> EntitySetIterator for Keys<'_, V> {}

/// An owning iterator over the keys of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::into_keys`] method.
/// See its documentation for more.
pub struct IntoKeys<V, S = EntityHash>(map::IntoKeys<Entity, V>, PhantomData<S>);

impl<V> IntoKeys<V> {
    /// Constructs a [`IntoKeys<V, S>`] from a [`map::IntoKeys<V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `into_keys` must either be empty, or have been obtained from a
    /// [`IndexMap`] using the `S` hasher.
    #[inline]
    pub const unsafe fn from_into_keys_unchecked<S>(
        into_keys: map::IntoKeys<Entity, V>,
    ) -> IntoKeys<V, S> {
        IntoKeys::<_, S>(into_keys, PhantomData)
    }

    /// Returns the inner [`IntoKeys`](map::IntoKeys).
    #[inline]
    pub fn into_inner(self) -> map::IntoKeys<Entity, V> {
        self.0
    }
}

impl<V> Deref for IntoKeys<V> {
    type Target = map::IntoKeys<Entity, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for IntoKeys<V> {
    type Item = Entity;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for IntoKeys<V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for IntoKeys<V> {}

impl<V> FusedIterator for IntoKeys<V> {}

impl<V: Debug> Debug for IntoKeys<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoKeys")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<V> Default for IntoKeys<V> {
    #[inline]
    fn default() -> Self {
        // SAFETY: `IntoKeys` is empty.
        unsafe { Self::from_into_keys_unchecked(Default::default()) }
    }
}

// SAFETY: IntoKeys stems from a correctly behaving `IndexMap<Entity, V, EntityHash>`.
unsafe impl<V> EntitySetIterator for IntoKeys<V> {}
