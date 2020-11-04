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
pub type AhashMap<K, V> = std::collections::HashMap<K, V, RandomState>;

pub trait AhashExt {
    fn new() -> Self;

    fn with_capacity(capacity: usize) -> Self;
}

impl<K, V> AhashExt for AhashMap<K, V> {
    /// Creates an empty `HashMap` with Ahash.
    ///
    /// The hash map is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{AhashMap, AhashExt};
    /// let mut map: AhashMap<&str, i32> = AhashMap::new();
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
    /// use bevy_utils::{AhashMap, AhashExt};
    /// let mut map: AhashMap<&str, i32> = AhashMap::with_capacity(10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        AhashMap::with_capacity_and_hasher(capacity, RandomState::default())
    }
}

/// A std hash set implementing Ahash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Ahash is designed for performance and is NOT cryptographically secure.
pub type AhashSet<K> = std::collections::HashSet<K, RandomState>;

impl<K> AhashExt for AhashSet<K> {
    /// Creates an empty `HashSet` with Ahash.
    ///
    /// The hash set is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{AhashSet, AhashExt};
    /// let set: AhashSet<i32> = AhashSet::new();
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
    /// use bevy_utils::{AhashSet, AhashExt};
    /// let set: AhashSet<i32> = AhashSet::with_capacity(10);
    /// assert!(set.capacity() >= 10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        AhashSet::with_capacity_and_hasher(capacity, RandomState::default())
    }
}
