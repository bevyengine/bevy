use core::{
    fmt::{self, Debug, Formatter},
    hash::BuildHasher,
    iter::FusedIterator,
    marker::PhantomData,
    ops::{BitAnd, BitOr, BitXor, Deref, DerefMut, Index, RangeBounds, Sub},
};

use indexmap::set::{self, IndexSet};

use super::{Entity, EntityHash, EntitySetIterator};

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

    /// Clears the `IndexSet` in the given index range, returning those values
    /// as a drain iterator.
    ///
    /// Equivalent to [`IndexSet::drain`].
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_> {
        Drain(self.0.drain(range), PhantomData)
    }

    /// Return an iterator over the values of the set, in their order.
    ///
    /// Equivalent to [`IndexSet::iter`].
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter(), PhantomData)
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

impl Index<usize> for EntityIndexSet {
    type Output = Entity;
    fn index(&self, key: usize) -> &Entity {
        self.0.index(key)
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
