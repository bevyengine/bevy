use super::RenderResource;
use crate::asset::{Handle, HandleId};
use legion::prelude::Entity;
use std::{any::TypeId, collections::HashMap, hash::Hash};

// TODO: if/when const generics land, revisit this design

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct BatchKey2 {
    handle1: HandleId,
    handle2: HandleId,
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

#[derive(Hash, PartialEq, Debug)]
pub struct Batch2 {
    pub entities: Vec<Entity>,
    pub buffer1: Option<RenderResource>,
    pub buffer2: Option<RenderResource>,
}

pub struct AssetSetBatcher2 {
    key: AssetSetBatcherKey2,
    set_batches: HashMap<BatchKey2, Batch2>,
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
            Some(instance_set) => {
                instance_set.entities.push(entity);
            }
            None => {
                self.set_batches.insert(
                    key,
                    Batch2 {
                        entities: vec![entity],
                        buffer1: None,
                        buffer2: None,
                    },
                );
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
    fn get_batch2(&self, key: &BatchKey2) -> Option<&Batch2> {
        self.set_batches.get(key)
    }

    fn get_batches2(&self) -> std::collections::hash_map::Iter<'_, BatchKey2, Batch2> {
        self.set_batches.iter()
    }
}

pub trait AssetBatcher {
    fn set_entity_handle(&mut self, entity: Entity, handle_type: TypeId, handle_id: HandleId);
    fn get_batch2(&self, key: &BatchKey2) -> Option<&Batch2>;
    fn get_batches2(&self) -> std::collections::hash_map::Iter<'_, BatchKey2, Batch2>;
}

#[derive(Default)]
pub struct AssetBatchers {
    asset_batchers: Vec<Box<dyn AssetBatcher + Send + Sync>>,
    asset_batcher_indices2: HashMap<AssetSetBatcherKey2, usize>,
}

impl AssetBatchers {
    pub fn set_entity_handle<T>(&mut self, entity: Entity, handle: Handle<T>)
    where
        T: 'static,
    {
        let handle_type = TypeId::of::<T>();
        for asset_batcher in self.asset_batchers.iter_mut() {
            asset_batcher.set_entity_handle(entity, handle_type, handle.id);
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

        self.asset_batcher_indices2
            .insert(key, self.asset_batchers.len() - 1);
    }

    pub fn get_batches2<T1, T2>(&mut self) -> Option<std::collections::hash_map::Iter<'_, BatchKey2, Batch2>>
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

    pub fn get_batch2<T1, T2>(
        &mut self,
        handle1: Handle<T1>,
        handle2: Handle<T2>,
    ) -> Option<&Batch2>
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
        assert_eq!(
            asset_batchers.get_batch2(a1, b1).unwrap(),
            &Batch2 {
                entities: vec![entities[0],],
                buffer1: None,
                buffer2: None,
            }
        );
        asset_batchers.set_entity_handle(entities[0], c1);

        asset_batchers.set_entity_handle(entities[1], a1);
        asset_batchers.set_entity_handle(entities[1], b1);

        // all entities with Handle<A> and Handle<B> are returned
        assert_eq!(
            asset_batchers.get_batch2(a1, b1).unwrap(),
            &Batch2 {
                entities: vec![entities[0], entities[1],],
                buffer1: None,
                buffer2: None,
            }
        );

        // uncreated batches are empty
        assert_eq!(asset_batchers.get_batch2(a1, c1), None);

        // batch iteration works
        asset_batchers.set_entity_handle(entities[2], a2);
        asset_batchers.set_entity_handle(entities[2], b2);
        assert_eq!(
            asset_batchers
                .get_batches2::<A, B>()
                .unwrap()
                .collect::<Vec<(&BatchKey2, &Batch2)>>(),
            vec![(
                &BatchKey2 {
                    handle1: a1.id,
                    handle2: b1.id,
                },
                &Batch2 {
                    buffer1: None,
                    buffer2: None,
                    entities: vec![entities[0], entities[1]]
                }
            ),(
                &BatchKey2 {
                    handle1: a2.id,
                    handle2: b2.id,
                },
                &Batch2 {
                    buffer1: None,
                    buffer2: None,
                    entities: vec![entities[2]]
                }
            )]
        );
    }
}
