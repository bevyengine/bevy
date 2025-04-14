//! Provides [`HashSet`] based on [hashbrown]'s implementation.
//! Unlike [`hashbrown::HashSet`], [`HashSet`] defaults to [`FixedHasher`]
//! instead of [`RandomState`](crate::hash::RandomState).
//! This provides determinism by default with an acceptable compromise to denial
//! of service resistance in the context of a game engine.

use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash},
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, DerefMut, Sub,
        SubAssign,
    },
};

use hashbrown::{hash_set as hb, Equivalent};

use crate::hash::FixedHasher;

#[cfg(feature = "rayon")]
use rayon::prelude::{FromParallelIterator, IntoParallelIterator, ParallelExtend};

// Re-exports to match `std::collections::hash_set`
pub use hb::{Difference, Drain, Intersection, IntoIter, Iter, SymmetricDifference, Union};

// Additional items from `hashbrown`
pub use hb::{ExtractIf, OccupiedEntry, VacantEntry};

/// Shortcut for [`Entry`](hb::Entry) with [`FixedHasher`] as the default hashing provider.
pub type Entry<'a, T, S = FixedHasher> = hb::Entry<'a, T, S>;

/// New-type for [`HashSet`](hb::HashSet) with [`FixedHasher`] as the default hashing provider.
/// Can be trivially converted to and from a [hashbrown] [`HashSet`](hb::HashSet) using [`From`].
///
/// A new-type is used instead of a type alias due to critical methods like [`new`](hb::HashSet::new)
/// being incompatible with Bevy's choice of default hasher.
#[repr(transparent)]
pub struct HashSet<T, S = FixedHasher>(hb::HashSet<T, S>);

impl<T, S> Clone for HashSet<T, S>
where
    hb::HashSet<T, S>: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

impl<T, S> Debug for HashSet<T, S>
where
    hb::HashSet<T, S>: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <hb::HashSet<T, S> as Debug>::fmt(&self.0, f)
    }
}

impl<T, S> Default for HashSet<T, S>
where
    hb::HashSet<T, S>: Default,
{
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T, S> PartialEq for HashSet<T, S>
where
    hb::HashSet<T, S>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T, S> Eq for HashSet<T, S> where hb::HashSet<T, S>: Eq {}

impl<T, S, X> FromIterator<X> for HashSet<T, S>
where
    hb::HashSet<T, S>: FromIterator<X>,
{
    #[inline]
    fn from_iter<U: IntoIterator<Item = X>>(iter: U) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}

impl<T, S> IntoIterator for HashSet<T, S>
where
    hb::HashSet<T, S>: IntoIterator,
{
    type Item = <hb::HashSet<T, S> as IntoIterator>::Item;

    type IntoIter = <hb::HashSet<T, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S>
where
    &'a hb::HashSet<T, S>: IntoIterator,
{
    type Item = <&'a hb::HashSet<T, S> as IntoIterator>::Item;

    type IntoIter = <&'a hb::HashSet<T, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, T, S> IntoIterator for &'a mut HashSet<T, S>
where
    &'a mut hb::HashSet<T, S>: IntoIterator,
{
    type Item = <&'a mut hb::HashSet<T, S> as IntoIterator>::Item;

    type IntoIter = <&'a mut hb::HashSet<T, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<T, S, X> Extend<X> for HashSet<T, S>
where
    hb::HashSet<T, S>: Extend<X>,
{
    #[inline]
    fn extend<U: IntoIterator<Item = X>>(&mut self, iter: U) {
        self.0.extend(iter);
    }
}

impl<T, const N: usize> From<[T; N]> for HashSet<T, FixedHasher>
where
    T: Eq + Hash,
{
    fn from(value: [T; N]) -> Self {
        value.into_iter().collect()
    }
}

impl<T, S> From<crate::collections::HashMap<T, (), S>> for HashSet<T, S> {
    #[inline]
    fn from(value: crate::collections::HashMap<T, (), S>) -> Self {
        Self(hb::HashSet::from(hashbrown::HashMap::from(value)))
    }
}

impl<T, S> From<hb::HashSet<T, S>> for HashSet<T, S> {
    #[inline]
    fn from(value: hb::HashSet<T, S>) -> Self {
        Self(value)
    }
}

impl<T, S> From<HashSet<T, S>> for hb::HashSet<T, S> {
    #[inline]
    fn from(value: HashSet<T, S>) -> Self {
        value.0
    }
}

impl<T, S> Deref for HashSet<T, S> {
    type Target = hb::HashSet<T, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> DerefMut for HashSet<T, S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "serialize")]
impl<T, S> serde::Serialize for HashSet<T, S>
where
    hb::HashSet<T, S>: serde::Serialize,
{
    #[inline]
    fn serialize<U>(&self, serializer: U) -> Result<U::Ok, U::Error>
    where
        U: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, T, S> serde::Deserialize<'de> for HashSet<T, S>
where
    hb::HashSet<T, S>: serde::Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(serde::Deserialize::deserialize(deserializer)?))
    }
}

#[cfg(feature = "rayon")]
impl<T, S, U> FromParallelIterator<U> for HashSet<T, S>
where
    hb::HashSet<T, S>: FromParallelIterator<U>,
    U: Send,
{
    fn from_par_iter<P>(par_iter: P) -> Self
    where
        P: IntoParallelIterator<Item = U>,
    {
        Self(<hb::HashSet<T, S> as FromParallelIterator<U>>::from_par_iter(par_iter))
    }
}

#[cfg(feature = "rayon")]
impl<T, S> IntoParallelIterator for HashSet<T, S>
where
    hb::HashSet<T, S>: IntoParallelIterator,
{
    type Item = <hb::HashSet<T, S> as IntoParallelIterator>::Item;
    type Iter = <hb::HashSet<T, S> as IntoParallelIterator>::Iter;

    fn into_par_iter(self) -> Self::Iter {
        self.0.into_par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<'a, T: Sync, S> IntoParallelIterator for &'a HashSet<T, S>
where
    &'a hb::HashSet<T, S>: IntoParallelIterator,
{
    type Item = <&'a hb::HashSet<T, S> as IntoParallelIterator>::Item;
    type Iter = <&'a hb::HashSet<T, S> as IntoParallelIterator>::Iter;

    fn into_par_iter(self) -> Self::Iter {
        (&self.0).into_par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<T, S, U> ParallelExtend<U> for HashSet<T, S>
where
    hb::HashSet<T, S>: ParallelExtend<U>,
    U: Send,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = U>,
    {
        <hb::HashSet<T, S> as ParallelExtend<U>>::par_extend(&mut self.0, par_iter);
    }
}

impl<T> HashSet<T, FixedHasher> {
    /// Creates an empty [`HashSet`].
    ///
    /// Refer to [`new`](hb::HashSet::new) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// // Creates a HashSet with zero capacity.
    /// let map = HashSet::new();
    /// #
    /// # let mut map = map;
    /// # map.insert("foo");
    /// # assert_eq!(map.get("foo"), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self::with_hasher(FixedHasher)
    }

    /// Creates an empty [`HashSet`] with the specified capacity.
    ///
    /// Refer to [`with_capacity`](hb::HashSet::with_capacity) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// // Creates a HashSet with capacity for at least 5 entries.
    /// let map = HashSet::with_capacity(5);
    /// #
    /// # let mut map = map;
    /// # map.insert("foo");
    /// # assert_eq!(map.get("foo"), Some("foo").as_ref());
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, FixedHasher)
    }
}

impl<T, S> HashSet<T, S> {
    /// Returns the number of elements the set can hold without reallocating.
    ///
    /// Refer to [`capacity`](hb::HashSet::capacity) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let map = HashSet::with_capacity(5);
    ///
    /// # let map: HashSet<()> = map;
    /// #
    /// assert!(map.capacity() >= 5);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// An iterator visiting all elements in arbitrary order.
    /// The iterator element type is `&'a T`.
    ///
    /// Refer to [`iter`](hb::HashSet::iter) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// for value in map.iter() {
    ///     // "foo", "bar", "baz"
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.iter().count(), 3);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.0.iter()
    }

    /// Returns the number of elements in the set.
    ///
    /// Refer to [`len`](hb::HashSet::len) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// assert_eq!(map.len(), 0);
    ///
    /// map.insert("foo");
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// Refer to [`is_empty`](hb::HashSet::is_empty) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// assert!(map.is_empty());
    ///
    /// map.insert("foo");
    ///
    /// assert!(!map.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the set, returning all elements in an iterator.
    ///
    /// Refer to [`drain`](hb::HashSet::drain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// for value in map.drain() {
    ///     // "foo", "bar", "baz"
    ///     // Note that the above order is not guaranteed
    /// }
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, T> {
        self.0.drain()
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Refer to [`retain`](hb::HashSet::retain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// map.retain(|value| *value == "baz");
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.0.retain(f);
    }

    /// Drains elements which are true under the given predicate,
    /// and returns an iterator over the removed items.
    ///
    /// Refer to [`extract_if`](hb::HashSet::extract_if) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// let extracted = map
    ///     .extract_if(|value| *value == "baz")
    ///     .collect::<Vec<_>>();
    ///
    /// assert_eq!(map.len(), 2);
    /// assert_eq!(extracted.len(), 1);
    /// ```
    #[inline]
    pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.0.extract_if(f)
    }

    /// Clears the set, removing all values.
    ///
    /// Refer to [`clear`](hb::HashSet::clear) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// map.clear();
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Creates a new empty hash set which will use the given hasher to hash
    /// keys.
    ///
    /// Refer to [`with_hasher`](hb::HashSet::with_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashSet with the provided hasher.
    /// let map = HashSet::with_hasher(SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert("foo");
    /// # assert_eq!(map.get("foo"), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn with_hasher(hasher: S) -> Self {
        Self(hb::HashSet::with_hasher(hasher))
    }

    /// Creates an empty [`HashSet`] with the specified capacity, using
    /// `hasher` to hash the keys.
    ///
    /// Refer to [`with_capacity_and_hasher`](hb::HashSet::with_capacity_and_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashSet with capacity for 5 entries and the provided hasher.
    /// let map = HashSet::with_capacity_and_hasher(5, SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert("foo");
    /// # assert_eq!(map.get("foo"), Some("foo").as_ref());
    /// ```
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self(hb::HashSet::with_capacity_and_hasher(capacity, hasher))
    }

    /// Returns a reference to the set's [`BuildHasher`].
    ///
    /// Refer to [`hasher`](hb::HashSet::hasher) for further details.
    #[inline]
    pub fn hasher(&self) -> &S {
        self.0.hasher()
    }

    /// Takes the inner [`HashSet`](hb::HashSet) out of this wrapper.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let map: HashSet<&'static str> = HashSet::new();
    /// let map: hashbrown::HashSet<&'static str, _> = map.into_inner();
    /// ```
    #[inline]
    pub fn into_inner(self) -> hb::HashSet<T, S> {
        self.0
    }
}

impl<T, S> HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the [`HashSet`]. The collection may reserve more space to avoid
    /// frequent reallocations.
    ///
    /// Refer to [`reserve`](hb::HashSet::reserve) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::with_capacity(5);
    ///
    /// # let mut map: HashSet<()> = map;
    /// #
    /// assert!(map.capacity() >= 5);
    ///
    /// map.reserve(10);
    ///
    /// assert!(map.capacity() - map.len() >= 10);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements to be inserted
    /// in the given `HashSet<K,V>`. The collection may reserve more space to avoid
    /// frequent reallocations.
    ///
    /// Refer to [`try_reserve`](hb::HashSet::try_reserve) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::with_capacity(5);
    ///
    /// # let mut map: HashSet<()> = map;
    /// #
    /// assert!(map.capacity() >= 5);
    ///
    /// map.try_reserve(10).expect("Out of Memory!");
    ///
    /// assert!(map.capacity() - map.len() >= 10);
    /// ```
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), hashbrown::TryReserveError> {
        self.0.try_reserve(additional)
    }

    /// Shrinks the capacity of the set as much as possible. It will drop
    /// down as much as possible while maintaining the internal rules
    /// and possibly leaving some space in accordance with the resize policy.
    ///
    /// Refer to [`shrink_to_fit`](hb::HashSet::shrink_to_fit) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::with_capacity(5);
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// assert!(map.capacity() >= 5);
    ///
    /// map.shrink_to_fit();
    ///
    /// assert_eq!(map.capacity(), 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Shrinks the capacity of the set with a lower limit. It will drop
    /// down no lower than the supplied limit while maintaining the internal rules
    /// and possibly leaving some space in accordance with the resize policy.
    ///
    /// Refer to [`shrink_to`](hb::HashSet::shrink_to) for further details.
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.0.shrink_to(min_capacity);
    }

    /// Visits the values representing the difference,
    /// i.e., the values that are in `self` but not in `other`.
    ///
    /// Refer to [`difference`](hb::HashSet::difference) for further details.
    #[inline]
    pub fn difference<'a>(&'a self, other: &'a Self) -> Difference<'a, T, S> {
        self.0.difference(other)
    }

    /// Visits the values representing the symmetric difference,
    /// i.e., the values that are in `self` or in `other` but not in both.
    ///
    /// Refer to [`symmetric_difference`](hb::HashSet::symmetric_difference) for further details.
    #[inline]
    pub fn symmetric_difference<'a>(&'a self, other: &'a Self) -> SymmetricDifference<'a, T, S> {
        self.0.symmetric_difference(other)
    }

    /// Visits the values representing the intersection,
    /// i.e., the values that are both in `self` and `other`.
    ///
    /// Refer to [`intersection`](hb::HashSet::intersection) for further details.
    #[inline]
    pub fn intersection<'a>(&'a self, other: &'a Self) -> Intersection<'a, T, S> {
        self.0.intersection(other)
    }

    /// Visits the values representing the union,
    /// i.e., all the values in `self` or `other`, without duplicates.
    ///
    /// Refer to [`union`](hb::HashSet::union) for further details.
    #[inline]
    pub fn union<'a>(&'a self, other: &'a Self) -> Union<'a, T, S> {
        self.0.union(other)
    }

    /// Returns `true` if the set contains a value.
    ///
    /// Refer to [`contains`](hb::HashSet::contains) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.contains("foo"));
    /// ```
    #[inline]
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.contains(value)
    }

    /// Returns a reference to the value in the set, if any, that is equal to the given value.
    ///
    /// Refer to [`get`](hb::HashSet::get) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert_eq!(map.get("foo"), Some(&"foo"));
    /// ```
    #[inline]
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.get(value)
    }

    /// Inserts the given `value` into the set if it is not present, then
    /// returns a reference to the value in the set.
    ///
    /// Refer to [`get_or_insert`](hb::HashSet::get_or_insert) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// assert_eq!(map.get_or_insert("foo"), &"foo");
    /// ```
    #[inline]
    pub fn get_or_insert(&mut self, value: T) -> &T {
        self.0.get_or_insert(value)
    }

    /// Inserts a value computed from `f` into the set if the given `value` is
    /// not present, then returns a reference to the value in the set.
    ///
    /// Refer to [`get_or_insert_with`](hb::HashSet::get_or_insert_with) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// assert_eq!(map.get_or_insert_with(&"foo", |_| "foo"), &"foo");
    /// ```
    #[inline]
    pub fn get_or_insert_with<Q, F>(&mut self, value: &Q, f: F) -> &T
    where
        Q: Hash + Equivalent<T> + ?Sized,
        F: FnOnce(&Q) -> T,
    {
        self.0.get_or_insert_with(value, f)
    }

    /// Gets the given value's corresponding entry in the set for in-place manipulation.
    ///
    /// Refer to [`entry`](hb::HashSet::entry) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// let value = map.entry("foo").or_insert();
    /// #
    /// # assert_eq!(value, ());
    /// ```
    #[inline]
    pub fn entry(&mut self, value: T) -> Entry<'_, T, S> {
        self.0.entry(value)
    }

    /// Returns `true` if `self` has no elements in common with `other`.
    /// This is equivalent to checking for an empty intersection.
    ///
    /// Refer to [`is_disjoint`](hb::HashSet::is_disjoint) for further details.
    #[inline]
    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.0.is_disjoint(other)
    }

    /// Returns `true` if the set is a subset of another,
    /// i.e., `other` contains at least all the values in `self`.
    ///
    /// Refer to [`is_subset`](hb::HashSet::is_subset) for further details.
    #[inline]
    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.is_subset(other)
    }

    /// Returns `true` if the set is a superset of another,
    /// i.e., `self` contains at least all the values in `other`.
    ///
    /// Refer to [`is_superset`](hb::HashSet::is_superset) for further details.
    #[inline]
    pub fn is_superset(&self, other: &Self) -> bool {
        self.0.is_superset(other)
    }

    /// Adds a value to the set.
    ///
    /// Refer to [`insert`](hb::HashSet::insert) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.contains("foo"));
    /// ```
    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        self.0.insert(value)
    }

    /// Adds a value to the set, replacing the existing value, if any, that is equal to the given
    /// one. Returns the replaced value.
    ///
    /// Refer to [`replace`](hb::HashSet::replace) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert_eq!(map.replace("foo"), Some("foo"));
    /// ```
    #[inline]
    pub fn replace(&mut self, value: T) -> Option<T> {
        self.0.replace(value)
    }

    /// Removes a value from the set. Returns whether the value was
    /// present in the set.
    ///
    /// Refer to [`remove`](hb::HashSet::remove) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.remove("foo"));
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.remove(value)
    }

    /// Removes and returns the value in the set, if any, that is equal to the given one.
    ///
    /// Refer to [`take`](hb::HashSet::take) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert_eq!(map.take("foo"), Some("foo"));
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn take<Q>(&mut self, value: &Q) -> Option<T>
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.take(value)
    }

    /// Returns the total amount of memory allocated internally by the hash
    /// set, in bytes.
    ///
    /// Refer to [`allocation_size`](hb::HashSet::allocation_size) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// assert_eq!(map.allocation_size(), 0);
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.allocation_size() >= size_of::<&'static str>());
    /// ```
    #[inline]
    pub fn allocation_size(&self) -> usize {
        self.0.allocation_size()
    }

    /// Insert a value the set without checking if the value already exists in the set.
    ///
    /// Refer to [`insert_unique_unchecked`](hb::HashSet::insert_unique_unchecked) for further details.
    ///
    /// # Safety
    ///
    /// This operation is safe if a value does not exist in the set.
    ///
    /// However, if a value exists in the set already, the behavior is unspecified:
    /// this operation may panic, loop forever, or any following operation with the set
    /// may panic, loop forever or return arbitrary result.
    ///
    /// That said, this operation (and following operations) are guaranteed to
    /// not violate memory safety.
    ///
    /// However this operation is still unsafe because the resulting `HashSet`
    /// may be passed to unsafe code which does expect the set to behave
    /// correctly, and would cause unsoundness as a result.
    #[expect(
        unsafe_code,
        reason = "re-exporting unsafe method from Hashbrown requires unsafe code"
    )]
    #[inline]
    pub unsafe fn insert_unique_unchecked(&mut self, value: T) -> &T {
        // SAFETY: safety contract is ensured by the caller.
        unsafe { self.0.insert_unique_unchecked(value) }
    }
}

impl<T, S> BitOr<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: BitOr<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the union of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn bitor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitor(&rhs.0))
    }
}

impl<T, S> BitAnd<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: BitAnd<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the intersection of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn bitand(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitand(&rhs.0))
    }
}

impl<T, S> BitXor<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: BitXor<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the symmetric difference of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn bitxor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitxor(&rhs.0))
    }
}

impl<T, S> Sub<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: Sub<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the difference of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn sub(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.sub(&rhs.0))
    }
}

impl<T, S> BitOrAssign<&HashSet<T, S>> for HashSet<T, S>
where
    hb::HashSet<T, S>: for<'a> BitOrAssign<&'a hb::HashSet<T, S>>,
{
    /// Modifies this set to contain the union of `self` and `rhs`.
    #[inline]
    fn bitor_assign(&mut self, rhs: &HashSet<T, S>) {
        self.0.bitor_assign(&rhs.0);
    }
}

impl<T, S> BitAndAssign<&HashSet<T, S>> for HashSet<T, S>
where
    hb::HashSet<T, S>: for<'a> BitAndAssign<&'a hb::HashSet<T, S>>,
{
    /// Modifies this set to contain the intersection of `self` and `rhs`.
    #[inline]
    fn bitand_assign(&mut self, rhs: &HashSet<T, S>) {
        self.0.bitand_assign(&rhs.0);
    }
}

impl<T, S> BitXorAssign<&HashSet<T, S>> for HashSet<T, S>
where
    hb::HashSet<T, S>: for<'a> BitXorAssign<&'a hb::HashSet<T, S>>,
{
    /// Modifies this set to contain the symmetric difference of `self` and `rhs`.
    #[inline]
    fn bitxor_assign(&mut self, rhs: &HashSet<T, S>) {
        self.0.bitxor_assign(&rhs.0);
    }
}

impl<T, S> SubAssign<&HashSet<T, S>> for HashSet<T, S>
where
    hb::HashSet<T, S>: for<'a> SubAssign<&'a hb::HashSet<T, S>>,
{
    /// Modifies this set to contain the difference of `self` and `rhs`.
    #[inline]
    fn sub_assign(&mut self, rhs: &HashSet<T, S>) {
        self.0.sub_assign(&rhs.0);
    }
}
