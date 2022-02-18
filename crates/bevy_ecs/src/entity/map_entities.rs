use crate::entity::Entity;
use bevy_utils::{Entry, HashMap};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MapEntitiesError {
    #[error("the given entity does not exist in the map")]
    EntityNotFound(Entity),
}

/// Operation to map all contained [`Entity`](crate::entity::Entity) fields in
/// a component to new values.
/// 
/// If a component contains [`Entity`](crate::entity::Entity) values
/// that refer to other entities in the same world and scene functionality
/// is used to create such components, this trait must be implemented. The
/// is to replace all [`Entity`](crate::entity::Entity) values in the
/// component with values looked up from the given [`EntityMap`].
///
/// Implementing this trait is pretty straightforward:
/// 
/// ```
/// #[derive(Component)]
/// struct MyEntityRefs {
///     a: Entity,
///     b: Entity,
/// }
///
/// impl MapEntities for MyEntityRefs {
///     fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
///         self.a = entity_map.get(self.a)?;
///         self.b = entity_map.get(self.b)?;
///         Ok(())
///     }
/// }
/// ```
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
