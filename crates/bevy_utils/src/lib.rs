pub use ahash::AHasher;
use ahash::RandomState;
pub use instant::{Duration, Instant};
use std::{future::Future, pin::Pin};
pub use tracing;
pub use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(target_arch = "wasm32")]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A std hash map implementing AHash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// AHash is designed for performance and is NOT cryptographically secure.
pub type HashMap<K, V> = std::collections::HashMap<K, V, RandomState>;

pub trait AHashExt {
    fn new() -> Self;

    fn with_capacity(capacity: usize) -> Self;
}

impl<K, V> AHashExt for HashMap<K, V> {
    /// Creates an empty `HashMap` with AHash.
    ///
    /// The hash map is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashMap, AHashExt};
    /// let mut map: HashMap<&str, i32> = HashMap::new();
    /// ```
    #[inline]
    fn new() -> Self {
        Default::default()
    }

    /// Creates an empty `HashMap` with the specified capacity with AHash.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash map will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashMap, AHashExt};
    /// let mut map: HashMap<&str, i32> = HashMap::with_capacity(10);
    /// ```
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        HashMap::with_capacity_and_hasher(capacity, RandomState::default())
    }
}

/// A std hash set implementing AHash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// AHash is designed for performance and is NOT cryptographically secure.
pub type HashSet<K> = std::collections::HashSet<K, RandomState>;

impl<K> AHashExt for HashSet<K> {
    /// Creates an empty `HashSet` with AHash.
    ///
    /// The hash set is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_utils::{HashSet, AHashExt};
    /// let set: HashSet<i32> = HashSet::new();
    /// ```
    #[inline]
    fn new() -> Self {
        Default::default()
    }

    /// Creates an empty `HashSet` with the specified capacity with AHash.
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
