//! Provides [`HashMap`] based on [hashbrown]'s implementation.
//! Unlike [`hashbrown::HashMap`], [`HashMap`] defaults to [`FixedHasher`]
//! instead of [`RandomState`].
//! This provides determinism by default with an acceptable compromise to denial
//! of service resistance in the context of a game engine.

use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut, Index},
};

use hashbrown::{hash_map as hb, Equivalent};

use crate::hash::FixedHasher;

#[cfg(feature = "rayon")]
use rayon::prelude::{FromParallelIterator, IntoParallelIterator, ParallelExtend};

// Re-exports to match `std::collections::hash_map`
pub use {
    crate::hash::{DefaultHasher, RandomState},
    hb::{
        Drain, IntoIter, IntoKeys, IntoValues, Iter, IterMut, Keys, OccupiedEntry, VacantEntry,
        Values, ValuesMut,
    },
};

// Additional items from `hashbrown`
pub use hb::{
    EntryRef, ExtractIf, OccupiedError, RawEntryBuilder, RawEntryBuilderMut, RawEntryMut,
    RawOccupiedEntryMut,
};

/// Shortcut for [`Entry`](hb::Entry) with [`FixedHasher`] as the default hashing provider.
pub type Entry<'a, K, V, S = FixedHasher> = hb::Entry<'a, K, V, S>;

/// New-type for [`HashMap`](hb::HashMap) with [`FixedHasher`] as the default hashing provider.
/// Can be trivially converted to and from a [hashbrown] [`HashMap`](hb::HashMap) using [`From`].
///
/// A new-type is used instead of a type alias due to critical methods like [`new`](hb::HashMap::new)
/// being incompatible with Bevy's choice of default hasher.
#[repr(transparent)]
pub struct HashMap<K, V, S = FixedHasher>(hb::HashMap<K, V, S>);

impl<K, V, S> Clone for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Clone,
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

impl<K, V, S> Debug for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <hb::HashMap<K, V, S> as Debug>::fmt(&self.0, f)
    }
}

impl<K, V, S> Default for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Default,
{
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K, V, S> PartialEq for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<K, V, S> Eq for HashMap<K, V, S> where hb::HashMap<K, V, S>: Eq {}

impl<K, V, S, T> FromIterator<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: FromIterator<T>,
{
    #[inline]
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}

impl<K, V, S, T> Index<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Index<T>,
{
    type Output = <hb::HashMap<K, V, S> as Index<T>>::Output;

    #[inline]
    fn index(&self, index: T) -> &Self::Output {
        self.0.index(index)
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: IntoIterator,
{
    type Item = <hb::HashMap<K, V, S> as IntoIterator>::Item;

    type IntoIter = <hb::HashMap<K, V, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S>
where
    &'a hb::HashMap<K, V, S>: IntoIterator,
{
    type Item = <&'a hb::HashMap<K, V, S> as IntoIterator>::Item;

    type IntoIter = <&'a hb::HashMap<K, V, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S>
where
    &'a mut hb::HashMap<K, V, S>: IntoIterator,
{
    type Item = <&'a mut hb::HashMap<K, V, S> as IntoIterator>::Item;

    type IntoIter = <&'a mut hb::HashMap<K, V, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<K, V, S, T> Extend<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Extend<T>,
{
    #[inline]
    fn extend<U: IntoIterator<Item = T>>(&mut self, iter: U) {
        self.0.extend(iter);
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, FixedHasher>
where
    K: Eq + Hash,
{
    fn from(arr: [(K, V); N]) -> Self {
        arr.into_iter().collect()
    }
}

impl<K, V, S> From<hb::HashMap<K, V, S>> for HashMap<K, V, S> {
    #[inline]
    fn from(value: hb::HashMap<K, V, S>) -> Self {
        Self(value)
    }
}

impl<K, V, S> From<HashMap<K, V, S>> for hb::HashMap<K, V, S> {
    #[inline]
    fn from(value: HashMap<K, V, S>) -> Self {
        value.0
    }
}

impl<K, V, S> Deref for HashMap<K, V, S> {
    type Target = hb::HashMap<K, V, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, S> DerefMut for HashMap<K, V, S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "serialize")]
impl<K, V, S> serde::Serialize for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: serde::Serialize,
{
    #[inline]
    fn serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, K, V, S> serde::Deserialize<'de> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: serde::Deserialize<'de>,
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
impl<K, V, S, T> FromParallelIterator<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: FromParallelIterator<T>,
    T: Send,
{
    fn from_par_iter<P>(par_iter: P) -> Self
    where
        P: IntoParallelIterator<Item = T>,
    {
        Self(<hb::HashMap<K, V, S> as FromParallelIterator<T>>::from_par_iter(par_iter))
    }
}

#[cfg(feature = "rayon")]
impl<K, V, S> IntoParallelIterator for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: IntoParallelIterator,
{
    type Item = <hb::HashMap<K, V, S> as IntoParallelIterator>::Item;
    type Iter = <hb::HashMap<K, V, S> as IntoParallelIterator>::Iter;

    fn into_par_iter(self) -> Self::Iter {
        self.0.into_par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a HashMap<K, V, S>
where
    &'a hb::HashMap<K, V, S>: IntoParallelIterator,
{
    type Item = <&'a hb::HashMap<K, V, S> as IntoParallelIterator>::Item;
    type Iter = <&'a hb::HashMap<K, V, S> as IntoParallelIterator>::Iter;

    fn into_par_iter(self) -> Self::Iter {
        (&self.0).into_par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a mut HashMap<K, V, S>
where
    &'a mut hb::HashMap<K, V, S>: IntoParallelIterator,
{
    type Item = <&'a mut hb::HashMap<K, V, S> as IntoParallelIterator>::Item;
    type Iter = <&'a mut hb::HashMap<K, V, S> as IntoParallelIterator>::Iter;

    fn into_par_iter(self) -> Self::Iter {
        (&mut self.0).into_par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<K, V, S, T> ParallelExtend<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: ParallelExtend<T>,
    T: Send,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        <hb::HashMap<K, V, S> as ParallelExtend<T>>::par_extend(&mut self.0, par_iter);
    }
}

impl<K, V> HashMap<K, V, FixedHasher> {
    /// Creates an empty [`HashMap`].
    ///
    /// Refer to [`new`](hb::HashMap::new) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// // Creates a HashMap with zero capacity.
    /// let map = HashMap::new();
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self::with_hasher(FixedHasher)
    }

    /// Creates an empty [`HashMap`] with the specified capacity.
    ///
    /// Refer to [`with_capacity`](hb::HashMap::with_capacity) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// // Creates a HashMap with capacity for at least 5 entries.
    /// let map = HashMap::with_capacity(5);
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, FixedHasher)
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Creates an empty [`HashMap`] which will use the given hash builder to hash
    /// keys.
    ///
    /// Refer to [`with_hasher`](hb::HashMap::with_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashMap with the provided hasher.
    /// let map = HashMap::with_hasher(SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self(hb::HashMap::with_hasher(hash_builder))
    }

    /// Creates an empty [`HashMap`] with the specified capacity, using `hash_builder`
    /// to hash the keys.
    ///
    /// Refer to [`with_capacity_and_hasher`](hb::HashMap::with_capacity_and_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashMap with capacity for 5 entries and the provided hasher.
    /// let map = HashMap::with_capacity_and_hasher(5, SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self(hb::HashMap::with_capacity_and_hasher(
            capacity,
            hash_builder,
        ))
    }

    /// Returns a reference to the map's [`BuildHasher`], or `S` parameter.
    ///
    /// Refer to [`hasher`](hb::HashMap::hasher) for further details.
    #[inline]
    pub fn hasher(&self) -> &S {
        self.0.hasher()
    }

    /// Returns the number of elements the map can hold without reallocating.
    ///
    /// Refer to [`capacity`](hb::HashMap::capacity) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let map = HashMap::with_capacity(5);
    ///
    /// # let map: HashMap<(), ()> = map;
    /// #
    /// assert!(map.capacity() >= 5);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// An iterator visiting all keys in arbitrary order.
    /// The iterator element type is `&'a K`.
    ///
    /// Refer to [`keys`](hb::HashMap::keys) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for key in map.keys() {
    ///     // foo, bar, baz
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.keys().count(), 3);
    /// ```
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        self.0.keys()
    }

    /// An iterator visiting all values in arbitrary order.
    /// The iterator element type is `&'a V`.
    ///
    /// Refer to [`values`](hb::HashMap::values) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for key in map.values() {
    ///     // 0, 1, 2
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.values().count(), 3);
    /// ```
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        self.0.values()
    }

    /// An iterator visiting all values mutably in arbitrary order.
    /// The iterator element type is `&'a mut V`.
    ///
    /// Refer to [`values`](hb::HashMap::values) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for key in map.values_mut() {
    ///     // 0, 1, 2
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.values_mut().count(), 3);
    /// ```
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        self.0.values_mut()
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    /// The iterator element type is `(&'a K, &'a V)`.
    ///
    /// Refer to [`iter`](hb::HashMap::iter) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for (key, value) in map.iter() {
    ///     // ("foo", 0), ("bar", 1), ("baz", 2)
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.iter().count(), 3);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.0.iter()
    }

    /// An iterator visiting all key-value pairs in arbitrary order,
    /// with mutable references to the values.
    /// The iterator element type is `(&'a K, &'a mut V)`.
    ///
    /// Refer to [`iter_mut`](hb::HashMap::iter_mut) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for (key, value) in map.iter_mut() {
    ///     // ("foo", 0), ("bar", 1), ("baz", 2)
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.iter_mut().count(), 3);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.0.iter_mut()
    }

    /// Returns the number of elements in the map.
    ///
    /// Refer to [`len`](hb::HashMap::len) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// assert_eq!(map.len(), 0);
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// Refer to [`is_empty`](hb::HashMap::is_empty) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// assert!(map.is_empty());
    ///
    /// map.insert("foo", 0);
    ///
    /// assert!(!map.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the map, returning all key-value pairs as an iterator. Keeps the
    /// allocated memory for reuse.
    ///
    /// Refer to [`drain`](hb::HashMap::drain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for (key, value) in map.drain() {
    ///     // ("foo", 0), ("bar", 1), ("baz", 2)
    ///     // Note that the above order is not guaranteed
    /// }
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        self.0.drain()
    }

    /// Retains only the elements specified by the predicate. Keeps the
    /// allocated memory for reuse.
    ///
    /// Refer to [`retain`](hb::HashMap::retain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// map.retain(|key, value| *value == 2);
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.0.retain(f);
    }

    /// Drains elements which are true under the given predicate,
    /// and returns an iterator over the removed items.
    ///
    /// Refer to [`extract_if`](hb::HashMap::extract_if) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// let extracted = map
    ///     .extract_if(|key, value| *value == 2)
    ///     .collect::<Vec<_>>();
    ///
    /// assert_eq!(map.len(), 2);
    /// assert_eq!(extracted.len(), 1);
    /// ```
    #[inline]
    pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.0.extract_if(f)
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    ///
    /// Refer to [`clear`](hb::HashMap::clear) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// map.clear();
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Creates a consuming iterator visiting all the keys in arbitrary order.
    /// The map cannot be used after calling this.
    /// The iterator element type is `K`.
    ///
    /// Refer to [`into_keys`](hb::HashMap::into_keys) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for key in map.into_keys() {
    ///     // "foo", "bar", "baz"
    ///     // Note that the above order is not guaranteed
    /// }
    /// ```
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        self.0.into_keys()
    }

    /// Creates a consuming iterator visiting all the values in arbitrary order.
    /// The map cannot be used after calling this.
    /// The iterator element type is `V`.
    ///
    /// Refer to [`into_values`](hb::HashMap::into_values) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for key in map.into_values() {
    ///     // 0, 1, 2
    ///     // Note that the above order is not guaranteed
    /// }
    /// ```
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        self.0.into_values()
    }

    /// Takes the inner [`HashMap`](hb::HashMap) out of this wrapper.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let map: HashMap<&'static str, usize> = HashMap::new();
    /// let map: hashbrown::HashMap<&'static str, usize, _> = map.into_inner();
    /// ```
    #[inline]
    pub fn into_inner(self) -> hb::HashMap<K, V, S> {
        self.0
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the [`HashMap`]. The collection may reserve more space to avoid
    /// frequent reallocations.
    ///
    /// Refer to [`reserve`](hb::HashMap::reserve) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::with_capacity(5);
    ///
    /// # let mut map: HashMap<(), ()> = map;
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
    /// in the given `HashMap<K,V>`. The collection may reserve more space to avoid
    /// frequent reallocations.
    ///
    /// Refer to [`try_reserve`](hb::HashMap::try_reserve) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::with_capacity(5);
    ///
    /// # let mut map: HashMap<(), ()> = map;
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

    /// Shrinks the capacity of the map as much as possible. It will drop
    /// down as much as possible while maintaining the internal rules
    /// and possibly leaving some space in accordance with the resize policy.
    ///
    /// Refer to [`shrink_to_fit`](hb::HashMap::shrink_to_fit) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::with_capacity(5);
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
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

    /// Shrinks the capacity of the map with a lower limit. It will drop
    /// down no lower than the supplied limit while maintaining the internal rules
    /// and possibly leaving some space in accordance with the resize policy.
    ///
    /// Refer to [`shrink_to`](hb::HashMap::shrink_to) for further details.
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.0.shrink_to(min_capacity);
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    ///
    /// Refer to [`entry`](hb::HashMap::entry) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// let value = map.entry("foo").or_insert(0);
    /// #
    /// # assert_eq!(*value, 0);
    /// ```
    #[inline]
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, S> {
        self.0.entry(key)
    }

    /// Gets the given key's corresponding entry by reference in the map for in-place manipulation.
    ///
    /// Refer to [`entry_ref`](hb::HashMap::entry_ref) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    /// # let mut map: HashMap<&'static str, usize> = map;
    ///
    /// let value = map.entry_ref("foo").or_insert(0);
    /// #
    /// # assert_eq!(*value, 0);
    /// ```
    #[inline]
    pub fn entry_ref<'a, 'b, Q>(&'a mut self, key: &'b Q) -> EntryRef<'a, 'b, K, Q, V, S>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.entry_ref(key)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// Refer to [`get`](hb::HashMap::get) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get("foo"), Some(&0));
    /// ```
    #[inline]
    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get(k)
    }

    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// Refer to [`get_key_value`](hb::HashMap::get_key_value) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get_key_value("foo"), Some((&"foo", &0)));
    /// ```
    #[inline]
    pub fn get_key_value<Q>(&self, k: &Q) -> Option<(&K, &V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_key_value(k)
    }

    /// Returns the key-value pair corresponding to the supplied key, with a mutable reference to value.
    ///
    /// Refer to [`get_key_value_mut`](hb::HashMap::get_key_value_mut) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get_key_value_mut("foo"), Some((&"foo", &mut 0)));
    /// ```
    #[inline]
    pub fn get_key_value_mut<Q>(&mut self, k: &Q) -> Option<(&K, &mut V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_key_value_mut(k)
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// Refer to [`contains_key`](hb::HashMap::contains_key) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert!(map.contains_key("foo"));
    /// ```
    #[inline]
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.contains_key(k)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// Refer to [`get_mut`](hb::HashMap::get_mut) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get_mut("foo"), Some(&mut 0));
    /// ```
    #[inline]
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_mut(k)
    }

    /// Attempts to get mutable references to `N` values in the map at once.
    ///
    /// Refer to [`get_many_mut`](hb::HashMap::get_many_mut) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// let result = map.get_many_mut(["foo", "bar"]);
    ///
    /// assert_eq!(result, [Some(&mut 0), Some(&mut 1)]);
    /// ```
    #[inline]
    pub fn get_many_mut<Q, const N: usize>(&mut self, ks: [&Q; N]) -> [Option<&'_ mut V>; N]
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_many_mut(ks)
    }

    /// Attempts to get mutable references to `N` values in the map at once, with immutable
    /// references to the corresponding keys.
    ///
    /// Refer to [`get_many_key_value_mut`](hb::HashMap::get_many_key_value_mut) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// let result = map.get_many_key_value_mut(["foo", "bar"]);
    ///
    /// assert_eq!(result, [Some((&"foo", &mut 0)), Some((&"bar", &mut 1))]);
    /// ```
    #[inline]
    pub fn get_many_key_value_mut<Q, const N: usize>(
        &mut self,
        ks: [&Q; N],
    ) -> [Option<(&'_ K, &'_ mut V)>; N]
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_many_key_value_mut(ks)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// Refer to [`insert`](hb::HashMap::insert) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get("foo"), Some(&0));
    /// ```
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.insert(k, v)
    }

    /// Tries to insert a key-value pair into the map, and returns
    /// a mutable reference to the value in the entry.
    ///
    /// Refer to [`try_insert`](hb::HashMap::try_insert) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.try_insert("foo", 0).unwrap();
    ///
    /// assert!(map.try_insert("foo", 1).is_err());
    /// ```
    #[inline]
    pub fn try_insert(&mut self, key: K, value: V) -> Result<&mut V, OccupiedError<'_, K, V, S>> {
        self.0.try_insert(key, value)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map. Keeps the allocated memory for reuse.
    ///
    /// Refer to [`remove`](hb::HashMap::remove) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.remove("foo"), Some(0));
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.remove(k)
    }

    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map. Keeps the allocated memory for reuse.
    ///
    /// Refer to [`remove_entry`](hb::HashMap::remove_entry) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.remove_entry("foo"), Some(("foo", 0)));
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn remove_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.remove_entry(k)
    }

    /// Returns the total amount of memory allocated internally by the hash
    /// set, in bytes.
    ///
    /// Refer to [`allocation_size`](hb::HashMap::allocation_size) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// assert_eq!(map.allocation_size(), 0);
    ///
    /// map.insert("foo", 0u32);
    ///
    /// assert!(map.allocation_size() >= size_of::<&'static str>() + size_of::<u32>());
    /// ```
    #[inline]
    pub fn allocation_size(&self) -> usize {
        self.0.allocation_size()
    }

    /// Insert a key-value pair into the map without checking
    /// if the key already exists in the map.
    ///
    /// Refer to [`insert_unique_unchecked`](hb::HashMap::insert_unique_unchecked) for further details.
    ///
    /// # Safety
    ///
    /// This operation is safe if a key does not exist in the map.
    ///
    /// However, if a key exists in the map already, the behavior is unspecified:
    /// this operation may panic, loop forever, or any following operation with the map
    /// may panic, loop forever or return arbitrary result.
    ///
    /// That said, this operation (and following operations) are guaranteed to
    /// not violate memory safety.
    ///
    /// However this operation is still unsafe because the resulting `HashMap`
    /// may be passed to unsafe code which does expect the map to behave
    /// correctly, and would cause unsoundness as a result.
    #[expect(
        unsafe_code,
        reason = "re-exporting unsafe method from Hashbrown requires unsafe code"
    )]
    #[inline]
    pub unsafe fn insert_unique_unchecked(&mut self, key: K, value: V) -> (&K, &mut V) {
        // SAFETY: safety contract is ensured by the caller.
        unsafe { self.0.insert_unique_unchecked(key, value) }
    }

    /// Attempts to get mutable references to `N` values in the map at once, without validating that
    /// the values are unique.
    ///
    /// Refer to [`get_many_unchecked_mut`](hb::HashMap::get_many_unchecked_mut) for further details.
    ///
    /// Returns an array of length `N` with the results of each query. `None` will be used if
    /// the key is missing.
    ///
    /// For a safe alternative see [`get_many_mut`](`HashMap::get_many_mut`).
    ///
    /// # Safety
    ///
    /// Calling this method with overlapping keys is *[undefined behavior]* even if the resulting
    /// references are not used.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    #[expect(
        unsafe_code,
        reason = "re-exporting unsafe method from Hashbrown requires unsafe code"
    )]
    #[inline]
    pub unsafe fn get_many_unchecked_mut<Q, const N: usize>(
        &mut self,
        keys: [&Q; N],
    ) -> [Option<&'_ mut V>; N]
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        // SAFETY: safety contract is ensured by the caller.
        unsafe { self.0.get_many_unchecked_mut(keys) }
    }

    /// Attempts to get mutable references to `N` values in the map at once, with immutable
    /// references to the corresponding keys, without validating that the values are unique.
    ///
    /// Refer to [`get_many_key_value_unchecked_mut`](hb::HashMap::get_many_key_value_unchecked_mut) for further details.
    ///
    /// Returns an array of length `N` with the results of each query. `None` will be returned if
    /// any of the keys are missing.
    ///
    /// For a safe alternative see [`get_many_key_value_mut`](`HashMap::get_many_key_value_mut`).
    ///
    /// # Safety
    ///
    /// Calling this method with overlapping keys is *[undefined behavior]* even if the resulting
    /// references are not used.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    #[expect(
        unsafe_code,
        reason = "re-exporting unsafe method from Hashbrown requires unsafe code"
    )]
    #[inline]
    pub unsafe fn get_many_key_value_unchecked_mut<Q, const N: usize>(
        &mut self,
        keys: [&Q; N],
    ) -> [Option<(&'_ K, &'_ mut V)>; N]
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        // SAFETY: safety contract is ensured by the caller.
        unsafe { self.0.get_many_key_value_unchecked_mut(keys) }
    }
}
