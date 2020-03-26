use super::{AssetSetBatcher2, AssetSetBatcherKey2, Batch, BatchKey2};
use crate::asset::{Handle, HandleId};
use legion::prelude::Entity;
use std::{any::TypeId, collections::HashMap};

pub trait AssetBatcher {
    fn set_entity_handle(&mut self, entity: Entity, handle_type: TypeId, handle_id: HandleId);
    fn get_batch2(&self, key: &BatchKey2) -> Option<&Batch>;
    // TODO: add pipeline handle here
    fn get_batches2(&self) -> std::collections::hash_map::Iter<'_, BatchKey2, Batch>;
    fn get_batches<'a>(&'a self) -> Box<dyn Iterator<Item = &Batch> + 'a>;
    fn get_batches_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut Batch> + 'a>;
}

#[derive(Default)]
pub struct AssetBatchers {
    asset_batchers: Vec<Box<dyn AssetBatcher + Send + Sync>>,
    asset_batcher_indices2: HashMap<AssetSetBatcherKey2, usize>,
    handle_batchers: HashMap<TypeId, Vec<usize>>,
}

impl AssetBatchers {
    pub fn set_entity_handle<T>(&mut self, entity: Entity, handle: Handle<T>)
    where
        T: 'static,
    {
        let handle_type = TypeId::of::<T>();
        if let Some(batcher_indices) = self.handle_batchers.get(&handle_type) {
            for index in batcher_indices.iter() {
                self.asset_batchers[*index].set_entity_handle(entity, handle_type, handle.id);
            }
        }
    }

    pub fn batch_types2<T1, T2>(&mut self)
    where
        T1: 'static,
        T2: 'static,
    {
        let key = AssetSetBatcherKey2 {
            handle1_type: TypeId::of::<T1>(),
            handle2_type: TypeId::of::<T2>(),
        };

        self.asset_batchers
            .push(Box::new(AssetSetBatcher2::new(key.clone())));

        let index = self.asset_batchers.len() - 1;

        let handle1_batchers = self
            .handle_batchers
            .entry(key.handle1_type.clone())
            .or_insert_with(|| Vec::new());
        handle1_batchers.push(index);

        let handle2_batchers = self
            .handle_batchers
            .entry(key.handle2_type.clone())
            .or_insert_with(|| Vec::new());
        handle2_batchers.push(index);

        self.asset_batcher_indices2.insert(key, index);
    }

    pub fn get_batches2<T1, T2>(
        &self,
    ) -> Option<std::collections::hash_map::Iter<'_, BatchKey2, Batch>>
    where
        T1: 'static,
        T2: 'static,
    {
        let key = AssetSetBatcherKey2 {
            handle1_type: TypeId::of::<T1>(),
            handle2_type: TypeId::of::<T2>(),
        };

        if let Some(index) = self.asset_batcher_indices2.get(&key) {
            Some(self.asset_batchers[*index].get_batches2())
        } else {
            None
        }
    }

    pub fn get_batch2<T1, T2>(&self, handle1: Handle<T1>, handle2: Handle<T2>) -> Option<&Batch>
    where
        T1: 'static,
        T2: 'static,
    {
        let key = AssetSetBatcherKey2 {
            handle1_type: TypeId::of::<T1>(),
            handle2_type: TypeId::of::<T2>(),
        };

        let batch_key = BatchKey2 {
            handle1: handle1.id,
            handle2: handle2.id,
        };

        if let Some(index) = self.asset_batcher_indices2.get(&key) {
            self.asset_batchers[*index].get_batch2(&batch_key)
        } else {
            None
        }
    }

    pub fn get_batches(&self) -> impl Iterator<Item = &Batch> {
        self.asset_batchers
            .iter()
            .map(|a| a.get_batches())
            .flatten()
    }

    pub fn get_handle_batches<T>(&self) -> Option<impl Iterator<Item = &Batch>>
    where
        T: 'static,
    {
        let handle_type = TypeId::of::<T>();
        if let Some(batcher_indices) = self.handle_batchers.get(&handle_type) {
            Some(
                // NOTE: it would be great to use batcher_indices.iter().map(|i| self.asset_batchers[*i].get_batches()) here
                // but unfortunately the lifetimes don't work out for some reason
                self.asset_batchers
                    .iter()
                    .enumerate()
                    .filter(move |(index, _a)| batcher_indices.contains(index))
                    .map(|(_index, a)| a.get_batches())
                    .flatten(),
            )
        } else {
            None
        }
    }

    pub fn get_handle_batches_mut<T>(&mut self) -> Option<impl Iterator<Item = &mut Batch>>
    where
        T: 'static,
    {
        let handle_type = TypeId::of::<T>();
        if let Some(batcher_indices) = self.handle_batchers.get(&handle_type) {
            Some(
                self.asset_batchers
                    .iter_mut()
                    .enumerate()
                    .filter(move |(index, _a)| batcher_indices.contains(index))
                    .map(|(_index, a)| a.get_batches_mut())
                    .flatten(),
            )
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use legion::prelude::*;
    struct A;
    struct B;
    struct C;

    #[test]
    fn test_batching() {
        let mut asset_batchers = AssetBatchers::default();
        asset_batchers.batch_types2::<A, B>();

        let mut world = World::new();
        let a1: Handle<A> = Handle::new(1);
        let b1: Handle<B> = Handle::new(1);
        let c1: Handle<C> = Handle::new(1);

        let a2: Handle<A> = Handle::new(2);
        let b2: Handle<B> = Handle::new(2);

        let entities = world.insert((), (0..3).map(|_| ()));
        asset_batchers.set_entity_handle(entities[0], a1);
        // batch is empty when Handle<B> is missing
        assert_eq!(asset_batchers.get_batch2(a1, b1), None);
        asset_batchers.set_entity_handle(entities[0], b1);
        // entity[0] is added to batch when it has both Handle<A> and Handle<B>
        let mut expected_batch = Batch {
            handles: vec![a1.into(), b1.into()],
            ..Default::default()
        };
        expected_batch.add_entity(entities[0]);
        assert_eq!(asset_batchers.get_batch2(a1, b1).unwrap(), &expected_batch);
        asset_batchers.set_entity_handle(entities[0], c1);

        asset_batchers.set_entity_handle(entities[1], a1);
        asset_batchers.set_entity_handle(entities[1], b1);

        // all entities with Handle<A> and Handle<B> are returned
        let mut expected_batch = Batch {
            handles: vec![a1.into(), b1.into()],
            ..Default::default()
        };
        expected_batch.add_entity(entities[0]);
        expected_batch.add_entity(entities[1]);
        assert_eq!(asset_batchers.get_batch2(a1, b1).unwrap(), &expected_batch);

        // uncreated batches are empty
        assert_eq!(asset_batchers.get_batch2(a1, c1), None);

        // batch iteration works
        asset_batchers.set_entity_handle(entities[2], a2);
        asset_batchers.set_entity_handle(entities[2], b2);

        let mut batches = asset_batchers
            .get_batches2::<A, B>()
            .unwrap()
            .collect::<Vec<(&BatchKey2, &Batch)>>();

        batches.sort_by(|a, b| a.0.cmp(b.0));
        let mut expected_batch1 = Batch {
            handles: vec![a1.into(), b1.into()],
            ..Default::default()
        };
        expected_batch1.add_entity(entities[0]);
        expected_batch1.add_entity(entities[1]);
        let mut expected_batch2 = Batch {
            handles: vec![a2.into(), b2.into()],
            ..Default::default()
        };
        expected_batch2.add_entity(entities[2]);
        let mut expected_batches = vec![
            (
                BatchKey2 {
                    handle1: a1.id,
                    handle2: b1.id,
                },
                expected_batch1,
            ),
            (
                BatchKey2 {
                    handle1: a2.id,
                    handle2: b2.id,
                },
                expected_batch2,
            ),
        ];
        expected_batches.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            batches,
            expected_batches
                .iter()
                .map(|(a, b)| (a, b))
                .collect::<Vec<(&BatchKey2, &Batch)>>()
        );
    }
}
