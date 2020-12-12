use crate::Entity;
use bevy_utils::HashMap;
use std::collections::hash_map::Entry;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MapEntitiesError {
    #[error("the given entity does not exist in the map")]
    EntityNotFound(Entity),
}

pub trait MapEntities {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError>;
}

#[derive(Default, Debug)]
pub struct EntityMap {
    map: HashMap<Entity, Entity>,
}

impl EntityMap {
    pub fn insert(&mut self, from: Entity, to: Entity) {
        self.map.insert(from, to);
    }

    pub fn remove(&mut self, entity: Entity) {
        self.map.remove(&entity);
    }

    pub fn entry(&mut self, entity: Entity) -> Entry<'_, Entity, Entity> {
        self.map.entry(entity)
    }

    pub fn get(&self, entity: Entity) -> Result<Entity, MapEntitiesError> {
        self.map
            .get(&entity)
            .cloned()
            .ok_or(MapEntitiesError::EntityNotFound(entity))
    }

    pub fn keys(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.keys().cloned()
    }

    pub fn values(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.values().cloned()
    }
}
