use super::{RenderResourceId, RenderResourceSet, RenderResourceSetId};
use crate::pipeline::{BindGroupDescriptor, BindGroupDescriptorId, PipelineSpecialization};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
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
pub enum RenderResourceSetStatus {
    Changed(RenderResourceSetId),
    Unchanged(RenderResourceSetId),
    NoMatch,
}

// PERF: if the assignments are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug, Default)]
pub struct RenderResourceAssignments {
    pub id: RenderResourceAssignmentsId,
    render_resources: HashMap<String, RenderResourceAssignment>,
    vertex_buffers: HashMap<String, (RenderResourceId, Option<RenderResourceId>)>,
    render_resource_sets: HashMap<RenderResourceSetId, RenderResourceSet>,
    bind_group_render_resource_sets: HashMap<BindGroupDescriptorId, RenderResourceSetId>,
    dirty_render_resource_sets: HashSet<RenderResourceSetId>,
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
                // TODO: this is crude. we shouldn't need to invalidate all render resource sets
                for id in self.render_resource_sets.keys() {
                    self.dirty_render_resource_sets.insert(*id);
                }
            }
        }
    }

    pub fn extend(&mut self, render_resource_assignments: &RenderResourceAssignments) {
        for (name, assignment) in render_resource_assignments.render_resources.iter() {
            self.set(name, assignment.clone());
        }

        for (name, (vertex_buffer, index_buffer)) in
            render_resource_assignments.vertex_buffers.iter()
        {
            self.set_vertex_buffer(name, *vertex_buffer, index_buffer.clone());
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

    fn create_render_resource_set(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> RenderResourceSetStatus {
        let resource_set = self.build_render_resource_set(bind_group_descriptor);
        if let Some(resource_set) = resource_set {
            let id = resource_set.id;
            self.render_resource_sets.insert(id, resource_set);
            self.bind_group_render_resource_sets
                .insert(bind_group_descriptor.id, id);
            RenderResourceSetStatus::Changed(id)
        } else {
            RenderResourceSetStatus::NoMatch
        }
    }

    pub fn update_bind_group(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> RenderResourceSetStatus {
        if let Some(id) = self
            .bind_group_render_resource_sets
            .get(&bind_group_descriptor.id)
        {
            if self.dirty_render_resource_sets.contains(id) {
                self.dirty_render_resource_sets.remove(id);
                self.create_render_resource_set(bind_group_descriptor)
            } else {
                RenderResourceSetStatus::Unchanged(*id)
            }
        } else {
            self.create_render_resource_set(bind_group_descriptor)
        }
    }

    pub fn get_render_resource_set(&self, id: RenderResourceSetId) -> Option<&RenderResourceSet> {
        self.render_resource_sets.get(&id)
    }

    pub fn get_bind_group_render_resource_set(
        &self,
        id: BindGroupDescriptorId,
    ) -> Option<&RenderResourceSet> {
        self.bind_group_render_resource_sets
            .get(&id)
            .and_then(|render_resource_set_id| {
                self.get_render_resource_set(*render_resource_set_id)
            })
    }

    fn build_render_resource_set(
        &self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> Option<RenderResourceSet> {
        let mut render_resource_set_builder = RenderResourceSet::build();
        for binding_descriptor in bind_group_descriptor.bindings.iter() {
            if let Some(assignment) = self.get(&binding_descriptor.name) {
                render_resource_set_builder = render_resource_set_builder
                    .add_assignment(binding_descriptor.index, assignment.clone());
            } else {
                return None;
            }
        }

        Some(render_resource_set_builder.finish())
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct RenderResourceAssignmentsId(Uuid);

impl Default for RenderResourceAssignmentsId {
    fn default() -> Self {
        RenderResourceAssignmentsId(Uuid::new_v4())
    }
}

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

        let status = assignments.update_bind_group(&bind_group_descriptor);
        let id = if let RenderResourceSetStatus::Changed(id) = status {
            id
        } else {
            panic!("expected a changed render resource set");
        };

        let different_set_status = different_assignments.update_bind_group(&bind_group_descriptor);
        if let RenderResourceSetStatus::Changed(different_set_id) = different_set_status {
            assert_ne!(
                id, different_set_id,
                "different set shouldn't have the same id"
            );
            different_set_id
        } else {
            panic!("expected a changed render resource set");
        };

        let equal_set_status = equal_assignments.update_bind_group(&bind_group_descriptor);
        if let RenderResourceSetStatus::Changed(equal_set_id) = equal_set_status {
            assert_eq!(id, equal_set_id, "equal set should have the same id");
        } else {
            panic!("expected a changed render resource set");
        };

        let mut unmatched_assignments = RenderResourceAssignments::default();
        unmatched_assignments.set("a", resource1.clone());
        let unmatched_set_status = unmatched_assignments.update_bind_group(&bind_group_descriptor);
        assert_eq!(unmatched_set_status, RenderResourceSetStatus::NoMatch);
    }
}
