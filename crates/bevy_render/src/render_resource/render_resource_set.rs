use super::{RenderResourceAssignment, RenderResourceId};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
};

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct RenderResourceSetId(pub u64);

#[derive(Eq, PartialEq, Debug)]
pub struct IndexedRenderResourceAssignment {
    pub index: u32,
    pub assignment: RenderResourceAssignment,
}

// TODO: consider renaming this to BindGroup for parity with renderer terminology
#[derive(Eq, PartialEq, Debug)]
pub struct RenderResourceSet {
    pub id: RenderResourceSetId,
    pub indexed_assignments: Vec<IndexedRenderResourceAssignment>,
    pub dynamic_uniform_indices: Option<Arc<Vec<u32>>>,
}

impl RenderResourceSet {
    pub fn build() -> RenderResourceSetBuilder {
        RenderResourceSetBuilder::default()
    }
}

#[derive(Default)]
pub struct RenderResourceSetBuilder {
    pub indexed_assignments: Vec<IndexedRenderResourceAssignment>,
    pub dynamic_uniform_indices: Vec<u32>,
    pub hasher: DefaultHasher,
}

impl RenderResourceSetBuilder {
    pub fn add_assignment(mut self, index: u32, assignment: RenderResourceAssignment) -> Self {
        if let RenderResourceAssignment::Buffer {
            dynamic_index: Some(dynamic_index),
            ..
        } = assignment
        {
            self.dynamic_uniform_indices.push(dynamic_index);
        }

        let resource = assignment.get_resource();
        resource.hash(&mut self.hasher);
        self.indexed_assignments
            .push(IndexedRenderResourceAssignment { index, assignment });
        self
    }

    pub fn add_texture(self, index: u32, render_resource: RenderResourceId) -> Self {
        self.add_assignment(index, RenderResourceAssignment::Texture(render_resource))
    }

    pub fn add_sampler(self, index: u32, render_resource: RenderResourceId) -> Self {
        self.add_assignment(index, RenderResourceAssignment::Sampler(render_resource))
    }

    pub fn add_buffer(
        self,
        index: u32,
        render_resource: RenderResourceId,
        range: Range<u64>,
    ) -> Self {
        self.add_assignment(
            index,
            RenderResourceAssignment::Buffer {
                resource: render_resource,
                range,
                dynamic_index: None,
            },
        )
    }

    pub fn add_dynamic_buffer(
        self,
        index: u32,
        render_resource: RenderResourceId,
        range: Range<u64>,
        dynamic_index: u32,
    ) -> Self {
        self.add_assignment(
            index,
            RenderResourceAssignment::Buffer {
                resource: render_resource,
                range,
                dynamic_index: Some(dynamic_index),
            },
        )
    }

    pub fn finish(mut self) -> RenderResourceSet {
        // this sort ensures that RenderResourceSets are insertion-order independent
        self.indexed_assignments.sort_by_key(|i| i.index);
        RenderResourceSet {
            id: RenderResourceSetId(self.hasher.finish()),
            indexed_assignments: self.indexed_assignments,
            dynamic_uniform_indices: if self.dynamic_uniform_indices.is_empty() {
                None
            } else {
                Some(Arc::new(self.dynamic_uniform_indices))
            },
        }
    }
}
