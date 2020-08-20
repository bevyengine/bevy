// modified by Bevy contributors

use core::fmt;
use hashbrown::HashMap;
#[cfg(feature = "std")]
use std::error::Error;

/// Lightweight unique ID of an entity
///
/// Obtained from `World::spawn`. Can be stored to refer to an entity in the future.
#[derive(Debug, Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity(u32);

#[allow(clippy::new_without_default)]
impl Entity {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self(rand::random::<u32>())
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn from_id(id: u32) -> Self {
        Self(id)
    }

    /// Extract a transiently unique identifier
    ///
    /// No two simultaneously-live entities share the same ID, but dead entities' IDs may collide
    /// with both live and dead entities. Useful for compactly representing entities within a
    /// specific snapshot of the world, such as when serializing.
    #[inline]
    pub fn id(self) -> u32 {
        self.0
    }
}

#[derive(Default)]
pub(crate) struct Entities {
    pub entity_locations: HashMap<Entity, Location>,
}

impl Entities {
    /// Destroy an entity, allowing it to be reused
    ///
    /// Must not be called on reserved entities prior to `flush`.
    pub fn free(&mut self, entity: Entity) -> Result<Location, NoSuchEntity> {
        if let Some(location) = self.entity_locations.remove(&entity) {
            Ok(location)
        } else {
            Err(NoSuchEntity)
        }
    }

    /// Ensure `n` at least allocations can succeed without reallocating
    pub fn reserve(&mut self, additional: u32) {
        self.entity_locations.reserve(additional as usize)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entity_locations.contains_key(&entity)
    }

    pub fn clear(&mut self) {
        self.entity_locations.clear();
    }

    /// Access the location storage of an entity
    pub fn get_mut(&mut self, entity: Entity) -> Result<&mut Location, NoSuchEntity> {
        self.entity_locations
            .get_mut(&entity)
            .ok_or_else(|| NoSuchEntity)
    }

    /// Access the location storage of an entity
    pub fn insert(&mut self, entity: Entity, location: Location) {
        self.entity_locations.insert(entity, location);
    }

    pub fn get(&self, entity: Entity) -> Result<Location, NoSuchEntity> {
        self.entity_locations
            .get(&entity)
            .cloned()
            .ok_or_else(|| NoSuchEntity)
    }
}

#[derive(Copy, Clone)]
#[allow(missing_docs)]
pub struct Location {
    pub archetype: u32,
    pub index: u32,
}

/// Error indicating that no entity with a particular ID exists
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuchEntity;

impl fmt::Display for NoSuchEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("no such entity")
    }
}

#[cfg(feature = "std")]
impl Error for NoSuchEntity {}
