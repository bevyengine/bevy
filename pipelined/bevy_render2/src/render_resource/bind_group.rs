use crate::render_resource::{
    BufferId, RenderResourceBinding, RenderResourceId, SamplerId, TextureViewId,
};
use bevy_utils::AHasher;
use std::{
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
};

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct BindGroupId(pub u64);

#[derive(Eq, PartialEq, Debug)]
pub struct IndexedBindGroupEntry {
    pub index: u32,
    pub entry: RenderResourceBinding,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BindGroup {
    pub id: BindGroupId,
    pub indexed_bindings: Arc<[IndexedBindGroupEntry]>,
}

impl BindGroup {
    pub fn build() -> BindGroupBuilder {
        BindGroupBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct BindGroupBuilder {
    pub indexed_bindings: Vec<IndexedBindGroupEntry>,
    pub hasher: AHasher,
}

impl BindGroupBuilder {
    pub fn add_binding<T: Into<RenderResourceBinding>>(mut self, index: u32, binding: T) -> Self {
        let binding = binding.into();
        self.hash_binding(&binding);
        self.indexed_bindings.push(IndexedBindGroupEntry {
            index,
            entry: binding,
        });
        self
    }

    pub fn add_texture_view(self, index: u32, texture: TextureViewId) -> Self {
        self.add_binding(index, RenderResourceBinding::TextureView(texture))
    }

    pub fn add_sampler(self, index: u32, sampler: SamplerId) -> Self {
        self.add_binding(index, RenderResourceBinding::Sampler(sampler))
    }

    pub fn add_buffer(self, index: u32, buffer: BufferId, range: Range<u64>) -> Self {
        self.add_binding(
            index,
            RenderResourceBinding::Buffer {
                buffer,
                range,
            },
        )
    }

    pub fn finish(mut self) -> BindGroup {
        // this sort ensures that RenderResourceSets are insertion-order independent
        self.indexed_bindings.sort_by_key(|i| i.index);
        BindGroup {
            id: BindGroupId(self.hasher.finish()),
            indexed_bindings: self.indexed_bindings.into(),
        }
    }

    fn hash_binding(&mut self, binding: &RenderResourceBinding) {
        match binding {
            RenderResourceBinding::Buffer {
                buffer,
                range,
            } => {
                RenderResourceId::Buffer(*buffer).hash(&mut self.hasher);
                range.hash(&mut self.hasher);
            }
            RenderResourceBinding::TextureView(texture) => {
                RenderResourceId::TextureView(*texture).hash(&mut self.hasher);
            }
            RenderResourceBinding::Sampler(sampler) => {
                RenderResourceId::Sampler(*sampler).hash(&mut self.hasher);
            }
        }
    }
}
