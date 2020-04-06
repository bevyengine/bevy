use super::{AssetBatcher, Batch};
use bevy_asset::{HandleId, HandleUntyped};
use legion::prelude::Entity;
use std::{any::TypeId, collections::HashMap, hash::Hash};

// TODO: if/when const generics land, revisit this design in favor of generic array lengths

// TODO: add sorting by primary / secondary handle to reduce rebinds of data

#[derive(Hash, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub struct BatchKey2 {
    pub handle1: HandleId,
    pub handle2: HandleId,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct AssetSetBatcherKey2 {
    pub handle1_type: TypeId,
    pub handle2_type: TypeId,
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

pub struct AssetSetBatcher2 {
    key: AssetSetBatcherKey2,
    set_batches: HashMap<BatchKey2, Batch>,
    entity_set_states: HashMap<Entity, EntitySetState2>,
}

impl AssetSetBatcher2 {
    pub fn new(key: AssetSetBatcherKey2) -> Self {
        AssetSetBatcher2 {
            key,
            set_batches: HashMap::new(),
            entity_set_states: HashMap::new(),
        }
    }

    pub fn add_entity_to_set(&mut self, entity: Entity) {
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
