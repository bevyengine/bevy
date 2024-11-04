use std::cell::RefCell;
use bevy_utils::Parallel;
use crate::component::ComponentId;
use crate::entity::Entity;

/// A shorthand for [`Vec<EntityChange>`].
pub type EntityChanges = Vec<EntityChange>;

/// A Collection of EntityChange storages
/// Can be accessed via [`World`]
#[derive(Default)]
pub struct ParallelEntityChanges {
    list: Parallel<EntityChanges>,
}

impl ParallelEntityChanges {

    /// Returns a default `Changes`

    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a mutable iterator over all of the per-thread [`EntityChanges`].
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut EntityChanges> {
        self.list.iter_mut()
    }

    /// Get a thread local [`EntityChanges`] instance
    pub fn get_local_ref_cell(&self) -> &RefCell<EntityChanges> {
        self.list.borrow_local_cell()
    }
}

/// A Record hint which entity's component has changed
#[derive(Copy, Clone, Debug)]
pub struct EntityChange {
    entity:    Entity,
    component: ComponentId,
}

impl EntityChange {
    /// Return a new EntityChange
    pub fn new(entity: Entity, component: ComponentId) -> Self {
        EntityChange { entity, component }
    }
    /// Access change's entity
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Access change's component id
    pub fn component(&self) -> ComponentId {
        self.component
    }
}