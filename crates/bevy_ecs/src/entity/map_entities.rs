use crate::entity::Entity;
use bevy_utils::{Entry, HashMap};
use std::fmt;

/// The errors that might be returned while using [`MapEntities::map_entities`].
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

/// Operation to map all contained [`Entity`] fields in a type to new values.
///
/// As entity IDs are valid only for the [`World`] they're sourced from, using [`Entity`]
/// as references in components copied from another world will be invalid. This trait
/// allows defining custom mappings for these references via [`EntityMap`].
///
/// Implementing this trait correctly is required for properly loading components
/// with entity references from scenes.
///
/// ## Example
///
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::entity::{EntityMap, MapEntities, MapEntitiesError};
///
/// #[derive(Component)]
/// struct Spring {
///     a: Entity,
///     b: Entity,
/// }
///
/// impl MapEntities for Spring {
///     fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
///         self.a = entity_map.get(self.a)?;
///         self.b = entity_map.get(self.b)?;
///         Ok(())
///     }
/// }
/// ```
///
/// [`World`]: crate::world::World
pub trait MapEntities {
    /// Updates all [`Entity`] references stored inside using `entity_map`.
    ///
    /// Implementors should look up any and all [`Entity`] values stored within and
    /// update them to the mapped values via `entity_map`.
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError>;
}

/// A mapping from one set of entities to another.
///
/// The API generally follows [`HashMap`], but each [`Entity`] is returned by value, as they are [`Copy`].
///
/// This is typically used to coordinate data transfer between sets of entities, such as between a scene and the world or over the network.
/// This is required as [`Entity`] identifiers are opaque; you cannot and do not want to reuse identifiers directly.
#[derive(Default, Debug)]
pub struct EntityMap {
    map: HashMap<Entity, Entity>,
}

impl EntityMap {
    /// Inserts an entities pair into the map.
    ///
    /// If the map did not have `from` present, [`None`] is returned.
    ///
    /// If the map did have `from` present, the value is updated, and the old value is returned.
    pub fn insert(&mut self, from: Entity, to: Entity) -> Option<Entity> {
        self.map.insert(from, to)
    }

    /// Removes an `entity` from the map, returning the mapped value of it if the `entity` was previously in the map.
    pub fn remove(&mut self, entity: Entity) -> Option<Entity> {
        self.map.remove(&entity)
    }

    /// Gets the given entity's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, entity: Entity) -> Entry<'_, Entity, Entity> {
        self.map.entry(entity)
    }

    /// Returns the corresponding mapped entity.
    pub fn get(&self, entity: Entity) -> Result<Entity, MapEntitiesError> {
        self.map
            .get(&entity)
            .cloned()
            .ok_or(MapEntitiesError::EntityNotFound(entity))
    }

    /// An iterator visiting all keys in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.keys().cloned()
    }

    /// An iterator visiting all values in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.values().cloned()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
