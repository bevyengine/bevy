use super::{RenderResourceAssignmentsId, RenderResourceAssignmentsProvider};
use crate::prelude::Renderable;
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

pub fn build_entity_render_resource_assignments_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("EntityRenderResourceAssignments")
        .write_resource::<EntityRenderResourceAssignments>()
        .write_resource::<RenderResourceAssignmentsProvider>()
        .with_query(<Write<Renderable>>::query().filter(changed::<Renderable>()))
        .build(|_, world, (entity_assignments, provider), query| {
            for (entity, mut renderable) in query.iter_entities_mut(world) {
                if renderable.is_instanced {
                    renderable.render_resource_assignments = None;
                } else if let None = renderable.render_resource_assignments {
                    let render_resource_assignments = provider.next();
                    entity_assignments.set(render_resource_assignments.get_id(), entity);
                    renderable.render_resource_assignments = Some(render_resource_assignments);
                }
            }
        })
}
