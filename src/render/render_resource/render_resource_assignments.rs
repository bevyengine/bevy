use super::RenderResource;
use crate::render::pipeline::BindGroupDescriptor;
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};
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

    pub fn get_vertex_buffer(
        &self,
        name: &str,
    ) -> Option<(RenderResource, Option<RenderResource>)> {
        self.vertex_buffers.get(name).cloned()
    }

    pub fn set_vertex_buffer(
        &mut self,
        name: &str,
        vertices_resource: RenderResource,
        indices_resource: Option<RenderResource>,
    ) {
        self.vertex_buffers
            .insert(name.to_string(), (vertices_resource, indices_resource));
    }

    pub fn get_id(&self) -> RenderResourceAssignmentsId {
        self.id
    }

    pub fn get_render_resource_set_id(
        &self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> Option<RenderResourceSetId> {
        let mut hasher = DefaultHasher::new();
        for binding_descriptor in bind_group_descriptor.bindings.iter() {
            if let Some(render_resource) = self.get(&binding_descriptor.name) {
                render_resource.hash(&mut hasher);
            } else {
                return None;
            }
        }

        Some(RenderResourceSetId(hasher.finish()))
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct RenderResourceAssignmentsId(Uuid);

impl Default for RenderResourceAssignmentsId {
    fn default() -> Self {
        RenderResourceAssignmentsId(Uuid::new_v4())
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct RenderResourceSetId(u64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::pipeline::{
        BindType, BindingDescriptor, UniformProperty, UniformPropertyType,
    };

    #[test]
    fn test_render_resource_sets() {
        let bind_group_descriptor = BindGroupDescriptor::new(
            0,
            vec![
                BindingDescriptor {
                    index: 0,
                    name: "a".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: false,
                        properties: vec![UniformProperty {
                            name: "A".to_string(),
                            property_type: UniformPropertyType::Struct(vec![
                                UniformProperty {
                                    name: "".to_string(),
                                    property_type: UniformPropertyType::Mat4,
                                }
                            ]),
                        }],
                    },
                },
                BindingDescriptor {
                    index: 1,
                    name: "b".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: false,
                        properties: vec![UniformProperty {
                            name: "B".to_string(),
                            property_type: UniformPropertyType::Float
                        }],
                    },
                }
            ],
        );

        let mut assignments = RenderResourceAssignments::default();
        assignments.set("a", RenderResource(1));
        assignments.set("b", RenderResource(2));

        let mut different_assignments = RenderResourceAssignments::default();
        different_assignments.set("a", RenderResource(3));
        different_assignments.set("b", RenderResource(4));

        let mut equal_assignments = RenderResourceAssignments::default();
        equal_assignments.set("a", RenderResource(1));
        equal_assignments.set("b", RenderResource(2));

        let set_id = assignments.get_render_resource_set_id(&bind_group_descriptor);
        assert_ne!(set_id, None);

        let different_set_id = different_assignments.get_render_resource_set_id(&bind_group_descriptor);
        assert_ne!(different_set_id, None);
        assert_ne!(different_set_id, set_id);

        let equal_set_id = equal_assignments.get_render_resource_set_id(&bind_group_descriptor);
        assert_ne!(equal_set_id, None);
        assert_eq!(equal_set_id, set_id);

        let mut unmatched_assignments = RenderResourceAssignments::default();
        unmatched_assignments.set("a", RenderResource(1));
        let unmatched_set_id = unmatched_assignments.get_render_resource_set_id(&bind_group_descriptor);
        assert_eq!(unmatched_set_id, None);
    }
}
