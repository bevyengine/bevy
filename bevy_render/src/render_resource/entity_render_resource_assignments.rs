use super::RenderResourceAssignmentsId;
use crate::Renderable;
use legion::prelude::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct EntityRenderResourceAssignments {
    entity_assignments: HashMap<RenderResourceAssignmentsId, Entity>,
}

impl EntityRenderResourceAssignments {
    pub fn set(&mut self, id: RenderResourceAssignmentsId, entity: Entity) {
        self.entity_assignments.insert(id, entity);
    }

    pub fn get(&self, id: RenderResourceAssignmentsId) -> Option<&Entity> {
        self.entity_assignments.get(&id)
    }
}

// TODO: make sure this runs right before rendering
pub fn build_entity_render_resource_assignments_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("entity_render_resource_assignments")
        .write_resource::<EntityRenderResourceAssignments>()
        .with_query(<Write<Renderable>>::query().filter(changed::<Renderable>()))
        .build(|_, world, entity_assignments, query| {
            for (entity, renderable) in query.iter_entities_mut(world) {
                entity_assignments.set(renderable.render_resource_assignments.id, entity);
            }
        })
}
