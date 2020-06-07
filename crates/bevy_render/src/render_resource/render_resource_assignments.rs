use super::RenderResourceId;
use crate::pipeline::{BindGroupDescriptor, BindGroupDescriptorId, PipelineSpecialization};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    ops::Range,
};
use uuid::Uuid;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum RenderResourceAssignment {
    Buffer {
        resource: RenderResourceId,
        range: Range<u64>,
        dynamic_index: Option<u32>,
    },
    Texture(RenderResourceId),
    Sampler(RenderResourceId),
}

impl RenderResourceAssignment {
    pub fn get_resource(&self) -> RenderResourceId {
        match self {
            RenderResourceAssignment::Buffer { resource, .. } => *resource,
            RenderResourceAssignment::Texture(resource) => *resource,
            RenderResourceAssignment::Sampler(resource) => *resource,
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct RenderResourceSet {
    pub id: RenderResourceSetId,
    pub dynamic_uniform_indices: Option<Vec<u32>>,
}

// PERF: if the assignments are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug, Default)]
pub struct RenderResourceAssignments {
    pub id: RenderResourceAssignmentsId,
    render_resources: HashMap<String, RenderResourceAssignment>,
    vertex_buffers: HashMap<String, (RenderResourceId, Option<RenderResourceId>)>,
    bind_group_resource_sets: HashMap<BindGroupDescriptorId, RenderResourceSet>,
    dirty_bind_groups: HashSet<BindGroupDescriptorId>,
    pub pipeline_specialization: PipelineSpecialization,
}

impl RenderResourceAssignments {
    pub fn get(&self, name: &str) -> Option<&RenderResourceAssignment> {
        self.render_resources.get(name)
    }

    pub fn set(&mut self, name: &str, assignment: RenderResourceAssignment) {
        self.try_set_dirty(name, &assignment);
        self.render_resources.insert(name.to_string(), assignment);
    }

    fn try_set_dirty(&mut self, name: &str, assignment: &RenderResourceAssignment) {
        if let Some(current_assignment) = self.render_resources.get(name) {
            if current_assignment != assignment {
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
    ) -> Option<(RenderResourceId, Option<RenderResourceId>)> {
        self.vertex_buffers.get(name).cloned()
    }

    pub fn set_vertex_buffer(
        &mut self,
        name: &str,
        vertices_resource: RenderResourceId,
        indices_resource: Option<RenderResourceId>,
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
            let resource_set = self.generate_render_resource_set(bind_group_descriptor);
            if let Some(resource_set) = resource_set {
                let id = resource_set.id;
                self.bind_group_resource_sets
                    .insert(bind_group_descriptor.id, resource_set);
                Some(id)
            } else {
                None
            }
        } else {
            self.bind_group_resource_sets
                .get(&bind_group_descriptor.id)
                .map(|set| set.id)
        }
    }

    pub fn get_render_resource_set(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
    ) -> Option<&RenderResourceSet> {
        self.bind_group_resource_sets.get(&bind_group_descriptor_id)
    }

    fn generate_render_resource_set(
        &self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> Option<RenderResourceSet> {
        let mut hasher = DefaultHasher::new();
        let mut indices = Vec::new();
        for binding_descriptor in bind_group_descriptor.bindings.iter() {
            if let Some(assignment) = self.get(&binding_descriptor.name) {
                let resource = assignment.get_resource();
                resource.hash(&mut hasher);
                if let RenderResourceAssignment::Buffer {
                    dynamic_index: Some(index),
                    ..
                } = assignment
                {
                    indices.push(*index);
                }
            } else {
                return None;
            }
        }

        Some(RenderResourceSet {
            id: RenderResourceSetId(hasher.finish()),
            dynamic_uniform_indices: if indices.is_empty() {
                None
            } else {
                Some(indices)
            },
        })
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
    use crate::pipeline::{BindType, BindingDescriptor, UniformProperty, UniformPropertyType};

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

        let resource1 = RenderResourceAssignment::Texture(RenderResourceId::new());
        let resource2 = RenderResourceAssignment::Texture(RenderResourceId::new());
        let resource3 = RenderResourceAssignment::Texture(RenderResourceId::new());
        let resource4 = RenderResourceAssignment::Texture(RenderResourceId::new());

        let mut assignments = RenderResourceAssignments::default();
        assignments.set("a", resource1.clone());
        assignments.set("b", resource2.clone());

        let mut different_assignments = RenderResourceAssignments::default();
        different_assignments.set("a", resource3.clone());
        different_assignments.set("b", resource4.clone());

        let mut equal_assignments = RenderResourceAssignments::default();
        equal_assignments.set("a", resource1.clone());
        equal_assignments.set("b", resource2.clone());

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
        unmatched_assignments.set("a", resource1.clone());
        let unmatched_set_id =
            unmatched_assignments.update_render_resource_set_id(&bind_group_descriptor);
        assert_eq!(unmatched_set_id, None);
    }
}
