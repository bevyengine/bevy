use super::RenderResource;
use crate::pipeline::{BindGroupDescriptor, BindGroupDescriptorId, PipelineSpecialization};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};
use uuid::Uuid;

// PERF: if the assignments are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug, Default)]
pub struct RenderResourceAssignments {
    pub id: RenderResourceAssignmentsId,
    render_resources: HashMap<String, (RenderResource, Option<u32>)>,
    vertex_buffers: HashMap<String, (RenderResource, Option<RenderResource>)>,
    bind_group_resource_sets:
        HashMap<BindGroupDescriptorId, (RenderResourceSetId, Option<Vec<u32>>)>,
    dirty_bind_groups: HashSet<BindGroupDescriptorId>,
    pub pipeline_specialization: PipelineSpecialization,
}

impl RenderResourceAssignments {
    pub fn get(&self, name: &str) -> Option<RenderResource> {
        self.render_resources.get(name).map(|(r, _i)| *r)
    }

    pub fn get_indexed(&self, name: &str) -> Option<(RenderResource, Option<u32>)> {
        self.render_resources.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, resource: RenderResource) {
        self.try_set_dirty(name, resource);
        self.render_resources
            .insert(name.to_string(), (resource, None));
    }

    pub fn set_indexed(&mut self, name: &str, resource: RenderResource, index: u32) {
        self.try_set_dirty(name, resource);
        self.render_resources
            .insert(name.to_string(), (resource, Some(index)));
    }

    fn try_set_dirty(&mut self, name: &str, resource: RenderResource) {
        if let Some((render_resource, _)) = self.render_resources.get(name) {
            if *render_resource != resource {
                // TODO: this is pretty crude. can we do better?
                for bind_group_id in self.bind_group_resource_sets.keys() {
                    self.dirty_bind_groups.insert(*bind_group_id);
                }
            }
        }
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

    pub fn update_render_resource_set_id(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> Option<RenderResourceSetId> {
        if !self
            .bind_group_resource_sets
            .contains_key(&bind_group_descriptor.id)
            || self.dirty_bind_groups.contains(&bind_group_descriptor.id)
        {
            let result = self.generate_render_resource_set_id(bind_group_descriptor);
            if let Some((set_id, indices)) = result {
                self.bind_group_resource_sets
                    .insert(bind_group_descriptor.id, (set_id, indices));
                Some(set_id)
            } else {
                None
            }
        } else {
            self.bind_group_resource_sets
                .get(&bind_group_descriptor.id)
                .map(|(set_id, _indices)| *set_id)
        }
    }

    pub fn get_render_resource_set_id(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
    ) -> Option<&(RenderResourceSetId, Option<Vec<u32>>)> {
        self.bind_group_resource_sets.get(&bind_group_descriptor_id)
    }

    fn generate_render_resource_set_id(
        &self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> Option<(RenderResourceSetId, Option<Vec<u32>>)> {
        let mut hasher = DefaultHasher::new();
        let mut indices = Vec::new();
        for binding_descriptor in bind_group_descriptor.bindings.iter() {
            if let Some((render_resource, index)) = self.get_indexed(&binding_descriptor.name) {
                render_resource.hash(&mut hasher);
                if let Some(index) = index {
                    indices.push(index);
                }
            } else {
                return None;
            }
        }

        Some((
            RenderResourceSetId(hasher.finish()),
            if indices.is_empty() {
                None
            } else {
                Some(indices)
            },
        ))
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
    use crate::pipeline::{
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
                            property_type: UniformPropertyType::Struct(vec![UniformProperty {
                                name: "".to_string(),
                                property_type: UniformPropertyType::Mat4,
                            }]),
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
                            property_type: UniformPropertyType::Float,
                        }],
                    },
                },
            ],
        );

        let resource1 = RenderResource::new();
        let resource2 = RenderResource::new();
        let resource3 = RenderResource::new();
        let resource4 = RenderResource::new();

        let mut assignments = RenderResourceAssignments::default();
        assignments.set("a", resource1);
        assignments.set("b", resource2);

        let mut different_assignments = RenderResourceAssignments::default();
        different_assignments.set("a", resource3);
        different_assignments.set("b", resource4);

        let mut equal_assignments = RenderResourceAssignments::default();
        equal_assignments.set("a", resource1);
        equal_assignments.set("b", resource2);

        let set_id = assignments.update_render_resource_set_id(&bind_group_descriptor);
        assert_ne!(set_id, None);

        let different_set_id =
            different_assignments.update_render_resource_set_id(&bind_group_descriptor);
        assert_ne!(different_set_id, None);
        assert_ne!(different_set_id, set_id);

        let equal_set_id = equal_assignments.update_render_resource_set_id(&bind_group_descriptor);
        assert_ne!(equal_set_id, None);
        assert_eq!(equal_set_id, set_id);

        let mut unmatched_assignments = RenderResourceAssignments::default();
        unmatched_assignments.set("a", resource1);
        let unmatched_set_id =
            unmatched_assignments.update_render_resource_set_id(&bind_group_descriptor);
        assert_eq!(unmatched_set_id, None);
    }
}
