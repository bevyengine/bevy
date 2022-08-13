use std::hash::Hash;

use parking_lot::{RwLock, RwLockReadGuard};

use crate::FixedState;

type IndexSet<T> = indexmap::IndexSet<T, FixedState>;

/// A data structure used to intern a set of values of a specific type.
/// To store multiple distinct types, or generic types, try [`AnyInterner`].
pub struct Interner<T: Clone + Hash + Eq>(
    // The `IndexSet` is a hash set that preserves ordering as long as
    // you don't remove items (which we don't).
    // This allows us to have O(~1) hashing and map each entry to a stable index.
    RwLock<IndexSet<T>>,
);

/// The type returned from [`Interner::get`](Interner#method.get).
///
/// Will hold a lock on the interner until this guard gets dropped.
pub type InternGuard<'a, L> = parking_lot::MappedRwLockReadGuard<'a, L>;

impl<T: Clone + Hash + Eq> Interner<T> {
    pub const fn new() -> Self {
        Self(RwLock::new(IndexSet::with_hasher(FixedState)))
    }

    /// Interns a value, if it was not already interned in this set.
    ///
    /// Returns an integer used to refer to the value later on.
    pub fn intern(&self, val: &T) -> usize {
        use parking_lot::RwLockUpgradableReadGuard as Guard;

        // Acquire an upgradeable read lock, since we might not have to do any writing.
        let set = self.0.upgradable_read();

        // If the value is already interned, return its index.
        if let Some(idx) = set.get_index_of(val) {
            return idx;
        }

        // Upgrade to a mutable lock.
        let mut set = Guard::upgrade(set);
        let (idx, _) = set.insert_full(val.clone());
        idx
    }

    /// Gets a reference to the value with specified index.
    pub fn get(&self, idx: usize) -> Option<InternGuard<T>> {
        RwLockReadGuard::try_map(self.0.read(), |set| set.get_index(idx)).ok()
    }
}
