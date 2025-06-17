use core::{any::TypeId, hash::Hash};

use bevy_platform::{
    collections::{hash_map::Entry, HashMap},
    hash::{Hashed, NoOpHash, PassHash},
};

/// A [`HashMap`] pre-configured to use [`Hashed`] keys and [`PassHash`] passthrough hashing.
/// Iteration order only depends on the order of insertions and deletions.
pub type PreHashMap<K, V> = HashMap<Hashed<K>, V, PassHash>;

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
pub type TypeIdMap<V> = HashMap<TypeId, V, NoOpHash>;

/// Extension trait to make use of [`TypeIdMap`] more ergonomic.
///
/// Each function on this trait is a trivial wrapper for a function
/// on [`HashMap`], replacing a `TypeId` key with a
/// generic parameter `T`.
///
/// # Examples
///
/// ```rust
/// # use std::any::TypeId;
/// # use bevy_utils::TypeIdMap;
/// use bevy_utils::TypeIdMapExt;
///
/// struct MyType;
///
/// // Using the built-in `HashMap` functions requires manually looking up `TypeId`s.
/// let mut map = TypeIdMap::default();
/// map.insert(TypeId::of::<MyType>(), 7);
/// assert_eq!(map.get(&TypeId::of::<MyType>()), Some(&7));
///
/// // Using `TypeIdMapExt` functions does the lookup for you.
/// map.insert_type::<MyType>(7);
/// assert_eq!(map.get_type::<MyType>(), Some(&7));
/// ```
pub trait TypeIdMapExt<V> {
    /// Inserts a value for the type `T`.
    ///
    /// If the map did not previously contain this key then [`None`] is returned,
    /// otherwise the value for this key is updated and the old value returned.
    fn insert_type<T: ?Sized + 'static>(&mut self, v: V) -> Option<V>;

    /// Returns a reference to the value for type `T`, if one exists.
    fn get_type<T: ?Sized + 'static>(&self) -> Option<&V>;

    /// Returns a mutable reference to the value for type `T`, if one exists.
    fn get_type_mut<T: ?Sized + 'static>(&mut self) -> Option<&mut V>;

    /// Removes type `T` from the map, returning the value for this
    /// key if it was previously present.
    fn remove_type<T: ?Sized + 'static>(&mut self) -> Option<V>;

    /// Gets the type `T`'s entry in the map for in-place manipulation.
    fn entry_type<T: ?Sized + 'static>(&mut self) -> Entry<'_, TypeId, V, NoOpHash>;
}

impl<V> TypeIdMapExt<V> for TypeIdMap<V> {
    #[inline]
    fn insert_type<T: ?Sized + 'static>(&mut self, v: V) -> Option<V> {
        self.insert(TypeId::of::<T>(), v)
    }

    #[inline]
    fn get_type<T: ?Sized + 'static>(&self) -> Option<&V> {
        self.get(&TypeId::of::<T>())
    }

    #[inline]
    fn get_type_mut<T: ?Sized + 'static>(&mut self) -> Option<&mut V> {
        self.get_mut(&TypeId::of::<T>())
    }

    #[inline]
    fn remove_type<T: ?Sized + 'static>(&mut self) -> Option<V> {
        self.remove(&TypeId::of::<T>())
    }

    #[inline]
    fn entry_type<T: ?Sized + 'static>(&mut self) -> Entry<'_, TypeId, V, NoOpHash> {
        self.entry(TypeId::of::<T>())
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

    crate::cfg::alloc! {
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
}