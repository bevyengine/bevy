//! Contains the [`EntityEquivalentHashSet`] type, a [`HashSet`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around Bevy's [`HashSet`] that is more performant for [`Entity`] keys.

use core::{
    fmt::{self, Debug, Formatter},
    hash::Hash,
    iter::FusedIterator,
    marker::PhantomData,
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, DerefMut, Sub,
        SubAssign,
    },
};

use bevy_platform_support::collections::hash_set::{self, HashSet};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

use super::{
    Entity, EntityHash, EntitySet, EntitySetIterator, FromEntitySetIterator, TrustedBuildHasher,
    TrustedEntityBorrow,
};

/// A [`HashSet`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEquivalentHashSet<K: TrustedEntityBorrow + Hash>(
    pub(crate) HashSet<K, EntityHash>,
)
where
    EntityHash: TrustedBuildHasher<K>;

/// An [`HashSet`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityHashSet = EntityEquivalentHashSet<Entity>;

impl<K: TrustedEntityBorrow + Hash> EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Creates an empty `EntityEquivalentHashSet`.
    ///
    /// Equivalent to [`HashSet::with_hasher(EntityHash)`].
    ///
    /// [`HashSet::with_hasher(EntityHash)`]: HashSet::with_hasher
    pub const fn new() -> Self {
        Self(HashSet::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityEquivalentHashSet` with the specified capacity.
    ///
    /// Equivalent to [`HashSet::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`HashSet::with_capacity_and_hasher(n, EntityHash)`]: HashSet::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(HashSet::with_capacity_and_hasher(n, EntityHash))
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the inner [`HashSet`].
    pub fn into_inner(self) -> HashSet<K, EntityHash> {
        self.0
    }

    /// Clears the set, returning all elements in an iterator.
    ///
    /// Equivalent to [`HashSet::drain`].
    pub fn drain(&mut self) -> Drain<'_, K> {
        Drain(self.0.drain(), PhantomData)
    }

    /// An iterator visiting all elements in arbitrary order.
    /// The iterator element type is `&'a Entity`.
    ///
    /// Equivalent to [`HashSet::iter`].
    pub fn iter(&self) -> Iter<'_, K> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Drains elements which are true under the given predicate,
    /// and returns an iterator over the removed items.
    ///
    /// Equivalent to [`HashSet::extract_if`].
    pub fn extract_if<F: FnMut(&K) -> bool>(&mut self, f: F) -> ExtractIf<'_, K, F> {
        ExtractIf(self.0.extract_if(f), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash> Deref for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = HashSet<K, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: TrustedEntityBorrow + Hash> DerefMut for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> IntoIterator for &'a EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = &'a K;

    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        Iter((&self.0).into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash> IntoIterator for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = K;

    type IntoIter = IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash> Default for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitAnd for &EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentHashSet<K>;

    fn bitand(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.bitand(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitAndAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn bitand_assign(&mut self, rhs: &Self) {
        self.0.bitand_assign(&rhs.0);
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitOr for &EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentHashSet<K>;

    fn bitor(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.bitor(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitOrAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn bitor_assign(&mut self, rhs: &Self) {
        self.0.bitor_assign(&rhs.0);
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitXor for &EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentHashSet<K>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.bitxor(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> BitXorAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn bitxor_assign(&mut self, rhs: &Self) {
        self.0.bitxor_assign(&rhs.0);
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> Sub for &EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Output = EntityEquivalentHashSet<K>;

    fn sub(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.sub(&rhs.0))
    }
}

impl<K: TrustedEntityBorrow + Hash + Clone> SubAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn sub_assign(&mut self, rhs: &Self) {
        self.0.sub_assign(&rhs.0);
    }
}

impl<'a, K: TrustedEntityBorrow + Hash + Copy> Extend<&'a K> for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = &'a K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash> Extend<K> for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: TrustedEntityBorrow + Hash, const N: usize> From<[K; N]> for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from(value: [K; N]) -> Self {
        Self(HashSet::from_iter(value))
    }
}

impl<K: TrustedEntityBorrow + Hash> FromIterator<K> for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from_iter<I: IntoIterator<Item = K>>(iterable: I) -> Self {
        Self(HashSet::from_iter(iterable))
    }
}

impl<K: TrustedEntityBorrow + Hash> FromEntitySetIterator<K> for EntityEquivalentHashSet<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn from_entity_set_iter<I: EntitySet<Item = K>>(set_iter: I) -> Self {
        let iter = set_iter.into_iter();
        let set = EntityEquivalentHashSet::with_capacity(iter.size_hint().0);
        iter.fold(set, |mut set, e| {
            // SAFETY: Every element in self is unique.
            unsafe {
                set.insert_unique_unchecked(e);
            }
            set
        })
    }
}

/// An iterator over the items of an [`EntityEquivalentHashSet`].
///
/// This struct is created by the [`iter`] method on [`EntityEquivalentHashSet`]. See its documentation for more.
///
/// [`iter`]: EntityEquivalentHashSet::iter
pub struct Iter<'a, K: TrustedEntityBorrow + Hash, S = EntityHash>(
    hash_set::Iter<'a, K>,
    PhantomData<S>,
);

impl<'a, K: TrustedEntityBorrow + Hash> Iter<'a, K> {
    /// Returns the inner [`Iter`](hash_set::Iter).
    pub fn into_inner(self) -> hash_set::Iter<'a, K> {
        self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> Deref for Iter<'a, K> {
    type Target = hash_set::Iter<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> Iterator for Iter<'a, K> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash> ExactSizeIterator for Iter<'_, K> {}

impl<K: TrustedEntityBorrow + Hash> FusedIterator for Iter<'_, K> {}

impl<K: TrustedEntityBorrow + Hash> Clone for Iter<'_, K> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<K: TrustedEntityBorrow + Hash + Debug> Debug for Iter<'_, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<K: TrustedEntityBorrow + Hash> Default for Iter<'_, K> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

// SAFETY: Iter stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for Iter<'_, K> {}

/// Owning iterator over the items of an [`EntityEquivalentHashSet`].
///
/// This struct is created by the [`into_iter`] method on [`EntityEquivalentHashSet`] (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: EntityEquivalentHashSet::into_iter
pub struct IntoIter<K: TrustedEntityBorrow + Hash, S: TrustedBuildHasher<K> = EntityHash>(
    hash_set::IntoIter<K>,
    PhantomData<S>,
);

impl<K: TrustedEntityBorrow + Hash> IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`IntoIter`](hash_set::IntoIter).
    pub fn into_inner(self) -> hash_set::IntoIter<K> {
        self.0
    }
}

impl<K: TrustedEntityBorrow + Hash> Deref for IntoIter<K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = hash_set::IntoIter<K>;

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

impl<K: TrustedEntityBorrow + Hash> ExactSizeIterator for IntoIter<K> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash> FusedIterator for IntoIter<K> where
    EntityHash: TrustedBuildHasher<K>
{
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

// SAFETY: IntoIter stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for IntoIter<K> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// A draining iterator over the items of an [`EntityEquivalentHashSet`].
///
/// This struct is created by the [`drain`] method on [`EntityEquivalentHashSet`]. See its documentation for more.
///
/// [`drain`]: EntityEquivalentHashSet::drain
pub struct Drain<'a, K: TrustedEntityBorrow + Hash, S: TrustedBuildHasher<K> = EntityHash>(
    hash_set::Drain<'a, K>,
    PhantomData<S>,
);

impl<'a, K: TrustedEntityBorrow + Hash> Drain<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`Drain`](hash_set::Drain).
    pub fn into_inner(self) -> hash_set::Drain<'a, K> {
        self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash> Deref for Drain<'a, K>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = hash_set::Drain<'a, K>;

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

// SAFETY: Drain stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for Drain<'_, K> where
    EntityHash: TrustedBuildHasher<K>
{
}

/// A draining iterator over entries of a [`EntityEquivalentHashSet`] which don't satisfy the predicate `f`.
///
/// This struct is created by the [`extract_if`] method on [`EntityEquivalentHashSet`]. See its documentation for more.
///
/// [`extract_if`]: EntityEquivalentHashSet::extract_if
pub struct ExtractIf<'a, K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool, S = EntityHash>(
    hash_set::ExtractIf<'a, K, F>,
    PhantomData<S>,
)
where
    EntityHash: TrustedBuildHasher<K>;

impl<'a, K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool> ExtractIf<'a, K, F>
where
    EntityHash: TrustedBuildHasher<K>,
{
    /// Returns the inner [`ExtractIf`](hash_set::ExtractIf).
    pub fn into_inner(self) -> hash_set::ExtractIf<'a, K, F> {
        self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool> Deref for ExtractIf<'a, K, F>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Target = hash_set::ExtractIf<'a, K, F>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool> Iterator for ExtractIf<'a, K, F>
where
    EntityHash: TrustedBuildHasher<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool> FusedIterator for ExtractIf<'_, K, F> where
    EntityHash: TrustedBuildHasher<K>
{
}

impl<K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool> Debug for ExtractIf<'_, K, F>
where
    EntityHash: TrustedBuildHasher<K>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExtractIf").finish()
    }
}

// SAFETY: ExtractIf stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: TrustedEntityBorrow + Hash, F: FnMut(&K) -> bool> EntitySetIterator
    for ExtractIf<'_, K, F>
where
    EntityHash: TrustedBuildHasher<K>,
{
}

// SAFETY: Difference stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator
    for hash_set::Difference<'_, K, EntityHash>
where
    EntityHash: TrustedBuildHasher<K>,
{
}

// SAFETY: Intersection stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator
    for hash_set::Intersection<'_, K, EntityHash>
where
    EntityHash: TrustedBuildHasher<K>,
{
}

// SAFETY: SymmetricDifference stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator
    for hash_set::SymmetricDifference<'_, K, EntityHash>
where
    EntityHash: TrustedBuildHasher<K>,
{
}

// SAFETY: Union stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: TrustedEntityBorrow + Hash> EntitySetIterator for hash_set::Union<'_, K, EntityHash> where
    EntityHash: TrustedBuildHasher<K>
{
}
