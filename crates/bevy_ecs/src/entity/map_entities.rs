use crate::entity::Entity;
use bevy_utils::{Entry, HashMap};
use std::fmt;

#[derive(Debug)]
pub enum MapEntitiesError {
    EntityNotFound(Entity),
}

impl std::error::Error for MapEntitiesError {}

impl fmt::Display for MapEntitiesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MapEntitiesError::EntityNotFound(_) => {
                write!(f, "the given entity does not exist in the map")
            }
        }
    }
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
