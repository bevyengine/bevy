use super::Batch;
use bevy_utils::HashMap;
use smallvec::{smallvec, SmallVec};
use std::{borrow::Cow, fmt, hash::Hash};

// TODO: add sorting by primary / secondary handle to reduce rebinds of data

// TValue: entityid
// TKey: handleuntyped

pub trait Key: Clone + Eq + Hash + 'static {}
impl<T: Clone + Eq + Hash + 'static> Key for T {}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BatchKey<TKey: Key>(pub Cow<'static, SmallVec<[TKey; 2]>>);

impl<TKey: Key> BatchKey<TKey> {
    pub fn key1(key: TKey) -> Self {
        BatchKey(Cow::Owned(smallvec![key]))
    }

    pub fn key2(key1: TKey, key2: TKey) -> Self {
        BatchKey(Cow::Owned(smallvec![key1, key2]))
    }

    pub fn key3(key1: TKey, key2: TKey, key3: TKey) -> Self {
        BatchKey(Cow::Owned(smallvec![key1, key2, key3]))
    }
}

#[derive(Debug)]
pub struct BatcherKeyState<TKey: Key> {
    batch_key: Option<BatchKey<TKey>>,
    keys: SmallVec<[Option<TKey>; 2]>,
}

impl<TKey: Key> BatcherKeyState<TKey> {
    pub fn new(size: usize) -> Self {
        BatcherKeyState {
            keys: smallvec![None; size],
            batch_key: None,
        }
    }

    pub fn set(&mut self, index: usize, key: TKey) {
        self.keys[index] = Some(key);
    }

    pub fn finish(&mut self) -> Option<BatchKey<TKey>> {
        let finished = self.keys.iter().filter(|x| x.is_some()).count() == self.keys.len();
        if finished {
            let batch_key = BatchKey(Cow::Owned(
                self.keys
                    .drain(..)
                    .map(|k| k.unwrap())
                    .collect::<SmallVec<[TKey; 2]>>(),
            ));
            self.batch_key = Some(batch_key);
            self.batch_key.clone()
        } else {
            None
        }
    }
}

/// An unordered batcher intended to support an arbitrary number of keys of the same type (but with some distinguishing factor)
/// NOTE: this may or may not be useful for anything. when paired with a higher-level "BatcherSet" it would allow updating batches
// per-key (ex: material, mesh) with no global knowledge of the number of batch types (ex: (Mesh), (Material, Mesh)) that key belongs
// to. The downside is that it is completely unordered, so it probably isn't useful for front->back or back->front rendering. But
// _maybe_ for gpu instancing?
pub struct Batcher<TKey, TValue, TData>
where
    TKey: Key,
{
    pub batches: HashMap<BatchKey<TKey>, Batch<TKey, TValue, TData>>,
    pub is_index: Vec<fn(&TKey) -> bool>,
    pub key_states: HashMap<TValue, BatcherKeyState<TKey>>,
    pub key_count: usize,
}

impl<TKey: Key, TValue, TData> fmt::Debug for Batcher<TKey, TValue, TData>
where
    TKey: Key + fmt::Debug,
    TValue: fmt::Debug,
    TData: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let is_index = self
            .is_index
            .iter()
            .map(|f| f as *const for<'r> fn(&'r TKey) -> bool)
            .collect::<Vec<_>>();

        f.debug_struct("Batcher")
            .field("batches", &self.batches)
            .field("is_index", &is_index)
            .field("key_states", &self.key_states)
            .field("key_count", &self.key_count)
            .finish()
    }
}

impl<TKey, TValue, TData> Batcher<TKey, TValue, TData>
where
    TKey: Key,
    TValue: Clone + Eq + Hash,
    TData: Default,
{
    pub fn new(is_index: Vec<fn(&TKey) -> bool>) -> Self {
        Batcher {
            batches: HashMap::default(),
            key_states: HashMap::default(),
            key_count: is_index.len(),
            is_index,
        }
    }

    pub fn get_batch(&self, batch_key: &BatchKey<TKey>) -> Option<&Batch<TKey, TValue, TData>> {
        self.batches.get(batch_key)
    }

    pub fn get_batch_mut(
        &mut self,
        batch_key: &BatchKey<TKey>,
    ) -> Option<&mut Batch<TKey, TValue, TData>> {
        self.batches.get_mut(batch_key)
    }

    pub fn add(&mut self, key: TKey, value: TValue) -> bool {
        let batch_key = {
            let key_count = self.key_count;
            let key_state = self
                .key_states
                .entry(value.clone())
                .or_insert_with(|| BatcherKeyState::new(key_count));

            // if all key states are set, the value is already in the batch
            if key_state.batch_key.is_some() {
                // TODO: if weights are ever added, make sure to get the batch and set the weight here
                return true;
            }

            let key_index = self
                .is_index
                .iter()
                .enumerate()
                .find(|(_i, is_index)| is_index(&key))
                .map(|(i, _)| i);
            if let Some(key_index) = key_index {
                key_state.set(key_index, key);
                key_state.finish()
            } else {
                return false;
            }
        };

        if let Some(batch_key) = batch_key {
            let batch = self
                .batches
                .entry(batch_key.clone())
                .or_insert_with(|| Batch::new(batch_key, TData::default()));

            batch.add(value);
        }

        true
    }

    pub fn iter(&self) -> impl Iterator<Item = &Batch<TKey, TValue, TData>> {
        self.batches.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Batch<TKey, TValue, TData>> {
        self.batches.values_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::{Batch, BatchKey, Batcher};
    use bevy_asset::{Handle, HandleUntyped};

    #[derive(Debug, Eq, PartialEq)]
    struct A;
    #[derive(Debug, Eq, PartialEq)]
    struct B;
    #[derive(Debug, Eq, PartialEq)]
    struct C;
    #[derive(Debug, Eq, PartialEq, Default)]
    struct Data;
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    struct Entity(usize);
    #[test]
    fn test_batcher_2() {
        let mut batcher: Batcher<HandleUntyped, Entity, Data> = Batcher::new(vec![
            HandleUntyped::is_handle::<A>,
            HandleUntyped::is_handle::<B>,
        ]);

        let e1 = Entity(1);
        let e2 = Entity(2);
        let e3 = Entity(3);

        let a1: HandleUntyped = Handle::<A>::new().into();
        let b1: HandleUntyped = Handle::<B>::new().into();
        let c1: HandleUntyped = Handle::<C>::new().into();

        let a2: HandleUntyped = Handle::<A>::new().into();
        let b2: HandleUntyped = Handle::<B>::new().into();

        let a1_b1 = BatchKey::key2(a1, b1);
        let a2_b2 = BatchKey::key2(a2, b2);

        assert_eq!(
            batcher.get_batch(&a1_b1),
            None,
            "a1_b1 batch should not exist yet"
        );
        batcher.add(a1, e1);
        assert_eq!(
            batcher.get_batch(&a1_b1),
            None,
            "a1_b1 batch should not exist yet"
        );
        batcher.add(b1, e1);

        let a1_b1_batch = Batch {
            batch_key: a1_b1.clone(),
            values: vec![e1],
            data: Data,
        };

        assert_eq!(
            batcher.get_batch(&a1_b1),
            Some(&a1_b1_batch),
            "a1_b1 batch should exist"
        );

        assert_eq!(
            batcher.get_batch(&a2_b2),
            None,
            "a2_b2 batch should not exist yet"
        );
        batcher.add(a2, e2);
        assert_eq!(
            batcher.get_batch(&a2_b2),
            None,
            "a2_b2 batch should not exist yet"
        );
        batcher.add(b2, e2);

        let expected_batch = Batch {
            batch_key: a2_b2.clone(),
            values: vec![e2],
            data: Data,
        };

        assert_eq!(
            batcher.get_batch(&a2_b2),
            Some(&expected_batch),
            "a2_b2 batch should have e2"
        );

        batcher.add(a2, e3);
        batcher.add(b2, e3);
        batcher.add(c1, e3); // this should be ignored
        let a2_b2_batch = Batch {
            batch_key: a2_b2.clone(),
            values: vec![e2, e3],
            data: Data,
        };

        assert_eq!(
            batcher.get_batch(&a2_b2),
            Some(&a2_b2_batch),
            "a2_b2 batch should have e2 and e3"
        );

        let mut found_a1_b1 = false;
        let mut found_a2_b2 = false;
        for batch in batcher.iter() {
            if batch == &a1_b1_batch {
                found_a1_b1 = true;
            } else if batch == &a2_b2_batch {
                found_a2_b2 = true;
            }
        }

        assert!(found_a1_b1 && found_a2_b2);
        assert_eq!(batcher.iter().count(), 2);
    }
}
