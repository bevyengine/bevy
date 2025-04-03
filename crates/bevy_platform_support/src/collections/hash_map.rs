//! Provides [`HashMap`]

use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut, Index},
};

use crate::hash::FixedHasher;
use hashbrown::{hash_map as hb, Equivalent};

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

impl<K, V> HashMap<K, V, FixedHasher> {
    /// Creates an empty `HashMap`.
    #[inline]
    pub const fn new() -> Self {
        Self::with_hasher(FixedHasher)
    }

    /// Creates an empty `HashMap` with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, FixedHasher)
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Creates an empty `HashMap` which will use the given hash builder to hash
    /// keys.
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self(hb::HashMap::with_hasher(hash_builder))
    }

    /// Creates an empty `HashMap` with the specified capacity, using `hash_builder`
    /// to hash the keys.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self(hb::HashMap::with_capacity_and_hasher(
            capacity,
            hash_builder,
        ))
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Returns a reference to the map's [`BuildHasher`].
    #[inline]
    pub fn hasher(&self) -> &S {
        self.0.hasher()
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// An iterator visiting all keys in arbitrary order.
    /// The iterator element type is `&'a K`.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        self.0.keys()
    }

    /// An iterator visiting all values in arbitrary order.
    /// The iterator element type is `&'a V`.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        self.0.values()
    }

    /// An iterator visiting all values mutably in arbitrary order.
    /// The iterator element type is `&'a mut V`.
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        self.0.values_mut()
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    /// The iterator element type is `(&'a K, &'a V)`.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.0.iter()
    }

    /// An iterator visiting all key-value pairs in arbitrary order,
    /// with mutable references to the values.
    /// The iterator element type is `(&'a K, &'a mut V)`.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.0.iter_mut()
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the map, returning all key-value pairs as an iterator. Keeps the
    /// allocated memory for reuse.
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        self.0.drain()
    }

    /// Retains only the elements specified by the predicate. Keeps the
    /// allocated memory for reuse.
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.0.retain(f);
    }

    /// Drains elements which are true under the given predicate,
    /// and returns an iterator over the removed items.
    #[inline]
    pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.0.extract_if(f)
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Creates a consuming iterator visiting all the keys in arbitrary order.
    /// The map cannot be used after calling this.
    /// The iterator element type is `K`.
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        self.0.into_keys()
    }

    /// Creates a consuming iterator visiting all the values in arbitrary order.
    /// The map cannot be used after calling this.
    /// The iterator element type is `V`.
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        self.0.into_values()
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the `HashMap`. The collection may reserve more space to avoid
    /// frequent reallocations.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements to be inserted
    /// in the given `HashMap<K,V>`. The collection may reserve more space to avoid
    /// frequent reallocations.
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), hashbrown::TryReserveError> {
        self.0.try_reserve(additional)
    }

    /// Shrinks the capacity of the map as much as possible. It will drop
    /// down as much as possible while maintaining the internal rules
    /// and possibly leaving some space in accordance with the resize policy.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Shrinks the capacity of the map with a lower limit. It will drop
    /// down no lower than the supplied limit while maintaining the internal rules
    /// and possibly leaving some space in accordance with the resize policy.
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.0.shrink_to(min_capacity);
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    #[inline]
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, S> {
        self.0.entry(key)
    }

    /// Gets the given key's corresponding entry by reference in the map for in-place manipulation.
    #[inline]
    pub fn entry_ref<'a, 'b, Q>(&'a mut self, key: &'b Q) -> EntryRef<'a, 'b, K, Q, V, S>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.entry_ref(key)
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get(k)
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value<Q>(&self, k: &Q) -> Option<(&K, &V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_key_value(k)
    }

    /// Returns the key-value pair corresponding to the supplied key, with a mutable reference to value.
    #[inline]
    pub fn get_key_value_mut<Q>(&mut self, k: &Q) -> Option<(&K, &mut V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_key_value_mut(k)
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.contains_key(k)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_mut(k)
    }

    /// Attempts to get mutable references to `N` values in the map at once.
    #[inline]
    pub fn get_many_mut<Q, const N: usize>(&mut self, ks: [&Q; N]) -> [Option<&'_ mut V>; N]
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_many_mut(ks)
    }

    /// Attempts to get mutable references to `N` values in the map at once, with immutable
    /// references to the corresponding keys.
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
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.insert(k, v)
    }

    /// Tries to insert a key-value pair into the map, and returns
    /// a mutable reference to the value in the entry.
    #[inline]
    pub fn try_insert(&mut self, key: K, value: V) -> Result<&mut V, OccupiedError<'_, K, V, S>> {
        self.0.try_insert(key, value)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map. Keeps the allocated memory for reuse.
    #[inline]
    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.remove(k)
    }

    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map. Keeps the allocated memory for reuse.
    #[inline]
    pub fn remove_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.remove_entry(k)
    }

    /// Returns the total amount of memory allocated internally by the hash
    /// set, in bytes.
    #[inline]
    pub fn allocation_size(&self) -> usize {
        self.0.allocation_size()
    }
}
