//! Contains the [`EntityHashSet`] type, a [`HashSet`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around Bevy's [`HashSet`] that is more performant for [`Entity`] keys.

use core::{
    fmt::{self, Debug, Formatter},
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

use super::{Entity, EntityHash, EntitySet, EntitySetIterator, FromEntitySetIterator};

/// A [`HashSet`] pre-configured to use [`EntityHash`] hashing.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityHashSet(pub(crate) HashSet<Entity, EntityHash>);

impl EntityHashSet {
    /// Creates an empty `EntityHashSet`.
    ///
    /// Equivalent to [`HashSet::with_hasher(EntityHash)`].
    ///
    /// [`HashSet::with_hasher(EntityHash)`]: HashSet::with_hasher
    pub const fn new() -> Self {
        Self(HashSet::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityHashSet` with the specified capacity.
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
    pub fn into_inner(self) -> HashSet<Entity, EntityHash> {
        self.0
    }

    /// Clears the set, returning all elements in an iterator.
    ///
    /// Equivalent to [`HashSet::drain`].
    pub fn drain(&mut self) -> Drain<'_> {
        Drain(self.0.drain(), PhantomData)
    }

    /// An iterator visiting all elements in arbitrary order.
    /// The iterator element type is `&'a Entity`.
    ///
    /// Equivalent to [`HashSet::iter`].
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter(), PhantomData)
    }

    /// Drains elements which are true under the given predicate,
    /// and returns an iterator over the removed items.
    ///
    /// Equivalent to [`HashSet::extract_if`].
    pub fn extract_if<F: FnMut(&Entity) -> bool>(&mut self, f: F) -> ExtractIf<'_, F> {
        ExtractIf(self.0.extract_if(f), PhantomData)
    }
}

impl Deref for EntityHashSet {
    type Target = HashSet<Entity, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EntityHashSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a EntityHashSet {
    type Item = &'a Entity;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter((&self.0).into_iter(), PhantomData)
    }
}

impl IntoIterator for EntityHashSet {
    type Item = Entity;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl BitAnd for &EntityHashSet {
    type Output = EntityHashSet;

    fn bitand(self, rhs: Self) -> Self::Output {
        EntityHashSet(self.0.bitand(&rhs.0))
    }
}

impl BitAndAssign<&EntityHashSet> for EntityHashSet {
    fn bitand_assign(&mut self, rhs: &Self) {
        self.0.bitand_assign(&rhs.0);
    }
}

impl BitOr for &EntityHashSet {
    type Output = EntityHashSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        EntityHashSet(self.0.bitor(&rhs.0))
    }
}

impl BitOrAssign<&EntityHashSet> for EntityHashSet {
    fn bitor_assign(&mut self, rhs: &Self) {
        self.0.bitor_assign(&rhs.0);
    }
}

impl BitXor for &EntityHashSet {
    type Output = EntityHashSet;

    fn bitxor(self, rhs: Self) -> Self::Output {
        EntityHashSet(self.0.bitxor(&rhs.0))
    }
}

impl BitXorAssign<&EntityHashSet> for EntityHashSet {
    fn bitxor_assign(&mut self, rhs: &Self) {
        self.0.bitxor_assign(&rhs.0);
    }
}

impl Sub for &EntityHashSet {
    type Output = EntityHashSet;

    fn sub(self, rhs: Self) -> Self::Output {
        EntityHashSet(self.0.sub(&rhs.0))
    }
}

impl SubAssign<&EntityHashSet> for EntityHashSet {
    fn sub_assign(&mut self, rhs: &Self) {
        self.0.sub_assign(&rhs.0);
    }
}

impl<'a> Extend<&'a Entity> for EntityHashSet {
    fn extend<T: IntoIterator<Item = &'a Entity>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl Extend<Entity> for EntityHashSet {
    fn extend<T: IntoIterator<Item = Entity>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<const N: usize> From<[Entity; N]> for EntityHashSet {
    fn from(value: [Entity; N]) -> Self {
        Self(HashSet::from_iter(value))
    }
}

impl FromIterator<Entity> for EntityHashSet {
    fn from_iter<I: IntoIterator<Item = Entity>>(iterable: I) -> Self {
        Self(HashSet::from_iter(iterable))
    }
}

impl FromEntitySetIterator<Entity> for EntityHashSet {
    fn from_entity_set_iter<I: EntitySet<Item = Entity>>(set_iter: I) -> Self {
        let iter = set_iter.into_iter();
        let set = EntityHashSet::with_capacity(iter.size_hint().0);
        iter.fold(set, |mut set, e| {
            // SAFETY: Every element in self is unique.
            unsafe {
                set.insert_unique_unchecked(e);
            }
            set
        })
    }
}

/// An iterator over the items of an [`EntityHashSet`].
///
/// This struct is created by the [`iter`] method on [`EntityHashSet`]. See its documentation for more.
///
/// [`iter`]: EntityHashSet::iter
pub struct Iter<'a, S = EntityHash>(hash_set::Iter<'a, Entity>, PhantomData<S>);

impl<'a> Iter<'a> {
    /// Returns the inner [`Iter`](hash_set::Iter).
    pub fn into_inner(self) -> hash_set::Iter<'a, Entity> {
        self.0
    }
}

impl<'a> Deref for Iter<'a> {
    type Target = hash_set::Iter<'a, Entity>;

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

// SAFETY: Iter stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl EntitySetIterator for Iter<'_> {}

/// Owning iterator over the items of an [`EntityHashSet`].
///
/// This struct is created by the [`into_iter`] method on [`EntityHashSet`] (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: EntityHashSet::into_iter
pub struct IntoIter<S = EntityHash>(hash_set::IntoIter<Entity>, PhantomData<S>);

impl IntoIter {
    /// Returns the inner [`IntoIter`](hash_set::IntoIter).
    pub fn into_inner(self) -> hash_set::IntoIter<Entity> {
        self.0
    }
}

impl Deref for IntoIter {
    type Target = hash_set::IntoIter<Entity>;

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

impl ExactSizeIterator for IntoIter {}

impl FusedIterator for IntoIter {}

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

// SAFETY: IntoIter stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl EntitySetIterator for IntoIter {}

/// A draining iterator over the items of an [`EntityHashSet`].
///
/// This struct is created by the [`drain`] method on [`EntityHashSet`]. See its documentation for more.
///
/// [`drain`]: EntityHashSet::drain
pub struct Drain<'a, S = EntityHash>(hash_set::Drain<'a, Entity>, PhantomData<S>);

impl<'a> Drain<'a> {
    /// Returns the inner [`Drain`](hash_set::Drain).
    pub fn into_inner(self) -> hash_set::Drain<'a, Entity> {
        self.0
    }
}

impl<'a> Deref for Drain<'a> {
    type Target = hash_set::Drain<'a, Entity>;

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

// SAFETY: Drain stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl EntitySetIterator for Drain<'_> {}

/// A draining iterator over entries of a [`EntityHashSet`] which don't satisfy the predicate `f`.
///
/// This struct is created by the [`extract_if`] method on [`EntityHashSet`]. See its documentation for more.
///
/// [`extract_if`]: EntityHashSet::extract_if
pub struct ExtractIf<'a, F: FnMut(&Entity) -> bool, S = EntityHash>(
    hash_set::ExtractIf<'a, Entity, F>,
    PhantomData<S>,
);

impl<'a, F: FnMut(&Entity) -> bool> ExtractIf<'a, F> {
    /// Returns the inner [`ExtractIf`](hash_set::ExtractIf).
    pub fn into_inner(self) -> hash_set::ExtractIf<'a, Entity, F> {
        self.0
    }
}

impl<'a, F: FnMut(&Entity) -> bool> Deref for ExtractIf<'a, F> {
    type Target = hash_set::ExtractIf<'a, Entity, F>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, F: FnMut(&Entity) -> bool> Iterator for ExtractIf<'a, F> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<F: FnMut(&Entity) -> bool> FusedIterator for ExtractIf<'_, F> {}

impl<F: FnMut(&Entity) -> bool> Debug for ExtractIf<'_, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExtractIf").finish()
    }
}

// SAFETY: ExtractIf stems from a correctly behaving `HashSet<Entity, EntityHash>`.
unsafe impl<F: FnMut(&Entity) -> bool> EntitySetIterator for ExtractIf<'_, F> {}

// SAFETY: Difference stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for hash_set::Difference<'_, Entity, EntityHash> {}

// SAFETY: Intersection stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for hash_set::Intersection<'_, Entity, EntityHash> {}

// SAFETY: SymmetricDifference stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for hash_set::SymmetricDifference<'_, Entity, EntityHash> {}

// SAFETY: Union stems from two correctly behaving `HashSet<Entity, EntityHash>`s.
unsafe impl EntitySetIterator for hash_set::Union<'_, Entity, EntityHash> {}
