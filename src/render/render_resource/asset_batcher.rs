use super::RenderResourceAssignments;
use crate::asset::{Handle, HandleId, HandleUntyped};
use legion::prelude::Entity;
use std::{any::TypeId, collections::HashMap, hash::Hash};

// TODO: if/when const generics land, revisit this design

#[derive(Hash, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub struct BatchKey2 {
    pub handle1: HandleId,
    pub handle2: HandleId,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct AssetSetBatcherKey2 {
    handle1_type: TypeId,
    handle2_type: TypeId,
}

struct EntitySetState2 {
    handle1: Option<HandleId>,
    handle2: Option<HandleId>,
}

impl EntitySetState2 {
    fn is_full(&self) -> bool {
        self.handle1.is_some() && self.handle2.is_some()
    }
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct Batch {
    pub handles: Vec<HandleUntyped>,
    pub entity_indices: HashMap<Entity, usize>,
    pub current_index: usize,
    pub render_resource_assignments: Option<RenderResourceAssignments>,
}

impl Batch {
    pub fn add_entity(&mut self, entity: Entity) {
        if let None = self.entity_indices.get(&entity) {
            self.entity_indices.insert(entity, self.current_index);
            self.current_index += 1;
        }
    }
}

pub struct AssetSetBatcher2 {
    key: AssetSetBatcherKey2,
    set_batches: HashMap<BatchKey2, Batch>,
    entity_set_states: HashMap<Entity, EntitySetState2>,
}

impl AssetSetBatcher2 {
    fn new(key: AssetSetBatcherKey2) -> Self {
        AssetSetBatcher2 {
            key,
            set_batches: HashMap::new(),
            entity_set_states: HashMap::new(),
        }
    }

    fn add_entity_to_set(&mut self, entity: Entity) {
        // these unwraps are safe because this function is only called from set_entity_handle on a "full" state
        let state = self.entity_set_states.get(&entity).unwrap();
        let key = BatchKey2 {
            handle1: state.handle1.unwrap(),
            handle2: state.handle2.unwrap(),
        };

        match self.set_batches.get_mut(&key) {
            Some(batch) => {
                batch.add_entity(entity);
            }
            None => {
                let mut batch = Batch::default();

                batch.handles.push(HandleUntyped {
                    id: key.handle1,
                    type_id: self.key.handle1_type,
                });
                batch.handles.push(HandleUntyped {
                    id: key.handle2,
                    type_id: self.key.handle2_type,
                });

                batch.add_entity(entity);
                self.set_batches.insert(key, batch);
            }
        }
    }

    pub fn set_entity_handle1(&mut self, entity: Entity, handle_id: HandleId) {
        match self.entity_set_states.get_mut(&entity) {
            None => {
                // TODO: when generalizing to set size 1, ensure you treat set as "full" here
                self.entity_set_states.insert(
                    entity,
                    EntitySetState2 {
                        handle1: Some(handle_id),
                        handle2: None,
                    },
                );
            }
            Some(state) => {
                state.handle1 = Some(handle_id);
                if state.is_full() {
                    self.add_entity_to_set(entity);
                }
            }
        }
    }

    pub fn set_entity_handle2(&mut self, entity: Entity, handle_id: HandleId) {
        match self.entity_set_states.get_mut(&entity) {
            None => {
                // TODO: when generalizing to set size 1, ensure you treat set as "full" here
                self.entity_set_states.insert(
                    entity,
                    EntitySetState2 {
                        handle1: None,
                        handle2: Some(handle_id),
                    },
                );
            }
            Some(state) => {
                state.handle2 = Some(handle_id);
                if state.is_full() {
                    self.add_entity_to_set(entity);
                }
            }
        }
    }
}

impl AssetBatcher for AssetSetBatcher2 {
    fn set_entity_handle(&mut self, entity: Entity, handle_type: TypeId, handle_id: HandleId) {
        if handle_type == self.key.handle1_type {
            self.set_entity_handle1(entity, handle_id);
        } else if handle_type == self.key.handle2_type {
            self.set_entity_handle2(entity, handle_id);
        }
    }
    fn get_batch2(&self, key: &BatchKey2) -> Option<&Batch> {
        self.set_batches.get(key)
    }

    fn get_batches2(&self) -> std::collections::hash_map::Iter<'_, BatchKey2, Batch> {
        self.set_batches.iter()
    }

    fn get_batches<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Batch> + 'a> {
        Box::new(self.set_batches.values())
    }

    fn get_batches_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Batch> + 'a> {
        Box::new(self.set_batches.values_mut())
    }
}

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

    pub fn get_batcher_indices<T>(&self) -> impl Iterator<Item = &usize>
    where
        T: 'static,
    {
        let handle_type = TypeId::of::<T>();
        self.handle_batchers
            .get(&handle_type)
            .unwrap()
            .iter()
    }

    pub fn get_batches_from_batcher(&self, index: usize) -> impl Iterator<Item = &Batch>
    {
        self.asset_batchers[index].get_batches()
    }

    pub fn get_batches_from_batcher_mut(&mut self, index: usize) -> impl Iterator<Item = &mut Batch>
    {
        self.asset_batchers[index].get_batches_mut()
    }

    // pub fn get_handle_batches<T>(&self) -> Option<impl Iterator<Item = &Batch>>
    // where
    //     T: 'static,
    // {
    //     let handle_type = TypeId::of::<T>();
    //     if let Some(batcher_indices) = self.handle_batchers.get(&handle_type) {
    //         Some(
    //             self.asset_batchers
    //                 .iter()
    //                 .enumerate()
    //                 .filter(|(index, a)| {
    //                     let handle_type = TypeId::of::<T>();
    //                     self.handle_batchers
    //                         .get(&handle_type)
    //                         .unwrap()
    //                         .contains(index)
    //                 })
    //                 .map(|(index, a)| a.get_batches())
    //                 .flatten(),
    //         )
    //     } else {
    //         None
    //     }
    // }
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
