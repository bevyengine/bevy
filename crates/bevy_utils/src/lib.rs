mod enum_variant_meta;
pub mod label;

pub use ahash::AHasher;
pub use enum_variant_meta::*;
pub use instant::{Duration, Instant};
pub use tracing;
pub use uuid::Uuid;

use ahash::RandomState;
use std::{future::Future, pin::Pin};

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(target_arch = "wasm32")]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A hasher builder that will create a fixed hasher.
#[derive(Debug, Clone, Default)]
pub struct FixedState;

impl std::hash::BuildHasher for FixedState {
    type Hasher = AHasher;

    #[inline]
    fn build_hasher(&self) -> AHasher {
        AHasher::new_with_keys(
            0b1001010111101110000001001100010000000011001001101011001001111000,
            0b1100111101101011011110001011010100000100001111100011010011010101,
        )
    }
}

/// A [`HashMap`][std::collections::HashMap] implementing [`aHash`], a high
/// speed keyed hashing algorithm intended for use in in-memory hashmaps.
///
/// `aHash` is designed for performance and is NOT cryptographically secure.
///
/// # Construction
///
/// Users may be surprised when a `HashMap` cannot be constructed with `HashMap::new()`:
///
/// ```compile_fail
/// # fn main() {
/// use bevy_utils::HashMap;
///
/// // Produces an error like "no function or associated item named `new` found [...]"
/// let map: HashMap<String, String> = HashMap::new();
/// # }
/// ```
///
/// The standard library's [`HashMap::new`][std::collections::HashMap::new] is
/// implemented only for `HashMap`s which use the
/// [`DefaultHasher`][std::collections::hash_map::DefaultHasher], so it's not
/// available for Bevy's `HashMap`.
///
/// However, an empty `HashMap` can easily be constructed using the `Default`
/// implementation:
///
/// ```
/// # fn main() {
/// use bevy_utils::HashMap;
///
/// // This works!
/// let map: HashMap<String, String> = HashMap::default();
/// assert!(map.is_empty());
/// # }
/// ```
///
/// [`aHash`]: https://github.com/tkaitchuck/aHash
pub type HashMap<K, V> = std::collections::HashMap<K, V, RandomState>;

pub trait AHashExt {
    fn with_capacity(capacity: usize) -> Self;
}

impl<K, V> AHashExt for HashMap<K, V> {
    /// Creates an empty `HashMap` with the specified capacity with aHash.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash map will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashMap, AHashExt};
    /// let mut map: HashMap<&str, i32> = HashMap::with_capacity(10);
    /// assert!(map.capacity() >= 10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        HashMap::with_capacity_and_hasher(capacity, RandomState::default())
    }
}

/// A stable std hash map implementing `aHash`, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Unlike [`HashMap`] this has an iteration order that only depends on the order
/// of insertions and deletions and not a random source.
///
/// `aHash` is designed for performance and is NOT cryptographically secure.
pub type StableHashMap<K, V> = std::collections::HashMap<K, V, FixedState>;

impl<K, V> AHashExt for StableHashMap<K, V> {
    /// Creates an empty `StableHashMap` with the specified capacity with `aHash`.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash map will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{StableHashMap, AHashExt};
    /// let mut map: StableHashMap<&str, i32> = StableHashMap::with_capacity(10);
    /// assert!(map.capacity() >= 10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        StableHashMap::with_capacity_and_hasher(capacity, FixedState::default())
    }
}

/// A [`HashSet`][std::collections::HashSet] implementing [`aHash`], a high
/// speed keyed hashing algorithm intended for use in in-memory hashmaps.
///
/// `aHash` is designed for performance and is NOT cryptographically secure.
///
/// # Construction
///
/// Users may be surprised when a `HashSet` cannot be constructed with `HashSet::new()`:
///
/// ```compile_fail
/// # fn main() {
/// use bevy_utils::HashSet;
///
/// // Produces an error like "no function or associated item named `new` found [...]"
/// let map: HashSet<String> = HashSet::new();
/// # }
/// ```
///
/// The standard library's [`HashSet::new`][std::collections::HashSet::new] is
/// implemented only for `HashSet`s which use the
/// [`DefaultHasher`][std::collections::hash_map::DefaultHasher], so it's not
/// available for Bevy's `HashSet`.
///
/// However, an empty `HashSet` can easily be constructed using the `Default`
/// implementation:
///
/// ```
/// # fn main() {
/// use bevy_utils::HashSet;
///
/// // This works!
/// let map: HashSet<String> = HashSet::default();
/// assert!(map.is_empty());
/// # }
/// ```
///
/// [`aHash`]: https://github.com/tkaitchuck/aHash
pub type HashSet<K> = std::collections::HashSet<K, RandomState>;

impl<K> AHashExt for HashSet<K> {
    /// Creates an empty `HashSet` with the specified capacity with aHash.
    ///
    /// The hash set will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash set will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashSet, AHashExt};
    /// let set: HashSet<i32> = HashSet::with_capacity(10);
    /// assert!(set.capacity() >= 10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        HashSet::with_capacity_and_hasher(capacity, RandomState::default())
    }
}

/// A stable std hash set implementing `aHash`, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Unlike [`HashSet`] this has an iteration order that only depends on the order
/// of insertions and deletions and not a random source.
///
/// `aHash` is designed for performance and is NOT cryptographically secure.
pub type StableHashSet<K> = std::collections::HashSet<K, FixedState>;

impl<K> AHashExt for StableHashSet<K> {
    /// Creates an empty `StableHashSet` with the specified capacity with `aHash`.
    ///
    /// The hash set will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash set will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{StableHashSet, AHashExt};
    /// let set: StableHashSet<i32> = StableHashSet::with_capacity(10);
    /// assert!(set.capacity() >= 10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        StableHashSet::with_capacity_and_hasher(capacity, FixedState::default())
    }
}
