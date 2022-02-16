mod enum_variant_meta;
pub mod label;

pub use ahash::AHasher;
pub use enum_variant_meta::*;
pub type Entry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, RandomState>;
pub use hashbrown;
use hashbrown::hash_map::RawEntryMut;
pub use instant::{Duration, Instant};
pub use tracing;
pub use uuid::Uuid;

use ahash::RandomState;
use std::{
    fmt::Debug,
    future::Future,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
};

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

/// A [`HashMap`][hashbrown::HashMap] implementing [`aHash`], a high
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
/// The standard library's [`HashMap::new`][hashbrown::HashMap::new] is
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
pub type HashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;

/// A stable std hash map implementing `aHash`, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Unlike [`HashMap`] this has an iteration order that only depends on the order
/// of insertions and deletions and not a random source.
///
/// `aHash` is designed for performance and is NOT cryptographically secure.
pub type StableHashMap<K, V> = hashbrown::HashMap<K, V, FixedState>;

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
pub type HashSet<K> = hashbrown::HashSet<K, RandomState>;

/// A stable std hash set implementing `aHash`, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Unlike [`HashSet`] this has an iteration order that only depends on the order
/// of insertions and deletions and not a random source.
///
/// `aHash` is designed for performance and is NOT cryptographically secure.
pub type StableHashSet<K> = hashbrown::HashSet<K, FixedState>;

pub struct Hashed<V, H = FixedState> {
    hash: u64,
    value: V,
    marker: PhantomData<H>,
}

impl<V: Hash, H: BuildHasher + Default> Hashed<V, H> {
    pub fn new(value: V) -> Self {
        let builder = H::default();
        let mut hasher = builder.build_hasher();
        value.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
            value,
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

impl<V, H> Hash for Hashed<V, H> {
    #[inline]
    fn hash<R: Hasher>(&self, state: &mut R) {
        state.write_u64(self.hash);
    }
}

impl<V, H> Deref for Hashed<V, H> {
    type Target = V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V: PartialEq, H> Hashed<V, H> {
    #[inline]
    pub fn fast_eq(&self, other: &Hashed<V, H>) -> bool {
        // Makes the common case of two values not being equal very fast
        self.hash == other.hash && self.value.eq(&other.value)
    }
}

impl<V: PartialEq, H> PartialEq for Hashed<V, H> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<V: Debug, H> Debug for Hashed<V, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hashed")
            .field("hash", &self.hash)
            .field("value", &self.value)
            .finish()
    }
}

impl<V: Clone, H> Clone for Hashed<V, H> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            value: self.value.clone(),
            marker: PhantomData,
        }
    }
}

impl<V: Eq, H> Eq for Hashed<V, H> {}

#[derive(Default)]
pub struct PassHash;

impl BuildHasher for PassHash {
    type Hasher = PassHasher;

    fn build_hasher(&self) -> Self::Hasher {
        PassHasher::default()
    }
}

#[derive(Debug)]
pub struct PassHasher {
    hash: u64,
}

impl Default for PassHasher {
    fn default() -> Self {
        Self { hash: 0 }
    }
}

impl Hasher for PassHasher {
    fn write(&mut self, _bytes: &[u8]) {
        panic!("cannot hash byte arrays using PassHasher");
    }

    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type PreHashMap<K, V> = hashbrown::HashMap<Hashed<K>, V, PassHash>;

pub trait PreHashMapExt<K, V> {
    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: &Hashed<K>, func: F) -> &mut V;
}

impl<K: Hash + Eq + PartialEq + Clone, V> PreHashMapExt<K, V> for PreHashMap<K, V> {
    #[inline]
    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: &Hashed<K>, func: F) -> &mut V {
        let entry = self
            .raw_entry_mut()
            .from_key_hashed_nocheck(key.hash(), key);
        match entry {
            RawEntryMut::Occupied(entry) => entry.into_mut(),
            RawEntryMut::Vacant(entry) => {
                let (_, value) = entry.insert_hashed_nocheck(key.hash(), key.clone(), func());
                value
            }
        }
    }
}
