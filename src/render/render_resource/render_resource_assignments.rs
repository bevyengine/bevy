use super::RenderResource;
use std::collections::{HashMap, HashSet};

// PERF: if the assignments are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug)]
pub struct RenderResourceAssignments {
    id: RenderResourceAssignmentsId,
    render_resources: HashMap<String, RenderResource>,
    pub(crate) shader_defs: HashSet<String>,
    // TODO: move offsets here to reduce hashing costs?
    // render_resource_offsets: HashMap<String, >,
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
            shader_defs: HashSet::new(),
        };

        self.current_id += 1;
        assignments
    }
}
