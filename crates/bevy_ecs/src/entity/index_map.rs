use core::{
    fmt::{self, Debug, Formatter},
    hash::BuildHasher,
    iter::FusedIterator,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use indexmap::map::{self, IndexMap};

use super::{Entity, EntityHash, EntitySetIterator, TrustedEntityBorrow};

/// A [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct EntityIndexMap<V>(pub(crate) IndexMap<Entity, V, EntityHash>);

impl<V> EntityIndexMap<V> {
    /// Creates an empty `EntityIndexMap`.
    ///
    /// Equivalent to [`IndexMap::with_hasher(EntityHash)`].
    ///
    /// [`IndexMap::with_hasher(EntityHash)`]: IndexMap::with_hasher
    pub const fn new() -> Self {
        Self(IndexMap::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityIndexMap` with the specified capacity.
    ///
    /// Equivalent to [`IndexMap::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`IndexMap:with_capacity_and_hasher(n, EntityHash)`]: IndexMap::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(IndexMap::with_capacity_and_hasher(n, EntityHash))
    }

    /// Returns the inner [`IndexMap`].
    pub fn into_inner(self) -> IndexMap<Entity, V, EntityHash> {
        self.0
    }

    /// Return an iterator over the key-value pairs of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::iter`].
    pub fn iter(&self) -> Iter<'_, V> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Return a mutable iterator over the key-value pairs of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::iter_mut`].
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        IterMut(self.0.iter_mut(), PhantomData)
    }

    /// Clears the `IndexMap` in the given index range, returning those
    /// key-value pairs as a drain iterator.
    ///
    /// Equivalent to [`IndexMap::drain`].
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, V> {
        Drain(self.0.drain(range), PhantomData)
    }

    /// Return an iterator over the keys of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::keys`].
    pub fn keys(&self) -> Keys<'_, V> {
        Keys(self.0.keys(), PhantomData)
    }

    /// Return an owning iterator over the keys of the map, in their order.
    ///
    /// Equivalent to [`IndexMap::into_keys`].
    pub fn into_keys(self) -> IntoKeys<V> {
        IntoKeys(self.0.into_keys(), PhantomData)
    }
}

impl<V> Default for EntityIndexMap<V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<V> Deref for EntityIndexMap<V> {
    type Target = IndexMap<Entity, V, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for EntityIndexMap<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, V: Copy> Extend<(&'a Entity, &'a V)> for EntityIndexMap<V> {
    fn extend<T: IntoIterator<Item = (&'a Entity, &'a V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V> Extend<(Entity, V)> for EntityIndexMap<V> {
    fn extend<T: IntoIterator<Item = (Entity, V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V, const N: usize> From<[(Entity, V); N]> for EntityIndexMap<V> {
    fn from(value: [(Entity, V); N]) -> Self {
        Self(IndexMap::from_iter(value))
    }
}

impl<V> FromIterator<(Entity, V)> for EntityIndexMap<V> {
    fn from_iter<I: IntoIterator<Item = (Entity, V)>>(iterable: I) -> Self {
        Self(IndexMap::from_iter(iterable))
    }
}

impl<V, Q: TrustedEntityBorrow + ?Sized> Index<&Q> for EntityIndexMap<V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<V> Index<usize> for EntityIndexMap<V> {
    type Output = V;
    fn index(&self, key: usize) -> &V {
        self.0.index(key)
    }
}

impl<V, Q: TrustedEntityBorrow + ?Sized> IndexMut<&Q> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.0.index_mut(&key.entity())
    }
}

impl<V> IndexMut<usize> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.0.index_mut(key)
    }
}

impl<'a, V> IntoIterator for &'a EntityIndexMap<V> {
    type Item = (&'a Entity, &'a V);
    type IntoIter = Iter<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter(), PhantomData)
    }
}

impl<'a, V> IntoIterator for &'a mut EntityIndexMap<V> {
    type Item = (&'a Entity, &'a mut V);
    type IntoIter = IterMut<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.0.iter_mut(), PhantomData)
    }
}

impl<V> IntoIterator for EntityIndexMap<V> {
    type Item = (Entity, V);
    type IntoIter = IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<V1, V2, S2> PartialEq<IndexMap<Entity, V2, S2>> for EntityIndexMap<V1>
where
    V1: PartialEq<V2>,
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexMap<Entity, V2, S2>) -> bool {
        self.0.eq(other)
    }
}

impl<V1, V2> PartialEq<EntityIndexMap<V2>> for EntityIndexMap<V1>
where
    V1: PartialEq<V2>,
{
    fn eq(&self, other: &EntityIndexMap<V2>) -> bool {
        self.0.eq(other)
    }
}

impl<V: Eq> Eq for EntityIndexMap<V> {}

/// An iterator over the entries of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::iter`] method.
/// See its documentation for more.
pub struct Iter<'a, V, S = EntityHash>(map::Iter<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Iter<'a, V> {
    /// Returns the inner [`Iter`](map::Iter).
    pub fn into_inner(self) -> map::Iter<'a, Entity, V> {
        self.0
    }
}

impl<'a, V> Deref for Iter<'a, V> {
    type Target = map::Iter<'a, Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a Entity, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for Iter<'_, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for Iter<'_, V> {}

impl<V> FusedIterator for Iter<'_, V> {}

impl<V> Clone for Iter<'_, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<V: Debug> Debug for Iter<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<V> Default for Iter<'_, V> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// A mutable iterator over the entries of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::iter_mut`] method.
/// See its documentation for more.
pub struct IterMut<'a, V, S = EntityHash>(map::IterMut<'a, Entity, V>, PhantomData<S>);

impl<'a, V> IterMut<'a, V> {
    /// Returns the inner [`IterMut`](map::IterMut).
    pub fn into_inner(self) -> map::IterMut<'a, Entity, V> {
        self.0
    }
}

impl<'a, V> Deref for IterMut<'a, V> {
    type Target = map::IterMut<'a, Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (&'a Entity, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for IterMut<'_, V> {
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
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// An owning iterator over the entries of an [`IndexMap`].
///
/// This `struct` is created by the [`IndexMap::into_iter`] method
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
pub struct IntoIter<V, S = EntityHash>(map::IntoIter<Entity, V>, PhantomData<S>);

impl<V> IntoIter<V> {
    /// Returns the inner [`IntoIter`](map::IntoIter).
    pub fn into_inner(self) -> map::IntoIter<Entity, V> {
        self.0
    }
}

impl<V> Deref for IntoIter<V> {
    type Target = map::IntoIter<Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for IntoIter<V> {
    type Item = (Entity, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for IntoIter<V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for IntoIter<V> {}

impl<V> FusedIterator for IntoIter<V> {}

impl<V: Clone> Clone for IntoIter<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
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
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// A draining iterator over the entries of an [`EntityIndexMap`].
///
/// This `struct` is created by the [`EntityIndexMap::drain`] method.
/// See its documentation for more.
pub struct Drain<'a, V, S = EntityHash>(map::Drain<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Drain<'a, V> {
    /// Returns the inner [`Drain`](map::Drain).
    pub fn into_inner(self) -> map::Drain<'a, Entity, V> {
        self.0
    }
}

impl<'a, V> Deref for Drain<'a, V> {
    type Target = map::Drain<'a, Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for Drain<'_, V> {
    type Item = (Entity, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for Drain<'_, V> {
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
    /// Returns the inner [`Keys`](map::Keys).
    pub fn into_inner(self) -> map::Keys<'a, Entity, V> {
        self.0
    }
}

impl<'a, V, S> Deref for Keys<'a, V, S> {
    type Target = map::Keys<'a, Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for Keys<'a, V> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for Keys<'_, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for Keys<'_, V> {}

impl<V> FusedIterator for Keys<'_, V> {}

impl<V> Index<usize> for Keys<'_, V> {
    type Output = Entity;

    fn index(&self, index: usize) -> &Entity {
        self.0.index(index)
    }
}

impl<V> Clone for Keys<'_, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<V: Debug> Debug for Keys<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Keys").field(&self.0).field(&self.1).finish()
    }
}

impl<V> Default for Keys<'_, V> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
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
    /// Returns the inner [`IntoKeys`](map::IntoKeys).
    pub fn into_inner(self) -> map::IntoKeys<Entity, V> {
        self.0
    }
}

impl<V> Deref for IntoKeys<V> {
    type Target = map::IntoKeys<Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for IntoKeys<V> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V> DoubleEndedIterator for IntoKeys<V> {
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
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: IntoKeys stems from a correctly behaving `IndexMap<Entity, V, EntityHash>`.
unsafe impl<V> EntitySetIterator for IntoKeys<V> {}
