//! Contains the [`EntityEquivalentHashMap`] type, a [`HashMap`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around Bevy's [`HashMap`] that is more performant for [`Entity`] keys.

use core::{
    fmt::{self, Debug, Formatter},
    hash::Hash,
    iter::FusedIterator,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index},
};

use bevy_platform::collections::hash_map::{self, HashMap};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

use super::{Entity, EntityEquivalent, EntityHash, EntitySetIterator};

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEquivalentHashMap<K: EntityEquivalent + Hash, V>(HashMap<K, V, EntityHash>);

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityHashMap<V> = EntityEquivalentHashMap<Entity, V>;

impl<K: EntityEquivalent + Hash, V> EntityEquivalentHashMap<K, V> {
    /// Creates an empty `EntityEquivalentHashMap`.
    ///
    /// Equivalent to [`HashMap::with_hasher(EntityHash)`].
    ///
    /// [`HashMap::with_hasher(EntityHash)`]: HashMap::with_hasher
    pub const fn new() -> Self {
        Self(HashMap::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityEquivalentHashMap` with the specified capacity.
    ///
    /// Equivalent to [`HashMap::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`HashMap::with_capacity_and_hasher(n, EntityHash)`]: HashMap::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(HashMap::with_capacity_and_hasher(n, EntityHash))
    }

    /// Constructs an `EntityEquivalentHashMap` from an [`HashMap`].
    pub const fn from_index_map(set: HashMap<K, V, EntityHash>) -> Self {
        Self(set)
    }

    /// Returns the inner [`HashMap`].
    pub fn into_inner(self) -> HashMap<K, V, EntityHash> {
        self.0
    }

    /// An iterator visiting all keys in arbitrary order.
    /// The iterator element type is `&'a K`.
    ///
    /// Equivalent to [`HashMap::keys`].
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.0.keys(), PhantomData)
    }

    /// Creates a consuming iterator visiting all the keys in arbitrary order.
    /// The map cannot be used after calling this.
    /// The iterator element type is [`Entity`].
    ///
    /// Equivalent to [`HashMap::into_keys`].
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys(self.0.into_keys(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash, V> Default for EntityEquivalentHashMap<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: EntityEquivalent + Hash, V> Deref for EntityEquivalentHashMap<K, V> {
    type Target = HashMap<K, V, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: EntityEquivalent + Hash, V> DerefMut for EntityEquivalentHashMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, K: EntityEquivalent + Hash + Copy, V: Copy> Extend<&'a (K, V)>
    for EntityEquivalentHashMap<K, V>
{
    fn extend<I: IntoIterator<Item = &'a (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<'a, K: EntityEquivalent + Hash + Copy, V: Copy> Extend<(&'a K, &'a V)>
    for EntityEquivalentHashMap<K, V>
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: EntityEquivalent + Hash, V> Extend<(K, V)> for EntityEquivalentHashMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: EntityEquivalent + Hash, V, const N: usize> From<[(K, V); N]>
    for EntityEquivalentHashMap<K, V>
{
    fn from(value: [(K, V); N]) -> Self {
        Self(HashMap::from_iter(value))
    }
}

impl<K: EntityEquivalent + Hash, V> FromIterator<(K, V)> for EntityEquivalentHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        Self(HashMap::from_iter(iterable))
    }
}

impl<K: EntityEquivalent + Hash, V> From<HashMap<K, V, EntityHash>>
    for EntityEquivalentHashMap<K, V>
{
    fn from(value: HashMap<K, V, EntityHash>) -> Self {
        Self(value)
    }
}

// `EntityEquivalent` does not guarantee maintained equality on conversions from one implementor to another,
// so we restrict this impl to only keys of type `Entity`.
impl<V, Q: EntityEquivalent + Hash + ?Sized> Index<&Q> for EntityHashMap<V> {
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<'a, K: EntityEquivalent + Hash, V> IntoIterator for &'a EntityEquivalentHashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K: EntityEquivalent + Hash, V> IntoIterator for &'a mut EntityEquivalentHashMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = hash_map::IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<K: EntityEquivalent + Hash, V> IntoIterator for EntityEquivalentHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = hash_map::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// An iterator over the keys of a [`EntityEquivalentHashMap`] in arbitrary order.
/// The iterator element type is `&'a K`.
///
/// This struct is created by the [`keys`] method on [`EntityEquivalentHashMap`]. See its documentation for more.
///
/// [`keys`]: EntityEquivalentHashMap::keys
pub struct Keys<'a, K: EntityEquivalent + Hash, V, S = EntityHash>(
    hash_map::Keys<'a, K, V>,
    PhantomData<S>,
);

impl<'a, K: EntityEquivalent + Hash, V> Keys<'a, K, V> {
    /// Constructs a [`Keys<'a, K, V, S>`] from a [`hash_map::Keys<'a, K, V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `keys` must either be empty, or have been obtained from a
    /// [`hash_map::HashMap`] using the `S` hasher.
    pub const unsafe fn from_keys_unchecked<S>(
        keys: hash_map::Keys<'a, K, V>,
    ) -> Keys<'a, K, V, S> {
        Keys(keys, PhantomData)
    }

    /// Returns the inner [`Keys`](hash_map::Keys).
    pub const fn into_inner(self) -> hash_map::Keys<'a, K, V> {
        self.0
    }
}

impl<'a, K: EntityEquivalent + Hash, V> Deref for Keys<'a, K, V> {
    type Target = hash_map::Keys<'a, K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: EntityEquivalent + Hash, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
    }
}

impl<K: EntityEquivalent + Hash, V> ExactSizeIterator for Keys<'_, K, V> {}

impl<K: EntityEquivalent + Hash, V> FusedIterator for Keys<'_, K, V> {}

impl<K: EntityEquivalent + Hash, V> Clone for Keys<'_, K, V> {
    fn clone(&self) -> Self {
        // SAFETY: We are cloning an already valid `Keys`.
        unsafe { Self::from_keys_unchecked(self.0.clone()) }
    }
}

impl<K: EntityEquivalent + Hash + Debug, V: Debug> Debug for Keys<'_, K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Keys").field(&self.0).field(&self.1).finish()
    }
}

impl<K: EntityEquivalent + Hash, V> Default for Keys<'_, K, V> {
    fn default() -> Self {
        // SAFETY: `Keys` is empty.
        unsafe { Self::from_keys_unchecked(Default::default()) }
    }
}

// SAFETY: Keys stems from a correctly behaving `HashMap<K, V, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash, V> EntitySetIterator for Keys<'_, K, V> {}

/// An owning iterator over the keys of a [`EntityEquivalentHashMap`] in arbitrary order.
/// The iterator element type is [`Entity`].
///
/// This struct is created by the [`into_keys`] method on [`EntityEquivalentHashMap`].
/// See its documentation for more.
/// The map cannot be used after calling that method.
///
/// [`into_keys`]: EntityEquivalentHashMap::into_keys
pub struct IntoKeys<K: EntityEquivalent + Hash, V, S = EntityHash>(
    hash_map::IntoKeys<K, V>,
    PhantomData<S>,
);

impl<K: EntityEquivalent + Hash, V> IntoKeys<K, V> {
    /// Constructs a [`IntoKeys<K, V, S>`] from a [`hash_map::IntoKeys<K, V>`] unsafely.
    ///
    /// # Safety
    ///
    /// `into_keys` must either be empty, or have been obtained from a
    /// [`hash_map::HashMap`] using the `S` hasher.
    pub const unsafe fn from_into_keys_unchecked<S>(
        into_keys: hash_map::IntoKeys<K, V>,
    ) -> IntoKeys<K, V, S> {
        IntoKeys(into_keys, PhantomData)
    }

    /// Returns the inner [`IntoKeys`](hash_map::IntoKeys).
    pub fn into_inner(self) -> hash_map::IntoKeys<K, V> {
        self.0
    }
}

impl<K: EntityEquivalent + Hash, V> Deref for IntoKeys<K, V> {
    type Target = hash_map::IntoKeys<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: EntityEquivalent + Hash, V> Iterator for IntoKeys<K, V> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
    }
}

impl<K: EntityEquivalent + Hash, V> ExactSizeIterator for IntoKeys<K, V> {}

impl<K: EntityEquivalent + Hash, V> FusedIterator for IntoKeys<K, V> {}

impl<K: EntityEquivalent + Hash + Debug, V: Debug> Debug for IntoKeys<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoKeys")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<K: EntityEquivalent + Hash, V> Default for IntoKeys<K, V> {
    fn default() -> Self {
        // SAFETY: `IntoKeys` is empty.
        unsafe { Self::from_into_keys_unchecked(Default::default()) }
    }
}

// SAFETY: IntoKeys stems from a correctly behaving `HashMap<K, V, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash, V> EntitySetIterator for IntoKeys<K, V> {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_reflect::Reflect;
    use static_assertions::assert_impl_all;

    // Check that the HashMaps are Clone if the key/values are Clone
    assert_impl_all!(EntityHashMap::<usize>: Clone);
    // EntityEquivalentHashMap should implement Reflect
    #[cfg(feature = "bevy_reflect")]
    assert_impl_all!(EntityHashMap::<i32>: Reflect);
}
