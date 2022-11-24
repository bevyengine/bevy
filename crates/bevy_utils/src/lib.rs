pub mod prelude {
    pub use crate::default;
}

pub mod futures;
pub mod label;
mod short_names;
pub use short_names::get_short_name;
pub mod synccell;

mod default;
mod float_ord;

pub use ahash::AHasher;
pub use default::default;
pub use float_ord::*;
pub use hashbrown;
pub use instant::{Duration, Instant};
pub use tracing;
pub use uuid::Uuid;

use ahash::RandomState;
use hashbrown::hash_map::RawEntryMut;
use std::{
    fmt::Debug,
    future::Future,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    ops::{Add, AddAssign, Deref, Div, Mul, RangeInclusive, Sub, SubAssign},
    pin::Pin,
};

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(target_arch = "wasm32")]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub type Entry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, RandomState>;

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

/// A [`HashMap`][hashbrown::HashMap] implementing aHash, a high
/// speed keyed hashing algorithm intended for use in in-memory hashmaps.
///
/// aHash is designed for performance and is NOT cryptographically secure.
pub type HashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;

/// A stable hash map implementing aHash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Unlike [`HashMap`] this has an iteration order that only depends on the order
/// of insertions and deletions and not a random source.
///
/// aHash is designed for performance and is NOT cryptographically secure.
pub type StableHashMap<K, V> = hashbrown::HashMap<K, V, FixedState>;

/// A [`HashSet`][hashbrown::HashSet] implementing aHash, a high
/// speed keyed hashing algorithm intended for use in in-memory hashmaps.
///
/// aHash is designed for performance and is NOT cryptographically secure.
pub type HashSet<K> = hashbrown::HashSet<K, RandomState>;

/// A stable hash set implementing aHash, a high speed keyed hashing algorithm
/// intended for use in in-memory hashmaps.
///
/// Unlike [`HashSet`] this has an iteration order that only depends on the order
/// of insertions and deletions and not a random source.
///
/// aHash is designed for performance and is NOT cryptographically secure.
pub type StableHashSet<K> = hashbrown::HashSet<K, FixedState>;

/// A pre-hashed value of a specific type. Pre-hashing enables memoization of hashes that are expensive to compute.
/// It also enables faster [`PartialEq`] comparisons by short circuiting on hash equality.
/// See [`PassHash`] and [`PassHasher`] for a "pass through" [`BuildHasher`] and [`Hasher`] implementation
/// designed to work with [`Hashed`]
/// See [`PreHashMap`] for a hashmap pre-configured to use [`Hashed`] keys.
pub struct Hashed<V, H = FixedState> {
    hash: u64,
    value: V,
    marker: PhantomData<H>,
}

impl<V: Hash, H: BuildHasher + Default> Hashed<V, H> {
    /// Pre-hashes the given value using the [`BuildHasher`] configured in the [`Hashed`] type.
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

    /// The pre-computed hash.
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

impl<V: PartialEq, H> PartialEq for Hashed<V, H> {
    /// A fast impl of [`PartialEq`] that first checks that `other`'s pre-computed hash
    /// matches this value's pre-computed hash.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.value.eq(&other.value)
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
            hash: self.hash,
            value: self.value.clone(),
            marker: PhantomData,
        }
    }
}

impl<V: Eq, H> Eq for Hashed<V, H> {}

/// A [`BuildHasher`] that results in a [`PassHasher`].
#[derive(Default)]
pub struct PassHash;

impl BuildHasher for PassHash {
    type Hasher = PassHasher;

    fn build_hasher(&self) -> Self::Hasher {
        PassHasher::default()
    }
}

#[derive(Debug, Default)]
pub struct PassHasher {
    hash: u64,
}

impl Hasher for PassHasher {
    fn write(&mut self, _bytes: &[u8]) {
        panic!("can only hash u64 using PassHasher");
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// A [`HashMap`] pre-configured to use [`Hashed`] keys and [`PassHash`] passthrough hashing.
pub type PreHashMap<K, V> = hashbrown::HashMap<Hashed<K>, V, PassHash>;

/// Extension methods intended to add functionality to [`PreHashMap`].
pub trait PreHashMapExt<K, V> {
    /// Tries to get or insert the value for the given `key` using the pre-computed hash first.
    /// If the [`PreHashMap`] does not already contain the `key`, it will clone it and insert
    /// the value returned by `func`.
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

/// General representation of progress between two values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress<T>
where
    T: Send + Sync + Copy + Add + AddAssign + Sub + SubAssign,
{
    /// The minimum value that the progress can have, inclusive.
    min: T,
    /// The maximum value that the progress can have, inlucsive.
    max: T,
    /// The current value of progress.
    value: T,
}

impl<T: Send + Sync + Copy + Add + AddAssign + Sub + SubAssign> Progress<T>
where
    T: PartialOrd<T>,
{
    /// Creates a new progress using a `value`, and a `min` and `max` that defines a `range`.
    ///
    /// The `value` must be within the bounds of the `range` or returns a [`ProgressError`].
    pub fn new(value: T, min: T, max: T) -> Result<Self, ProgressError> {
        if min < max {
            Self::from_range(value, min..=max)
        } else {
            Err(ProgressError::InvalidRange)
        }
    }

    /// Creates a new progress using a `value` and a `range`.
    ///
    /// The `value` must be within the bounds of the `range` or returns a [`ProgressError::OutOfBounds`].
    pub fn from_range(value: T, range: RangeInclusive<T>) -> Result<Self, ProgressError> {
        if range.contains(&value) {
            Ok(Self {
                value,
                min: *range.start(),
                max: *range.end(),
            })
        } else {
            Err(ProgressError::OutOfBounds)
        }
    }

    /// Gets the min bound of the progress.
    pub fn min(&self) -> T {
        self.min
    }

    /// Gets the max bound of the progress.
    pub fn max(&self) -> T {
        self.max
    }

    /// Gets the bounds of the progress.
    pub fn bounds(&self) -> RangeInclusive<T> {
        self.min..=self.max
    }

    /// Gets the current value of progress.
    pub fn progress(&self) -> T {
        self.value
    }

    /// Sets the progress to a new value and returns the new value if successful.
    ///
    /// The `value` must be within the bounds of the `range` or returns a [`ProgressError::OutOfBounds`].
    pub fn set_progress(&mut self, new_value: T) -> Result<T, ProgressError> {
        if self.bounds().contains(&new_value) {
            self.value = new_value;
            Ok(self.value)
        } else {
            Err(ProgressError::OutOfBounds)
        }
    }
}

impl Progress<f32> {
    /// Creates a new [`Progress`] using percent.
    /// `Min` = 0.0
    /// `Max` = 100.0
    pub fn from_percent(value: f32) -> Self {
        Self::from_range(value, 0.0..=100.0).unwrap()
    }

    /// Returns the current progress, normalized between 0 and 1.
    ///
    /// 0 represents value == min,
    /// 1 represents value == max.
    pub fn normalized(&self) -> f32 {
        remap_range(self.value, (self.min, self.max), (0.0, 1.0))
    }
}

impl Default for Progress<f32> {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            value: 0.0,
        }
    }
}

/// Error types for [`Progress`].
#[derive(Debug)]
pub enum ProgressError {
    // Value is outside the bounds of the Progress.
    OutOfBounds,
    /// Tried creating a new [`Progress`] using a range that was not valid.
    ///
    /// Usually by having `min` >= `max`.
    InvalidRange,
}

/// Maps a value from one range of values to a new range of values.
///
/// This is essentially an inverse linear interpolation followed by a normal linear interpolation.
#[inline]
pub fn remap_range<
    T: Add<Output = T> + Div<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy,
>(
    value: T,
    old_range: (T, T),
    new_range: (T, T),
) -> T {
    (value - old_range.0) / (old_range.1 - old_range.0) * (new_range.1 - new_range.0) + new_range.0
}
