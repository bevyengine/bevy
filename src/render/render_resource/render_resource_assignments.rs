use super::RenderResource;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// PERF: if the assignments are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug, Default)]
pub struct RenderResourceAssignments {
    id: RenderResourceAssignmentsId,
    render_resources: HashMap<String, RenderResource>,
    vertex_buffers: HashMap<String, (RenderResource, Option<RenderResource>)>,
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

    pub fn get_vertex_buffer(&self, name: &str) -> Option<(RenderResource, Option<RenderResource>)> {
        self.vertex_buffers.get(name).cloned()
    }

    pub fn set_vertex_buffer(&mut self, name: &str, vertices_resource: RenderResource, indices_resource: Option<RenderResource>) {
        self.vertex_buffers.insert(name.to_string(), (vertices_resource, indices_resource));
    }

    pub fn get_id(&self) -> RenderResourceAssignmentsId {
        self.id
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct RenderResourceAssignmentsId(Uuid);

impl Default for RenderResourceAssignmentsId {
    fn default() -> Self {
        RenderResourceAssignmentsId(Uuid::new_v4())
    }
}