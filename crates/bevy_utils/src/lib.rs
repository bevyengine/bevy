//! General utilities for first-party [Bevy] engine crates.
//!
//! [Bevy]: https://bevyengine.org/
//!
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]

#[allow(missing_docs)]
pub mod prelude {
    pub use crate::default;
}

pub mod futures;
pub mod label;
mod short_names;
pub use short_names::get_short_name;
pub mod synccell;
pub mod syncunsafecell;

mod cow_arc;
mod default;
mod float_ord;

pub use ahash::{AHasher, RandomState};
pub use bevy_utils_proc_macros::*;
pub use cow_arc::*;
pub use default::default;
pub use float_ord::*;
pub use hashbrown;
pub use instant::{Duration, Instant};
pub use petgraph;
pub use thiserror;
pub use tracing;
pub use uuid::Uuid;

#[allow(missing_docs)]
pub mod nonmax {
    pub use nonmax::*;
}

use hashbrown::hash_map::RawEntryMut;
use std::{
    fmt::Debug,
    future::Future,
    hash::{BuildHasher, BuildHasherDefault, Hash, Hasher},
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
};

/// An owned and dynamically typed Future used when you can't statically type your result or need to add some indirection.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[allow(missing_docs)]
#[cfg(target_arch = "wasm32")]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A shortcut alias for [`hashbrown::hash_map::Entry`].
pub type Entry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, BuildHasherDefault<AHasher>>;

/// A hasher builder that will create a fixed hasher.
#[derive(Debug, Clone, Default)]
pub struct FixedState;

impl std::hash::BuildHasher for FixedState {
    type Hasher = AHasher;

    #[inline]
    fn build_hasher(&self) -> AHasher {
        RandomState::with_seeds(
            0b10010101111011100000010011000100,
            0b00000011001001101011001001111000,
            0b11001111011010110111100010110101,
            0b00000100001111100011010011010101,
        )
        .build_hasher()
    }
}

/// A [`HashMap`][hashbrown::HashMap] implementing aHash, a high
/// speed keyed hashing algorithm intended for use in in-memory hashmaps.
///
/// aHash is designed for performance and is NOT cryptographically secure.
pub type HashMap<K, V> = hashbrown::HashMap<K, V, BuildHasherDefault<AHasher>>;

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
pub type HashSet<K> = hashbrown::HashSet<K, BuildHasherDefault<AHasher>>;

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

/// A no-op hash that only works on `u64`s. Will panic if attempting to
/// hash a type containing non-u64 fields.
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

/// A [`BuildHasher`] that results in a [`EntityHasher`].
#[derive(Default)]
pub struct EntityHash;

impl BuildHasher for EntityHash {
    type Hasher = EntityHasher;

    fn build_hasher(&self) -> Self::Hasher {
        EntityHasher::default()
    }
}

/// A very fast hash that is only designed to work on generational indices
/// like `Entity`. It will panic if attempting to hash a type containing
/// non-u64 fields.
#[derive(Debug, Default)]
pub struct EntityHasher {
    hash: u64,
}

// This value comes from rustc-hash (also known as FxHasher) which in turn got
// it from Firefox. It is something like `u64::MAX / N` for an N that gives a
// value close to π and works well for distributing bits for hashing when using
// with a wrapping multiplication.
const FRAC_U64MAX_PI: u64 = 0x517cc1b727220a95;

impl Hasher for EntityHasher {
    fn write(&mut self, _bytes: &[u8]) {
        panic!("can only hash u64 using EntityHasher");
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        // Apparently hashbrown's hashmap uses the upper 7 bits for some SIMD
        // optimisation that uses those bits for binning. This hash function
        // was faster than i | (i << (64 - 7)) in the worst cases, and was
        // faster than PassHasher for all cases tested.
        self.hash = i | (i.wrapping_mul(FRAC_U64MAX_PI) << 32);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing.
pub type EntityHashMap<K, V> = hashbrown::HashMap<K, V, EntityHash>;

/// A [`HashSet`] pre-configured to use [`EntityHash`] hashing.
pub type EntityHashSet<T> = hashbrown::HashSet<T, EntityHash>;

/// A type which calls a function when dropped.
/// This can be used to ensure that cleanup code is run even in case of a panic.
///
/// Note that this only works for panics that [unwind](https://doc.rust-lang.org/nomicon/unwinding.html)
/// -- any code within `OnDrop` will be skipped if a panic does not unwind.
/// In most cases, this will just work.
///
/// # Examples
///
/// ```
/// # use bevy_utils::OnDrop;
/// # fn test_panic(do_panic: bool, log: impl FnOnce(&str)) {
/// // This will print a message when the variable `_catch` gets dropped,
/// // even if a panic occurs before we reach the end of this scope.
/// // This is similar to a `try ... catch` block in languages such as C++.
/// let _catch = OnDrop::new(|| log("Oops, a panic occurred and this function didn't complete!"));
///
/// // Some code that may panic...
/// // ...
/// # if do_panic { panic!() }
///
/// // Make sure the message only gets printed if a panic occurs.
/// // If we remove this line, then the message will be printed regardless of whether a panic occurs
/// // -- similar to a `try ... finally` block.
/// std::mem::forget(_catch);
/// # }
/// #
/// # test_panic(false, |_| unreachable!());
/// # let mut did_log = false;
/// # std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
/// #   test_panic(true, |_| did_log = true);
/// # }));
/// # assert!(did_log);
/// ```
pub struct OnDrop<F: FnOnce()> {
    callback: ManuallyDrop<F>,
}

impl<F: FnOnce()> OnDrop<F> {
    /// Returns an object that will invoke the specified callback when dropped.
    pub fn new(callback: F) -> Self {
        Self {
            callback: ManuallyDrop::new(callback),
        }
    }
}

impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        // SAFETY: We may move out of `self`, since this instance can never be observed after it's dropped.
        let callback = unsafe { ManuallyDrop::take(&mut self.callback) };
        callback();
    }
}

/// Calls the [`tracing::info!`] macro on a value.
pub fn info<T: Debug>(data: T) {
    tracing::info!("{:?}", data);
}

/// Calls the [`tracing::debug!`] macro on a value.
pub fn dbg<T: Debug>(data: T) {
    tracing::debug!("{:?}", data);
}

/// Processes a [`Result`] by calling the [`tracing::warn!`] macro in case of an [`Err`] value.
pub fn warn<E: Debug>(result: Result<(), E>) {
    if let Err(warn) = result {
        tracing::warn!("{:?}", warn);
    }
}

/// Processes a [`Result`] by calling the [`tracing::error!`] macro in case of an [`Err`] value.
pub fn error<E: Debug>(result: Result<(), E>) {
    if let Err(error) = result {
        tracing::error!("{:?}", error);
    }
}

/// Like [`tracing::trace`], but conditional on cargo feature `detailed_trace`.
#[macro_export]
macro_rules! detailed_trace {
    ($($tts:tt)*) => {
        if cfg!(detailed_trace) {
            bevy_utils::tracing::trace!($($tts)*);
        }
    }
}
