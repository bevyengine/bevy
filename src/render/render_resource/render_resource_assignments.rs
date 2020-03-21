use super::RenderResource;
use std::collections::HashMap;

// TODO: consider merging this with entity_uniform_resource
// PERF: if the assignments are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug)]
pub struct RenderResourceAssignments {
    id: RenderResourceAssignmentsId,
    render_resources: HashMap<String, RenderResource>,
}

impl RenderResourceAssignments {
    pub fn get(&self, name: &str) -> Option<RenderResource> {
        self.render_resources.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, resource: RenderResource) {
        self.render_resources.insert(name.to_string(), resource);
    }

    pub fn get_id(&self) -> RenderResourceAssignmentsId {
        self.id
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct RenderResourceAssignmentsId(usize);

#[derive(Default)]
pub struct RenderResourceAssignmentsProvider {
    pub current_id: usize,
}

impl RenderResourceAssignmentsProvider {
    pub fn next(&mut self) -> RenderResourceAssignments {
        let assignments = RenderResourceAssignments {
            id: RenderResourceAssignmentsId(self.current_id),
            render_resources: HashMap::new(),
        };

        self.current_id += 1;
        assignments
    }
}