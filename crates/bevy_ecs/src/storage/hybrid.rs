use crate::query::DebugCheckedUnwrap;
use bevy_platform::collections::HashMap;
use bevy_platform::prelude::Vec;

use super::SparseSetIndex;

// Hybrid datastructures that can store both sparse and dense indices efficiently.
//
// This is done quite simply by having the first N indices be stored in a `Vec` or array, and for any indices larger than N, a sparse datastructure is used (like a HashMap).

#[derive(Debug)]
pub struct HybridMap<K, V>
where
    K: SparseSetIndex,
{
    cutoff: usize,
    vec: Vec<Option<V>>,
    hashmap: HashMap<K, V>,
}

impl<K, V> Default for HybridMap<K, V>
where
    K: SparseSetIndex,
{
    fn default() -> Self {
        HybridMap {
            cutoff: 128,
            vec: Vec::new(),
            hashmap: HashMap::new(),
        }
    }
}

impl<K, V> HybridMap<K, V>
where
    K: SparseSetIndex,
{
    pub fn contains_key(&self, key: &K) -> bool {
        let index = key.sparse_set_index();
        if index < self.cutoff {
            self.vec.get(index).is_some_and(Option::is_some)
        } else {
            self.contains_key(key)
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let index = key.sparse_set_index();
        if index < self.cutoff {
            self.vec.get(index).and_then(|val| val.as_ref())
        } else {
            self.hashmap.get(key)
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = key.sparse_set_index();
        if index < self.cutoff {
            self.vec.get_mut(index).and_then(|val| val.as_mut())
        } else {
            self.hashmap.get_mut(key)
        }
    }

    pub unsafe fn insert_unique_unchecked(&mut self, key: K, value: V) {
        let index = key.sparse_set_index();
        if index < self.cutoff {
            let least_len = index + 1;
            if self.vec.len() < least_len {
                self.vec.resize_with(least_len, || None);
            }
            // SAFETY: We just extended the vec to make this index valid
            let slot = unsafe { self.vec.get_mut(index).debug_checked_unwrap() };
            // Caller ensures id is unique
            debug_assert!(slot.is_none());
            *slot = Some(value);
        } else {
            // SAFETY: Caller ensures id is unique
            unsafe {
                self.hashmap.insert_unique_unchecked(key, value);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len() + self.hashmap.len()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.vec
            .iter()
            .filter_map(Option::as_ref)
            .chain(self.hashmap.values())
    }
}
