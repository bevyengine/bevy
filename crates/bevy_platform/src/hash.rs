//! Provides replacements for `std::hash` items using [`foldhash`].
//!
//! Also provides some additional items beyond the standard library.

use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
};

pub use foldhash::fast::{FixedState, FoldHasher as DefaultHasher, RandomState};

/// For when you want a deterministic hasher.
///
/// Seed was randomly generated with a fair dice roll. Guaranteed to be random:
/// <https://github.com/bevyengine/bevy/pull/1268/files#r560918426>
const FIXED_HASHER: FixedState =
    FixedState::with_seed(0b1001010111101110000001001100010000000011001001101011001001111000);

/// Deterministic hasher based upon a random but fixed state.
#[derive(Copy, Clone, Default, Debug)]
pub struct FixedHasher;
impl BuildHasher for FixedHasher {
    type Hasher = DefaultHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        FIXED_HASHER.build_hasher()
    }
}

/// A pre-hashed value of a specific type. Pre-hashing enables memoization of hashes that are expensive to compute.
///
/// It also enables faster [`PartialEq`] comparisons by short circuiting on hash equality.
/// See [`PassHash`] and [`PassHasher`] for a "pass through" [`BuildHasher`] and [`Hasher`] implementation
/// designed to work with [`Hashed`]
/// See `PreHashMap` for a hashmap pre-configured to use [`Hashed`] keys.
pub struct Hashed<V, S = FixedHasher> {
    hash: u64,
    value: V,
    marker: PhantomData<S>,
}

impl<V: Hash, H: BuildHasher + Default> Hashed<V, H> {
    /// Pre-hashes the given value using the [`BuildHasher`] configured in the [`Hashed`] type.
    pub fn new(value: V) -> Self {
        Self {
            hash: H::default().hash_one(&value),
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

impl<V: Copy, H> Copy for Hashed<V, H> {}

impl<V: Eq, H> Eq for Hashed<V, H> {}

/// A [`BuildHasher`] that results in a [`PassHasher`].
#[derive(Default, Clone)]
pub struct PassHash;

impl BuildHasher for PassHash {
    type Hasher = PassHasher;

    fn build_hasher(&self) -> Self::Hasher {
        PassHasher::default()
    }
}

/// A no-op hash that only works on `u64`s. Will panic if attempting to
/// hash a type containing non-u64 fields.
#[derive(Debug, Default)]
pub struct PassHasher {
    hash: u64,
}

impl Hasher for PassHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, _bytes: &[u8]) {
        panic!("can only hash u64 using PassHasher");
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }
}

/// [`BuildHasher`] for types that already contain a high-quality hash.
#[derive(Clone, Default)]
pub struct NoOpHash;

impl BuildHasher for NoOpHash {
    type Hasher = NoOpHasher;

    fn build_hasher(&self) -> Self::Hasher {
        NoOpHasher(0)
    }
}

#[doc(hidden)]
pub struct NoOpHasher(u64);

// This is for types that already contain a high-quality hash and want to skip
// re-hashing that hash.
impl Hasher for NoOpHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // This should never be called by consumers. Prefer to call `write_u64` instead.
        // Don't break applications (slower fallback, just check in test):
        self.0 = bytes.iter().fold(self.0, |hash, b| {
            hash.rotate_left(8).wrapping_add(*b as u64)
        });
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}
