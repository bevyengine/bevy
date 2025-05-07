#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]
#![no_std]

//! General utilities for first-party [Bevy] engine crates.
//!
//! [Bevy]: https://bevyengine.org/

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

/// The utilities prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::default;
}

pub mod synccell;
pub mod syncunsafecell;

mod default;
mod once;
#[cfg(feature = "std")]
mod parallel_queue;

#[doc(hidden)]
pub use once::OnceFlag;

pub use default::default;

#[cfg(feature = "std")]
pub use parallel_queue::*;

use core::mem::ManuallyDrop;

#[cfg(feature = "alloc")]
use {
    bevy_platform::{
        collections::HashMap,
        hash::{Hashed, NoOpHash, PassHash},
    },
    core::{any::TypeId, hash::Hash},
};

/// A [`HashMap`] pre-configured to use [`Hashed`] keys and [`PassHash`] passthrough hashing.
/// Iteration order only depends on the order of insertions and deletions.
#[cfg(feature = "alloc")]
pub type PreHashMap<K, V> = HashMap<Hashed<K>, V, PassHash>;

/// Extension methods intended to add functionality to [`PreHashMap`].
#[cfg(feature = "alloc")]
pub trait PreHashMapExt<K, V> {
    /// Tries to get or insert the value for the given `key` using the pre-computed hash first.
    /// If the [`PreHashMap`] does not already contain the `key`, it will clone it and insert
    /// the value returned by `func`.
    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: &Hashed<K>, func: F) -> &mut V;
}

#[cfg(feature = "alloc")]
impl<K: Hash + Eq + PartialEq + Clone, V> PreHashMapExt<K, V> for PreHashMap<K, V> {
    #[inline]
    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: &Hashed<K>, func: F) -> &mut V {
        use bevy_platform::collections::hash_map::RawEntryMut;
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

/// A specialized hashmap type with Key of [`TypeId`]
/// Iteration order only depends on the order of insertions and deletions.
#[cfg(feature = "alloc")]
pub type TypeIdMap<V> = HashMap<TypeId, V, NoOpHash>;

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
/// core::mem::forget(_catch);
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
        #![expect(
            unsafe_code,
            reason = "Taking from a ManuallyDrop requires unsafe code."
        )]
        // SAFETY: We may move out of `self`, since this instance can never be observed after it's dropped.
        let callback = unsafe { ManuallyDrop::take(&mut self.callback) };
        callback();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Check that the HashMaps are Clone if the key/values are Clone
    assert_impl_all!(PreHashMap::<u64, usize>: Clone);

    #[test]
    fn fast_typeid_hash() {
        struct Hasher;

        impl core::hash::Hasher for Hasher {
            fn finish(&self) -> u64 {
                0
            }
            fn write(&mut self, _: &[u8]) {
                panic!("Hashing of core::any::TypeId changed");
            }
            fn write_u64(&mut self, _: u64) {}
        }

        Hash::hash(&TypeId::of::<()>(), &mut Hasher);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn stable_hash_within_same_program_execution() {
        use alloc::vec::Vec;

        let mut map_1 = <HashMap<_, _>>::default();
        let mut map_2 = <HashMap<_, _>>::default();
        for i in 1..10 {
            map_1.insert(i, i);
            map_2.insert(i, i);
        }
        assert_eq!(
            map_1.iter().collect::<Vec<_>>(),
            map_2.iter().collect::<Vec<_>>()
        );
    }
}
