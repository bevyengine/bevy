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

use bevy_platform_support::collections::hash_map::{self, HashMap};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

use super::{Entity, EntityHash, EntitySetIterator, TrustedBuildHasher, TrustedEntityBorrow};

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEquivalentHashMap<K: TrustedEntityBorrow + Hash, V>(
    pub(crate) HashMap<K, V, EntityHash>,
)
where
    EntityHash: TrustedBuildHasher<K>;

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityHashMap<V> = EntityEquivalentHashMap<Entity, V>;

impl<K: TrustedEntityBorrow + Hash, V> EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
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
    /// [`HashMap:with_capacity_and_hasher(n, EntityHash)`]: HashMap::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(HashMap::with_capacity_and_hasher(n, EntityHash))
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

impl<K: TrustedEntityBorrow + Hash, V> Default for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Deref for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = HashMap<K, V, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> DerefMut for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash + Copy, V: Copy> Extend<&'a (K, V)>
    for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = &'a (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<'a, K: TrustedEntityBorrow + Hash + Copy, V: Copy> Extend<(&'a K, &'a V)>
    for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Extend<(K, V)> for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash, V, const N: usize> From<[(K, V); N]>
    for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from(value: [(K, V); N]) -> Self {
        Self(HashMap::from_iter(value))
    }
}

impl<K: TrustedEntityBorrow + Hash, V> FromIterator<(K, V)> for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        Self(HashMap::from_iter(iterable))
    }
}

// `TrustedEntityBorrow` does not guarantee maintained equality on conversions from one implementer to another,
// so we restrict this impl to only keys of type `Entity`.
impl<V, Q: TrustedEntityBorrow + Hash + ?Sized> Index<&Q> for EntityHashMap<V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> IntoIterator for &'a EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> IntoIterator for &'a mut EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = hash_map::IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<K: TrustedEntityBorrow + Hash, V> IntoIterator for EntityEquivalentHashMap<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
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
pub struct Keys<'a, K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    hash_map::Keys<'a, K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash, V> Keys<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`Keys`](hash_map::Keys).
    pub fn into_inner(self) -> hash_map::Keys<'a, K, V> {
        self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, V> Deref for Keys<'a, K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = hash_map::Keys<'a, K, V>;

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

impl<K: TrustedEntityBorrow + Hash, V> ExactSizeIterator for Keys<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> FusedIterator for Keys<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, V> Clone for Keys<'_, K, V>
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

// SAFETY: Keys stems from a correctly behaving `HashMap<K, V, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash, V> EntitySetIterator for Keys<'_, K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// An owning iterator over the keys of a [`EntityEquivalentHashMap`] in arbitrary order.
/// The iterator element type is [`Entity`].
///
/// This struct is created by the [`into_keys`] method on [`EntityEquivalentHashMap`].
/// See its documentation for more.
/// The map cannot be used after calling that method.
///
/// [`into_keys`]: EntityEquivalentHashMap::into_keys
pub struct IntoKeys<K: TrustedEntityBorrow + Hash, V, S = EntityHash>(
    hash_map::IntoKeys<K, V>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<K: TrustedEntityBorrow + Hash, V> IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`IntoKeys`](hash_map::IntoKeys).
    pub fn into_inner(self) -> hash_map::IntoKeys<K, V> {
        self.0
    }
}

impl<K: TrustedEntityBorrow + Hash, V> Deref for IntoKeys<K, V>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = hash_map::IntoKeys<K, V>;

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

// SAFETY: IntoKeys stems from a correctly behaving `HashMap<K, V, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash, V> EntitySetIterator for IntoKeys<K, V> where
    EntityHash: TrustedBuildHasher<K>
{
}

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
