use ahash::RandomState;
use std::{future::Future, pin::Pin};

pub use ahash::AHasher;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(target_arch = "wasm32")]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub type AhashMap<K, V> = std::collections::HashMap<K, V, RandomState>;
pub type AhashSet<K> = std::collections::HashSet<K, RandomState>;

pub trait AhashExt {
    fn with_capacity(cap: usize) -> Self;
}

impl<K, V> AhashExt for AhashMap<K, V> {
    fn with_capacity(cap: usize) -> Self {
        AhashMap::with_capacity_and_hasher(cap, RandomState::default())
    }
}
