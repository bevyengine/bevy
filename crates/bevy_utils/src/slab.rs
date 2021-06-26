use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::HashMap;

#[derive(Debug)]
pub struct SlabKey<V> {
    index: usize,
    marker: PhantomData<V>,
}

impl<V> Copy for SlabKey<V> {}

impl<V> Clone for SlabKey<V> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            marker: PhantomData,
        }
    }
}

impl<V> SlabKey<V> {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }
}

pub struct Slab<V> {
    values: Vec<Option<V>>,
    empty_indices: Vec<usize>,
}

impl<V> Default for Slab<V> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            empty_indices: Default::default(),
        }
    }
}

impl<V> Slab<V> {
    pub fn get(&self, key: SlabKey<V>) -> Option<&V> {
        self.values[key.index].as_ref()
    }

    pub fn get_mut(&mut self, key: SlabKey<V>) -> Option<&mut V> {
        self.values[key.index].as_mut()
    }

    pub fn add(&mut self, value: V) -> SlabKey<V> {
        let index = if let Some(index) = self.empty_indices.pop() {
            self.values[index] = Some(value);
            index
        } else {
            let index = self.values.len();
            self.values.push(Some(value));
            index
        };
        SlabKey {
            index,
            marker: PhantomData,
        }
    }

    pub fn remove(&mut self, key: SlabKey<V>) -> Option<V> {
        if let Some(value) = self.values[key.index].take() {
            self.empty_indices.push(key.index);
            Some(value)
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter().filter_map(|v| v.as_ref())
    }

    /// Retains any items matching the given predicate. Removed items will be dropped and their indices will be
    /// made available for future items.
    pub fn retain_in_place(&mut self, mut predicate: impl FnMut(&mut V) -> bool) {
        for (i, value) in self.values.iter_mut().enumerate() {
            if let Some(value) = value {
                if predicate(value) {
                    continue;
                }
            } else {
                continue;
            }

            *value = None;
            self.empty_indices.push(i);
        }
    }
}

impl<V> Index<SlabKey<V>> for Slab<V> {
    type Output = V;

    #[inline]
    fn index(&self, index: SlabKey<V>) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<V> IndexMut<SlabKey<V>> for Slab<V> {
    #[inline]
    fn index_mut(&mut self, index: SlabKey<V>) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

pub struct FrameSlabMapValue<K, V> {
    value: V,
    key: K,
    frames_since_last_use: usize,
}

pub struct FrameSlabMap<K, V> {
    slab: Slab<FrameSlabMapValue<K, V>>,
    keys: HashMap<K, FrameSlabMapKey<K, V>>,
}

impl<K, V> Default for FrameSlabMap<K, V> {
    fn default() -> Self {
        Self {
            slab: Default::default(),
            keys: Default::default(),
        }
    }
}

pub type FrameSlabMapKey<K, V> = SlabKey<FrameSlabMapValue<K, V>>;

impl<K: std::hash::Hash + Eq + Clone, V> FrameSlabMap<K, V> {
    pub fn get_value(&self, slab_key: FrameSlabMapKey<K, V>) -> Option<&V> {
        let value = self.slab.get(slab_key)?;
        Some(&value.value)
    }

    pub fn get_value_mut(&mut self, slab_key: FrameSlabMapKey<K, V>) -> Option<&mut V> {
        let value = self.slab.get_mut(slab_key)?;
        Some(&mut value.value)
    }

    pub fn get_or_insert_with(
        &mut self,
        key: K,
        f: impl FnOnce() -> V,
    ) -> SlabKey<FrameSlabMapValue<K, V>> {
        match self.keys.entry(key.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let slab_key = *entry.get();
                match self.slab.get_mut(slab_key) {
                    Some(value) => {
                        value.frames_since_last_use = 0;
                        slab_key
                    }
                    None => {
                        let key = self.slab.add(FrameSlabMapValue {
                            frames_since_last_use: 0,
                            value: f(),
                            key,
                        });
                        entry.insert(key);
                        key
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let key = self.slab.add(FrameSlabMapValue {
                    frames_since_last_use: 0,
                    value: f(),
                    key,
                });
                entry.insert(key);
                key
            }
        }
    }

    pub fn next_frame(&mut self) {
        let keys = &mut self.keys;
        self.slab.retain_in_place(|v| {
            v.frames_since_last_use += 1;
            if v.frames_since_last_use < 3 {
                true
            } else {
                keys.remove(&v.key);
                false
            }
        })
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Index<FrameSlabMapKey<K, V>> for FrameSlabMap<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, index: FrameSlabMapKey<K, V>) -> &Self::Output {
        self.get_value(index).unwrap()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> IndexMut<FrameSlabMapKey<K, V>> for FrameSlabMap<K, V> {
    #[inline]
    fn index_mut(&mut self, index: FrameSlabMapKey<K, V>) -> &mut Self::Output {
        self.get_value_mut(index).unwrap()
    }
}
