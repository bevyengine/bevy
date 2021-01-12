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

/// A std hash set implementing AHash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// AHash is designed for performance and is NOT cryptographically secure.
pub type HashSet<K> = std::collections::HashSet<K, RandomState>;
