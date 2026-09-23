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

use bevy_platform::collections::hash_set::{self, HashSet};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

use super::{
    Entity, EntityEquivalent, EntityHash, EntitySet, EntitySetIterator, FromEntitySetIterator,
};

/// A [`HashSet`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEquivalentHashSet<K: EntityEquivalent + Hash>(HashSet<K, EntityHash>);

/// An [`HashSet`] pre-configured to use [`EntityHash`] hashing with an [`Entity`].
pub type EntityHashSet = EntityEquivalentHashSet<Entity>;

impl<K: EntityEquivalent + Hash> EntityEquivalentHashSet<K> {
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

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Constructs an `EntityHashSet` from an [`HashSet`].
    pub const fn from_hash_set(set: HashSet<K, EntityHash>) -> Self {
        Self(set)
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

impl<K: EntityEquivalent + Hash> Deref for EntityEquivalentHashSet<K> {
    type Target = HashSet<K, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: EntityEquivalent + Hash> DerefMut for EntityEquivalentHashSet<K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, K: EntityEquivalent + Hash> IntoIterator for &'a EntityEquivalentHashSet<K> {
    type Item = &'a K;

    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        Iter((&self.0).into_iter(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash> IntoIterator for EntityEquivalentHashSet<K> {
    type Item = K;

    type IntoIter = IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<K: EntityEquivalent + Hash> Default for EntityEquivalentHashSet<K> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitAnd for &EntityEquivalentHashSet<K> {
    type Output = EntityEquivalentHashSet<K>;

    fn bitand(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.bitand(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitAndAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
{
    fn bitand_assign(&mut self, rhs: &Self) {
        self.0.bitand_assign(&rhs.0);
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitOr for &EntityEquivalentHashSet<K> {
    type Output = EntityEquivalentHashSet<K>;

    fn bitor(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.bitor(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitOrAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
{
    fn bitor_assign(&mut self, rhs: &Self) {
        self.0.bitor_assign(&rhs.0);
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitXor for &EntityEquivalentHashSet<K> {
    type Output = EntityEquivalentHashSet<K>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.bitxor(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> BitXorAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
{
    fn bitxor_assign(&mut self, rhs: &Self) {
        self.0.bitxor_assign(&rhs.0);
    }
}

impl<K: EntityEquivalent + Hash + Clone> Sub for &EntityEquivalentHashSet<K> {
    type Output = EntityEquivalentHashSet<K>;

    fn sub(self, rhs: Self) -> Self::Output {
        EntityEquivalentHashSet(self.0.sub(&rhs.0))
    }
}

impl<K: EntityEquivalent + Hash + Clone> SubAssign<&EntityEquivalentHashSet<K>>
    for EntityEquivalentHashSet<K>
{
    fn sub_assign(&mut self, rhs: &Self) {
        self.0.sub_assign(&rhs.0);
    }
}

impl<'a, K: EntityEquivalent + Hash + Copy> Extend<&'a K> for EntityEquivalentHashSet<K> {
    fn extend<I: IntoIterator<Item = &'a K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: EntityEquivalent + Hash> Extend<K> for EntityEquivalentHashSet<K> {
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<K: EntityEquivalent + Hash, const N: usize> From<[K; N]> for EntityEquivalentHashSet<K> {
    fn from(value: [K; N]) -> Self {
        Self(HashSet::from_iter(value))
    }
}

impl<K: EntityEquivalent + Hash> FromIterator<K> for EntityEquivalentHashSet<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iterable: I) -> Self {
        Self(HashSet::from_iter(iterable))
    }
}

impl<K: EntityEquivalent + Hash> FromEntitySetIterator<K> for EntityEquivalentHashSet<K> {
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

impl<K: EntityEquivalent + Hash> From<HashSet<K, EntityHash>> for EntityEquivalentHashSet<K> {
    fn from(value: HashSet<K, EntityHash>) -> Self {
        Self(value)
    }
}

/// An iterator over the items of an [`EntityEquivalentHashSet`].
///
/// This struct is created by the [`iter`] method on [`EntityEquivalentHashSet`]. See its documentation for more.
///
/// [`iter`]: EntityEquivalentHashSet::iter
pub struct Iter<'a, K: EntityEquivalent + Hash, S = EntityHash>(
    hash_set::Iter<'a, K>,
    PhantomData<S>,
);

impl<'a, K: EntityEquivalent + Hash> Iter<'a, K> {
    /// Constructs a [`Iter<'a, K, S>`] from a [`hash_set::Iter<'a, K>`] unsafely.
    ///
    /// # Safety
    ///
    /// `iter` must either be empty, or have been obtained from a
    /// [`hash_set::HashSet`] using the `S` hasher.
    pub const unsafe fn from_iter_unchecked<S>(iter: hash_set::Iter<'a, K>) -> Iter<'a, K, S> {
        Iter(iter, PhantomData)
    }

    /// Returns the inner [`Iter`](hash_set::Iter).
    pub const fn into_inner(self) -> hash_set::Iter<'a, K> {
        self.0
    }
}

impl<'a, K: EntityEquivalent + Hash> Deref for Iter<'a, K> {
    type Target = hash_set::Iter<'a, K>;

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

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
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

// SAFETY: Iter stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for Iter<'_, K> {}

/// Owning iterator over the items of an [`EntityEquivalentHashSet`].
///
/// This struct is created by the [`into_iter`] method on [`EntityEquivalentHashSet`] (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: EntityEquivalentHashSet::into_iter
pub struct IntoIter<K: EntityEquivalent + Hash, S = EntityHash>(
    hash_set::IntoIter<K>,
    PhantomData<S>,
);

impl<K: EntityEquivalent + Hash> IntoIter<K> {
    /// Constructs a [`IntoIter<K, S>`] from a [`hash_set::IntoIter<K>`] unsafely.
    ///
    /// # Safety
    ///
    /// `into_iter` must either be empty, or have been obtained from a
    /// [`hash_set::HashSet`] using the `S` hasher.
    pub const unsafe fn from_into_iter_unchecked<S>(
        into_iter: hash_set::IntoIter<K>,
    ) -> IntoIter<K, S> {
        IntoIter(into_iter, PhantomData)
    }

    /// Returns the inner [`IntoIter`](hash_set::IntoIter).
    pub fn into_inner(self) -> hash_set::IntoIter<K> {
        self.0
    }
}

impl<K: EntityEquivalent + Hash> Deref for IntoIter<K> {
    type Target = hash_set::IntoIter<K>;

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

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
    }
}

impl<K: EntityEquivalent + Hash> ExactSizeIterator for IntoIter<K> {}

impl<K: EntityEquivalent + Hash> FusedIterator for IntoIter<K> {}

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

// SAFETY: IntoIter stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for IntoIter<K> {}

/// A draining iterator over the items of an [`EntityEquivalentHashSet`].
///
/// This struct is created by the [`drain`] method on [`EntityEquivalentHashSet`]. See its documentation for more.
///
/// [`drain`]: EntityEquivalentHashSet::drain
pub struct Drain<'a, K: EntityEquivalent + Hash, S = EntityHash>(
    hash_set::Drain<'a, K>,
    PhantomData<S>,
);

impl<'a, K: EntityEquivalent + Hash> Drain<'a, K> {
    /// Constructs a [`Drain<'a, K, S>`] from a [`hash_set::Drain<'a, K>`] unsafely.
    ///
    /// # Safety
    ///
    /// `drain` must either be empty, or have been obtained from a
    /// [`hash_set::HashSet`] using the `S` hasher.
    pub const unsafe fn from_drain_unchecked<S>(drain: hash_set::Drain<'a, K>) -> Drain<'a, K, S> {
        Drain(drain, PhantomData)
    }

    /// Returns the inner [`Drain`](hash_set::Drain).
    pub fn into_inner(self) -> hash_set::Drain<'a, K> {
        self.0
    }
}

impl<'a, K: EntityEquivalent + Hash> Deref for Drain<'a, K> {
    type Target = hash_set::Drain<'a, K>;

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

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
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

// SAFETY: Drain stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for Drain<'_, K> {}

/// A draining iterator over entries of a [`EntityEquivalentHashSet`] which don't satisfy the predicate `f`.
///
/// This struct is created by the [`extract_if`] method on [`EntityEquivalentHashSet`]. See its documentation for more.
///
/// [`extract_if`]: EntityEquivalentHashSet::extract_if
pub struct ExtractIf<'a, K: EntityEquivalent + Hash, F: FnMut(&K) -> bool, S = EntityHash>(
    hash_set::ExtractIf<'a, K, F>,
    PhantomData<S>,
);

impl<'a, K: EntityEquivalent + Hash, F: FnMut(&K) -> bool> ExtractIf<'a, K, F> {
    /// Constructs a [`ExtractIf<'a, K, F, S>`] from a [`hash_set::ExtractIf<'a, K, F>`] unsafely.
    ///
    /// # Safety
    ///
    /// `extract_if` must either be empty, or have been obtained from a
    /// [`hash_set::HashSet`] using the `S` hasher.
    pub const unsafe fn from_extract_if_unchecked<S>(
        extract_if: hash_set::ExtractIf<'a, K, F>,
    ) -> ExtractIf<'a, K, F, S> {
        ExtractIf(extract_if, PhantomData)
    }

    /// Returns the inner [`ExtractIf`](hash_set::ExtractIf).
    pub fn into_inner(self) -> hash_set::ExtractIf<'a, K, F> {
        self.0
    }
}

impl<'a, K: EntityEquivalent + Hash, F: FnMut(&K) -> bool> Deref for ExtractIf<'a, K, F> {
    type Target = hash_set::ExtractIf<'a, K, F>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, K: EntityEquivalent + Hash, F: FnMut(&K) -> bool> Iterator for ExtractIf<'a, K, F> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<K: EntityEquivalent + Hash, F: FnMut(&K) -> bool> FusedIterator for ExtractIf<'_, K, F> {}

impl<K: EntityEquivalent + Hash, F: FnMut(&K) -> bool> Debug for ExtractIf<'_, K, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExtractIf").finish()
    }
}

// SAFETY: ExtractIf stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<K: EntityEquivalent + Hash, F: FnMut(&K) -> bool> EntitySetIterator
    for ExtractIf<'_, K, F>
{
}

// SAFETY: Difference stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator
    for hash_set::Difference<'_, K, EntityHash>
{
}

// SAFETY: Intersection stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator
    for hash_set::Intersection<'_, K, EntityHash>
{
}

// SAFETY: SymmetricDifference stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator
    for hash_set::SymmetricDifference<'_, K, EntityHash>
{
}

// SAFETY: Union stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl<K: EntityEquivalent + Hash> EntitySetIterator for hash_set::Union<'_, K, EntityHash> {}
