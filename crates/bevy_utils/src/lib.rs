use ahash::RandomState;
use std::{future::Future, pin::Pin};

pub use ahash::AHasher;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(target_arch = "wasm32")]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A std hash map implementing Ahash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Ahash is designed for performance and is NOT cryptographically secure.
pub type HashMap<K, V> = std::collections::HashMap<K, V, RandomState>;

pub trait AhashExt {
    fn new() -> Self;

    fn with_capacity(capacity: usize) -> Self;
}

impl<K, V> AhashExt for HashMap<K, V> {
    /// Creates an empty `HashMap` with Ahash.
    ///
    /// The hash map is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashMap, AhashExt};
    /// let mut map: HashMap<&str, i32> = HashMap::new();
    /// ```
    #[inline]
    fn new() -> Self {
        Default::default()
    }

    /// Creates an empty `HashMap` with the specified capacity with Ahash.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash map will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashMap, AhashExt};
    /// let mut map: HashMap<&str, i32> = HashMap::with_capacity(10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        HashMap::with_capacity_and_hasher(capacity, RandomState::default())
    }
}

/// A std hash set implementing Ahash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Ahash is designed for performance and is NOT cryptographically secure.
pub type HashSet<K> = std::collections::HashSet<K, RandomState>;

impl<K> AhashExt for HashSet<K> {
    /// Creates an empty `HashSet` with Ahash.
    ///
    /// The hash set is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashSet, AhashExt};
    /// let set: HashSet<i32> = HashSet::new();
    /// ```
    #[inline]
    fn new() -> Self {
        Default::default()
    }

    /// Creates an empty `HashSet` with the specified capacity with Ahash.
    ///
    /// The hash set will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash set will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashSet, AhashExt};
    /// let set: HashSet<i32> = HashSet::with_capacity(10);
    /// assert!(set.capacity() >= 10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        HashSet::with_capacity_and_hasher(capacity, RandomState::default())
    }
}
