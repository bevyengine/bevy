use super::RenderResourceAssignments;
use legion::prelude::Entity;
use std::collections::HashMap;

#[derive(Default)]
pub struct EntityRenderResourceAssignments {
    entity_assignments: HashMap<Entity, RenderResourceAssignments>,
}

impl EntityRenderResourceAssignments {
    pub fn set(&mut self, entity: Entity, assignments: RenderResourceAssignments) {
        self.entity_assignments.insert(entity, assignments);
    }

    pub fn get(&self, entity: Entity) -> Option<&RenderResourceAssignments> {
        self.entity_assignments.get(&entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut RenderResourceAssignments> {
        self.entity_assignments.get_mut(&entity)
    }
}
