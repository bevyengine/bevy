use super::{BufferId, RenderResourceBinding, RenderResourceId, SamplerId, TextureId};
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
    pub dynamic_uniform_indices: Option<Arc<[u32]>>,
}

impl BindGroup {
    pub fn build() -> BindGroupBuilder {
        BindGroupBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct BindGroupBuilder {
    pub indexed_bindings: Vec<IndexedBindGroupEntry>,
    pub dynamic_uniform_indices: Vec<u32>,
    pub hasher: AHasher,
}

impl BindGroupBuilder {
    pub fn add_binding(mut self, index: u32, binding: RenderResourceBinding) -> Self {
        if let RenderResourceBinding::Buffer {
            dynamic_index: Some(dynamic_index),
            ..
        } = binding
        {
            self.dynamic_uniform_indices.push(dynamic_index);
        }

        self.hash_binding(&binding);
        self.indexed_bindings.push(IndexedBindGroupEntry {
            index,
            entry: binding,
        });
        self
    }

    pub fn add_texture(self, index: u32, texture: TextureId) -> Self {
        self.add_binding(index, RenderResourceBinding::Texture(texture))
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
                dynamic_index: None,
            },
        )
    }

    pub fn add_dynamic_buffer(
        self,
        index: u32,
        buffer: BufferId,
        range: Range<u64>,
        dynamic_index: u32,
    ) -> Self {
        self.add_binding(
            index,
            RenderResourceBinding::Buffer {
                buffer,
                range,
                dynamic_index: Some(dynamic_index),
            },
        )
    }

    pub fn finish(mut self) -> BindGroup {
        // this sort ensures that RenderResourceSets are insertion-order independent
        self.indexed_bindings.sort_by_key(|i| i.index);
        BindGroup {
            id: BindGroupId(self.hasher.finish()),
            indexed_bindings: self.indexed_bindings.into(),
            dynamic_uniform_indices: if self.dynamic_uniform_indices.is_empty() {
                None
            } else {
                Some(self.dynamic_uniform_indices.into())
            },
        }
    }

    fn hash_binding(&mut self, binding: &RenderResourceBinding) {
        match binding {
            RenderResourceBinding::Buffer {
                buffer,
                range,
                dynamic_index: _, // dynamic_index is not a part of the binding
            } => {
                RenderResourceId::from(*buffer).hash(&mut self.hasher);
                range.hash(&mut self.hasher);
            }
            RenderResourceBinding::Texture(texture) => {
                RenderResourceId::from(*texture).hash(&mut self.hasher);
            }
            RenderResourceBinding::Sampler(sampler) => {
                RenderResourceId::from(*sampler).hash(&mut self.hasher);
            }
        }
    }
}
