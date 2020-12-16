use bevy_ecs::{Entity, MapEntities};
use bevy_reflect::{Reflect, ReflectComponent, ReflectMapEntities};
use bevy_utils::HashMap;
use smallvec::SmallVec;
use std::ops::Deref;

#[derive(Default, Clone, Debug, Reflect)]
#[reflect(Component, MapEntities)]
pub struct Children {
    order: SmallVec<[Entity; 8]>,
    uniqueness: HashMap<Entity, usize>,
}

impl MapEntities for Children {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::EntityMap,
    ) -> Result<(), bevy_ecs::MapEntitiesError> {
        for entity in self.order.iter_mut() {
            *entity = entity_map.get(*entity)?;
        }

        Ok(())
    }
}

impl Children {
    pub fn with(entities: &[Entity]) -> Self {
        let mut children = Self::default();
        for entity in entities {
            children.push(*entity);
        }
        children
    }

    /// Swaps the child at `a_index` with the child at `b_index`
    pub fn swap(&mut self, a_index: usize, b_index: usize) {
        let a_entity = self.order[a_index];
        let b_entity = self.order[b_index];
        self.order.swap(a_index, b_index);
        self.uniqueness.insert(a_entity, b_index);
        self.uniqueness.insert(b_entity, a_index);
    }

    pub(crate) fn push(&mut self, entity: Entity) {
        let order = &mut self.order;
        let uniqueness = &mut self.uniqueness;

        uniqueness.entry(entity).or_insert_with(|| {
            order.push(entity);
            order.len() - 1
        });

        let desired_index = order.len() - 1;
        let current_index = uniqueness[&entity];
        if current_index != desired_index {
            self.swap(current_index, desired_index);
        }
    }

    pub(crate) fn retain<F: FnMut(&Entity) -> bool>(&mut self, mut f: F) {
        let order = &mut self.order;
        let uniqueness = &mut self.uniqueness;

        let mut offset = 0;
        order.retain(|e| {
            if f(e) {
                *uniqueness.get_mut(e).unwrap() -= offset;
                true
            } else {
                offset += 1;
                uniqueness.remove(e);
                false
            }
        });
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.uniqueness.contains_key(entity)
    }

    pub(crate) fn extend<T: IntoIterator<Item = Entity>>(&mut self, iter: T) {
        for entity in iter {
            self.push(entity);
        }
    }

    pub(crate) fn insert(&mut self, index: usize, entities: &[Entity]) {
        self.extend(entities.iter().cloned());
        let mut desired_index = index;
        for entity in entities {
            let current_index = self.uniqueness[entity];
            if current_index != desired_index {
                self.swap(current_index, desired_index);
            }
            desired_index += 1;
        }
    }

    pub(crate) fn take(&mut self) -> SmallVec<[Entity; 8]> {
        self.uniqueness.clear();
        std::mem::take(&mut self.order)
    }
}

impl Deref for Children {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.order[..]
    }
}
