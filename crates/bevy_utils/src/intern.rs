use std::{any::TypeId, hash::Hash};

use parking_lot::{RwLock, RwLockReadGuard};

use crate::{FixedState, StableHashMap};

type IndexSet<T> = indexmap::IndexSet<T, FixedState>;

/// A data structure used to intern a set of values of a specific type.
/// To store multiple distinct types, or generic types, try [`Labels`].
pub struct Interner<T: Clone + Hash + Eq>(
    // The `IndexSet` is a hash set that preserves ordering as long as
    // you don't remove items (which we don't).
    // This allows us to have O(~1) hashing and map each entry to a stable index.
    RwLock<IndexSet<T>>,
);

/// The type returned from [`Labels::get`](Labels#method.get).
///
/// Will hold a lock on the label interner until this value gets dropped.
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

    /// Gets a reference to the label with specified index.
    pub fn get(&self, idx: usize) -> Option<InternGuard<T>> {
        RwLockReadGuard::try_map(self.0.read(), |set| set.get_index(idx)).ok()
    }
}

struct TypeMap(StableHashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>);

impl TypeMap {
    pub const fn new() -> Self {
        Self(StableHashMap::with_hasher(FixedState))
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<impl Drop> {
        self.0.insert(TypeId::of::<T>(), Box::new(val))
    }
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let val = self.0.get(&TypeId::of::<T>())?.as_ref();
        // SAFETY: `val` was keyed with the TypeId of `T`, so we can cast it to `T`.
        Some(unsafe { &*(val as *const _ as *const T) })
    }
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let val = self.0.get_mut(&TypeId::of::<T>())?.as_mut();
        // SAFETY: `val` was keyed with the TypeId of `T`, so we can cast it to `T`.
        Some(unsafe { &mut *(val as *mut _ as *mut T) })
    }
}

/// Data structure used to intern a set of values of any given type.
///
/// If you just need to store a single concrete type, [`Interner`] is more efficient.
pub struct AnyInterner(
    // This type-map stores instances of `IndexSet<T>`, for any `T`.
    RwLock<TypeMap>,
);

impl AnyInterner {
    pub const fn new() -> Self {
        Self(RwLock::new(TypeMap::new()))
    }

    /// Interns a value, if it was not already interned in this set.
    ///
    /// Returns an integer used to refer to the value later on.
    pub fn intern<L>(&self, val: &L) -> usize
    where
        L: Clone + Hash + Eq + Send + Sync + 'static,
    {
        use parking_lot::RwLockUpgradableReadGuard as Guard;

        // Acquire an upgradeable read lock, since we might not have to do any writing.
        let type_map = self.0.upgradable_read();

        if let Some(set) = type_map.get::<IndexSet<L>>() {
            // If the value is already interned, return its index.
            if let Some(idx) = set.get_index_of(val) {
                return idx;
            }

            // Get mutable access to the interner.
            let mut type_map = Guard::upgrade(type_map);
            let set = type_map.get_mut::<IndexSet<L>>().unwrap();

            // Insert a clone of the value and return its index.
            let (idx, _) = set.insert_full(val.clone());
            idx
        } else {
            let mut type_map = Guard::upgrade(type_map);

            // Initialize the `L` interner for the first time, including `val` in it.
            let mut set = IndexSet::default();
            let (idx, _) = set.insert_full(val.clone());
            let old = type_map.insert(set);
            // We already checked that there is no set for type `L`,
            // so let's avoid generating useless drop code for the "previous" entry.
            std::mem::forget(old);
            idx
        }
    }

    /// Gets a reference to the label with specified index.
    pub fn get<L: 'static>(&self, key: usize) -> Option<InternGuard<L>> {
        RwLockReadGuard::try_map(self.0.read(), |type_map| {
            type_map.get::<IndexSet<L>>()?.get_index(key)
        })
        .ok()
    }
}
