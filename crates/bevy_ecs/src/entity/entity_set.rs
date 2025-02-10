use alloc::{
    boxed::Box,
    collections::{btree_map, btree_set},
    rc::Rc,
};
use bevy_platform_support::collections::HashSet;

use core::{
    array,
    fmt::{Debug, Formatter},
    hash::{BuildHasher, Hash},
    iter::{self, FusedIterator},
    option, result,
};

use super::{Entity, UniqueEntitySlice};

use bevy_platform_support::sync::Arc;

/// A trait for entity borrows.
///
/// This trait can be thought of as `Borrow<Entity>`, but yielding `Entity` directly.
pub trait EntityBorrow {
    /// Returns the borrowed entity.
    fn entity(&self) -> Entity;
}

/// A trait for [`Entity`] borrows with trustworthy comparison behavior.
///
/// Comparison trait behavior between a [`TrustedEntityBorrow`] type and its underlying entity will match.
/// This property includes [`PartialEq`], [`Eq`], [`PartialOrd`], [`Ord`] and [`Hash`],
/// and remains even after [`Clone`] and/or [`Borrow`] calls.
///
/// # Safety
/// Any [`PartialEq`], [`Eq`], [`PartialOrd`], [`Ord`], and [`Hash`] impls must be
/// equivalent for `Self` and its underlying entity:
/// `x.entity() == y.entity()` should give the same result as `x == y`.
/// The above equivalence must also hold through and between calls to any [`Clone`]
/// and [`Borrow`]/[`BorrowMut`] impls in place of [`entity()`].
///
/// The result of [`entity()`] must be unaffected by any interior mutability.
///
/// [`Hash`]: core::hash::Hash
/// [`Borrow`]: core::borrow::Borrow
/// [`BorrowMut`]: core::borrow::BorrowMut
/// [`entity()`]: EntityBorrow::entity
pub unsafe trait TrustedEntityBorrow: EntityBorrow + Eq {}

impl EntityBorrow for Entity {
    fn entity(&self) -> Entity {
        *self
    }
}

// SAFETY:
// The trait implementations of Entity are correct and deterministic.
unsafe impl TrustedEntityBorrow for Entity {}

impl<T: EntityBorrow> EntityBorrow for &T {
    fn entity(&self) -> Entity {
        (**self).entity()
    }
}

// SAFETY:
// `&T` delegates `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` to T.
// `Clone` and `Borrow` maintain equality.
// `&T` is `Freeze`.
unsafe impl<T: TrustedEntityBorrow> TrustedEntityBorrow for &T {}

impl<T: EntityBorrow> EntityBorrow for &mut T {
    fn entity(&self) -> Entity {
        (**self).entity()
    }
}

// SAFETY:
// `&mut T` delegates `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` to T.
// `Borrow` and `BorrowMut` maintain equality.
//  `&mut T` is `Freeze`.
unsafe impl<T: TrustedEntityBorrow> TrustedEntityBorrow for &mut T {}

impl<T: EntityBorrow> EntityBorrow for Box<T> {
    fn entity(&self) -> Entity {
        (**self).entity()
    }
}

// SAFETY:
// `Box<T>` delegates `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` to T.
// `Clone`, `Borrow` and `BorrowMut` maintain equality.
// `Box<T>` is `Freeze`.
unsafe impl<T: TrustedEntityBorrow> TrustedEntityBorrow for Box<T> {}

impl<T: EntityBorrow> EntityBorrow for Rc<T> {
    fn entity(&self) -> Entity {
        (**self).entity()
    }
}

// SAFETY:
// `Rc<T>` delegates `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` to T.
// `Clone`, `Borrow` and `BorrowMut` maintain equality.
// `Rc<T>` is `Freeze`.
unsafe impl<T: TrustedEntityBorrow> TrustedEntityBorrow for Rc<T> {}

impl<T: EntityBorrow> EntityBorrow for Arc<T> {
    fn entity(&self) -> Entity {
        (**self).entity()
    }
}

// SAFETY:
// `Arc<T>` delegates `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` to T.
// `Clone`, `Borrow` and `BorrowMut` maintain equality.
// `Arc<T>` is `Freeze`.
unsafe impl<T: TrustedEntityBorrow> TrustedEntityBorrow for Arc<T> {}

/// A set of unique entities.
///
/// Any element returned by [`Self::IntoIter`] will compare non-equal to every other element in the iterator.
/// As a consequence, [`into_iter()`] on `EntitySet` will always produce another `EntitySet`.
///
/// Implementing this trait allows for unique query iteration over a list of entities.
/// See [`iter_many_unique`] and [`iter_many_unique_mut`]
///
/// Note that there is no guarantee of the [`IntoIterator`] impl being deterministic,
/// it might return different iterators when called multiple times.
/// Neither is there a guarantee that the comparison trait impls of `EntitySet` match that
/// of the respective [`EntitySetIterator`] (or of a [`Vec`] collected from its elements)
///
/// [`Self::IntoIter`]: IntoIterator::IntoIter
/// [`into_iter()`]: IntoIterator::into_iter
/// [`iter_many_unique`]: crate::system::Query::iter_many_unique
/// [`iter_many_unique_mut`]: crate::system::Query::iter_many_unique_mut
/// [`Vec`]: alloc::vec::Vec
pub trait EntitySet: IntoIterator<IntoIter: EntitySetIterator> {}

impl<T: IntoIterator<IntoIter: EntitySetIterator>> EntitySet for T {}

/// An iterator over a set of unique entities.
///
/// Every `EntitySetIterator` is also [`EntitySet`].
///
/// # Safety
///
/// `x != y` must hold for any 2 elements returned by the iterator.
/// This is always true for iterators that cannot return more than one element.
pub unsafe trait EntitySetIterator: Iterator<Item: TrustedEntityBorrow> {
    /// Transforms an `EntitySetIterator` into a collection.
    ///
    /// This is a specialized form of [`collect`], for collections which benefit from the uniqueness guarantee.
    /// When present, this should always be preferred over [`collect`].
    ///
    /// [`collect`]: Iterator::collect
    //  FIXME: When subtrait item shadowing stabilizes, this should be renamed and shadow `Iterator::collect`
    fn collect_set<B: FromEntitySetIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        FromEntitySetIterator::from_entity_set_iter(self)
    }
}

// SAFETY:
// A correct `BTreeMap` contains only unique keys.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeMap`.
unsafe impl<K: TrustedEntityBorrow, V> EntitySetIterator for btree_map::Keys<'_, K, V> {}

// SAFETY:
// A correct `BTreeMap` contains only unique keys.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeMap`.
unsafe impl<K: TrustedEntityBorrow, V> EntitySetIterator for btree_map::IntoKeys<K, V> {}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
// The sub-range maintains uniqueness.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for btree_set::Range<'_, T> {}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
// The "intersection" operation maintains uniqueness.
unsafe impl<T: TrustedEntityBorrow + Ord> EntitySetIterator for btree_set::Intersection<'_, T> {}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
// The "union" operation maintains uniqueness.
unsafe impl<T: TrustedEntityBorrow + Ord> EntitySetIterator for btree_set::Union<'_, T> {}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
// The "difference" operation maintains uniqueness.
unsafe impl<T: TrustedEntityBorrow + Ord> EntitySetIterator for btree_set::Difference<'_, T> {}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
// The "symmetric difference" operation maintains uniqueness.
unsafe impl<T: TrustedEntityBorrow + Ord> EntitySetIterator
    for btree_set::SymmetricDifference<'_, T>
{
}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for btree_set::Iter<'_, T> {}

// SAFETY:
// A correct `BTreeSet` contains only unique elements.
// TrustedEntityBorrow guarantees a trustworthy Ord impl for T, and thus a correct `BTreeSet`.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for btree_set::IntoIter<T> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for option::Iter<'_, T> {}

// SAFETY: This iterator only returns one element.
// unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for option::IterMut<'_, T> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for option::IntoIter<T> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for result::Iter<'_, T> {}

// SAFETY: This iterator only returns one element.
// unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for result::IterMut<'_, T> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for result::IntoIter<T> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for array::IntoIter<T, 1> {}

// SAFETY: This iterator does not return any elements.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for array::IntoIter<T, 0> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow, F: FnOnce() -> T> EntitySetIterator for iter::OnceWith<F> {}

// SAFETY: This iterator only returns one element.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for iter::Once<T> {}

// SAFETY: This iterator does not return any elements.
unsafe impl<T: TrustedEntityBorrow> EntitySetIterator for iter::Empty<T> {}

// SAFETY: Taking a mutable reference of an iterator has no effect on its elements.
unsafe impl<I: EntitySetIterator + ?Sized> EntitySetIterator for &mut I {}

// SAFETY: Boxing an iterator has no effect on its elements.
unsafe impl<I: EntitySetIterator + ?Sized> EntitySetIterator for Box<I> {}

// SAFETY: TrustedEntityBorrow ensures that Copy does not affect equality, via its restrictions on Clone.
unsafe impl<'a, T: 'a + TrustedEntityBorrow + Copy, I: EntitySetIterator<Item = &'a T>>
    EntitySetIterator for iter::Copied<I>
{
}

// SAFETY: TrustedEntityBorrow ensures that Clone does not affect equality.
unsafe impl<'a, T: 'a + TrustedEntityBorrow + Clone, I: EntitySetIterator<Item = &'a T>>
    EntitySetIterator for iter::Cloned<I>
{
}

// SAFETY: Discarding elements maintains uniqueness.
unsafe impl<I: EntitySetIterator, P: FnMut(&<I as Iterator>::Item) -> bool> EntitySetIterator
    for iter::Filter<I, P>
{
}

// SAFETY: Yielding only `None` after yielding it once can only remove elements, which maintains uniqueness.
unsafe impl<I: EntitySetIterator> EntitySetIterator for iter::Fuse<I> {}

// SAFETY:
// Obtaining immutable references the elements of an iterator does not affect uniqueness.
// TrustedEntityBorrow ensures the lack of interior mutability.
unsafe impl<I: EntitySetIterator, F: FnMut(&<I as Iterator>::Item)> EntitySetIterator
    for iter::Inspect<I, F>
{
}

// SAFETY: Reversing an iterator does not affect uniqueness.
unsafe impl<I: DoubleEndedIterator + EntitySetIterator> EntitySetIterator for iter::Rev<I> {}

// SAFETY: Discarding elements maintains uniqueness.
unsafe impl<I: EntitySetIterator> EntitySetIterator for iter::Skip<I> {}

// SAFETY: Discarding elements maintains uniqueness.
unsafe impl<I: EntitySetIterator, P: FnMut(&<I as Iterator>::Item) -> bool> EntitySetIterator
    for iter::SkipWhile<I, P>
{
}

// SAFETY: Discarding elements maintains uniqueness.
unsafe impl<I: EntitySetIterator> EntitySetIterator for iter::Take<I> {}

// SAFETY: Discarding elements maintains uniqueness.
unsafe impl<I: EntitySetIterator, P: FnMut(&<I as Iterator>::Item) -> bool> EntitySetIterator
    for iter::TakeWhile<I, P>
{
}

// SAFETY: Discarding elements maintains uniqueness.
unsafe impl<I: EntitySetIterator> EntitySetIterator for iter::StepBy<I> {}

/// Conversion from an `EntitySetIterator`.
///
/// Some collections, while they can be constructed from plain iterators,
/// benefit strongly from the additional uniqeness guarantee [`EntitySetIterator`] offers.
/// Mirroring [`Iterator::collect`]/[`FromIterator::from_iter`], [`EntitySetIterator::collect_set`] and
/// `FromEntitySetIterator::from_entity_set_iter` can be used for construction.
///
/// See also: [`EntitySet`].
// FIXME: When subtrait item shadowing stabilizes, this should be renamed and shadow `FromIterator::from_iter`
pub trait FromEntitySetIterator<A: TrustedEntityBorrow>: FromIterator<A> {
    /// Creates a value from an [`EntitySetIterator`].
    fn from_entity_set_iter<T: EntitySet<Item = A>>(set_iter: T) -> Self;
}

impl<T: TrustedEntityBorrow + Hash, S: BuildHasher + Default> FromEntitySetIterator<T>
    for HashSet<T, S>
{
    fn from_entity_set_iter<I: EntitySet<Item = T>>(set_iter: I) -> Self {
        let iter = set_iter.into_iter();
        let set = HashSet::<T, S>::with_capacity_and_hasher(iter.size_hint().0, S::default());
        iter.fold(set, |mut set, e| {
            // SAFETY: Every element in self is unique.
            unsafe {
                set.insert_unique_unchecked(e);
            }
            set
        })
    }
}

/// An iterator that yields unique entities.
///
/// This wrapper can provide an [`EntitySetIterator`] implementation when an instance of `I` is known to uphold uniqueness.
pub struct UniqueEntityIter<I: Iterator<Item: TrustedEntityBorrow>> {
    iter: I,
}

impl<I: EntitySetIterator> UniqueEntityIter<I> {
    /// Constructs a `UniqueEntityIter` from an [`EntitySetIterator`].
    pub fn from_entity_set_iterator<S>(iter: I) -> Self {
        Self { iter }
    }
}

impl<I: Iterator<Item: TrustedEntityBorrow>> UniqueEntityIter<I> {
    /// Constructs a [`UniqueEntityIter`] from an iterator unsafely.
    ///
    /// # Safety
    /// `iter` must only yield unique elements.
    /// As in, the resulting iterator must adhere to the safety contract of [`EntitySetIterator`].
    pub unsafe fn from_iterator_unchecked(iter: I) -> Self {
        Self { iter }
    }

    /// Returns the inner `I`.
    pub fn into_inner(self) -> I {
        self.iter
    }

    /// Returns a reference to the inner `I`.
    pub fn as_inner(&self) -> &I {
        &self.iter
    }

    /// Returns a mutable reference to the inner `I`.
    ///
    /// # Safety
    ///
    /// `self` must always contain an iterator that yields unique elements,
    /// even while this reference is live.
    pub unsafe fn as_mut_inner(&mut self) -> &mut I {
        &mut self.iter
    }
}

impl<I: Iterator<Item: TrustedEntityBorrow>> Iterator for UniqueEntityIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I: ExactSizeIterator<Item: TrustedEntityBorrow>> ExactSizeIterator for UniqueEntityIter<I> {}

impl<I: DoubleEndedIterator<Item: TrustedEntityBorrow>> DoubleEndedIterator
    for UniqueEntityIter<I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<I: FusedIterator<Item: TrustedEntityBorrow>> FusedIterator for UniqueEntityIter<I> {}

// SAFETY: The underlying iterator is ensured to only return unique elements by its construction.
unsafe impl<I: Iterator<Item: TrustedEntityBorrow>> EntitySetIterator for UniqueEntityIter<I> {}

impl<T, I: Iterator<Item: TrustedEntityBorrow> + AsRef<[T]>> AsRef<[T]> for UniqueEntityIter<I> {
    fn as_ref(&self) -> &[T] {
        self.iter.as_ref()
    }
}

impl<T: TrustedEntityBorrow, I: Iterator<Item: TrustedEntityBorrow> + AsRef<[T]>>
    AsRef<UniqueEntitySlice<T>> for UniqueEntityIter<I>
{
    fn as_ref(&self) -> &UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked(self.iter.as_ref()) }
    }
}

impl<T: TrustedEntityBorrow, I: Iterator<Item: TrustedEntityBorrow> + AsMut<[T]>>
    AsMut<UniqueEntitySlice<T>> for UniqueEntityIter<I>
{
    fn as_mut(&mut self) -> &mut UniqueEntitySlice<T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntitySlice::from_slice_unchecked_mut(self.iter.as_mut()) }
    }
}

// Default does not guarantee uniqueness, meaning `I` needs to be EntitySetIterator.
impl<I: EntitySetIterator + Default> Default for UniqueEntityIter<I> {
    fn default() -> Self {
        Self {
            iter: Default::default(),
        }
    }
}

// Clone does not guarantee to maintain uniqueness, meaning `I` needs to be EntitySetIterator.
impl<I: EntitySetIterator + Clone> Clone for UniqueEntityIter<I> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
        }
    }
}

impl<I: Iterator<Item: TrustedEntityBorrow> + Debug> Debug for UniqueEntityIter<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UniqueEntityIter")
            .field("iter", &self.iter)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::prelude::{Schedule, World};

    use crate::component::Component;
    use crate::entity::Entity;
    use crate::query::{QueryState, With};
    use crate::system::Query;
    use crate::world::Mut;

    use super::UniqueEntityIter;

    #[derive(Component, Clone)]
    pub struct Thing;

    #[expect(
        clippy::iter_skip_zero,
        reason = "The `skip(0)` is used to ensure that the `Skip` iterator implements `EntitySet`, which is needed to pass the iterator as the `entities` parameter."
    )]
    #[test]
    fn preserving_uniqueness() {
        let mut world = World::new();

        let mut query = QueryState::<&mut Thing>::new(&mut world);

        let spawn_batch: Vec<Entity> = world.spawn_batch(vec![Thing; 1000]).collect();

        // SAFETY: SpawnBatchIter is `EntitySetIterator`,
        let mut unique_entity_iter =
            unsafe { UniqueEntityIter::from_iterator_unchecked(spawn_batch.iter()) };

        let entity_set = unique_entity_iter
            .by_ref()
            .filter(|_| true)
            .fuse()
            .inspect(|_| ())
            .rev()
            .skip(0)
            .skip_while(|_| false)
            .take(1000)
            .take_while(|_| true)
            .step_by(2)
            .cloned();

        // With `iter_many_mut` collecting is not possible, because you need to drop each `Mut`/`&mut` before the next is retrieved.
        let _results: Vec<Mut<Thing>> =
            query.iter_many_unique_mut(&mut world, entity_set).collect();
    }

    #[test]
    fn nesting_queries() {
        let mut world = World::new();

        world.spawn_batch(vec![Thing; 1000]);

        pub fn system(
            mut thing_entities: Query<Entity, With<Thing>>,
            mut things: Query<&mut Thing>,
        ) {
            things.iter_many_unique(thing_entities.iter());
            things.iter_many_unique_mut(thing_entities.iter_mut());
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(system);
        schedule.run(&mut world);
    }
}
