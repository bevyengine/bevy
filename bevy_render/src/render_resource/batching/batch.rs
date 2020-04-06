use crate::render_resource::RenderResourceAssignments;
use bevy_asset::{Handle, HandleUntyped};
use legion::prelude::Entity;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, Debug, Default)]
pub struct Batch {
    pub handles: Vec<HandleUntyped>,
    pub entities: HashSet<Entity>,
    pub instanced_entity_indices: HashMap<Entity, usize>,
    pub current_instanced_entity_index: usize,
    pub render_resource_assignments: RenderResourceAssignments,
}

impl Batch {
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.insert(entity);
    }

    pub fn add_instanced_entity(&mut self, entity: Entity) {
        if let None = self.instanced_entity_indices.get(&entity) {
            self.instanced_entity_indices
                .insert(entity, self.current_instanced_entity_index);
            self.current_instanced_entity_index += 1;
        }
    }

    pub fn get_handle<T>(&self) -> Option<Handle<T>>
    where
        T: 'static,
    {
        self.handles
            .iter()
            .map(|h| Handle::from_untyped(*h))
            .find(|h| h.is_some())
            .map(|h| h.unwrap())
    }
}
