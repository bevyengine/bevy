use crate::{
    pipeline::{
        BindGroupDescriptorId, ComputePipelineDescriptor, PipelineId, RenderPipelineDescriptor,
    },
    render_resource::{
        BindGroup, BufferId, BufferInfo, BufferMapMode, SamplerId, SwapChainDescriptor, TextureId,
        TextureViewId,
    },
    renderer::RenderResourceContext,
    shader::{Shader, ShaderId},
    texture::{SamplerDescriptor, TextureDescriptor, TextureViewDescriptor},
};
use bevy_utils::HashMap;
use parking_lot::RwLock;
use std::{ops::Range, sync::Arc};

#[derive(Debug, Default)]
pub struct HeadlessRenderResourceContext {
    buffer_info: Arc<RwLock<HashMap<BufferId, BufferInfo>>>,
    texture_descriptors: Arc<RwLock<HashMap<TextureId, TextureDescriptor>>>,
    texture_view_descriptors: Arc<RwLock<HashMap<TextureViewId, TextureViewDescriptor>>>,
}

impl HeadlessRenderResourceContext {
    pub fn add_buffer_info(&self, buffer: BufferId, info: BufferInfo) {
        self.buffer_info.write().insert(buffer, info);
    }

    pub fn add_texture_descriptor(&self, texture: TextureId, descriptor: TextureDescriptor) {
        self.texture_descriptors.write().insert(texture, descriptor);
    }
}

impl RenderResourceContext for HeadlessRenderResourceContext {
    fn drop_swap_chain_texture(&self, _texture_view: TextureViewId) {}

    fn drop_all_swap_chain_textures(&self) {}

    fn create_sampler(&self, _sampler_descriptor: &SamplerDescriptor) -> SamplerId {
        SamplerId::new()
    }

    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> TextureId {
        let texture = TextureId::new();
        self.add_texture_descriptor(texture, texture_descriptor);
        texture
    }

    fn create_texture_view(
        &self,
        _texture_id: TextureId,
        texture_view_descriptor: TextureViewDescriptor,
    ) -> TextureViewId {
        let texture_view_id = TextureViewId::new();
        self.texture_view_descriptors
            .write()
            .insert(texture_view_id, texture_view_descriptor);
        texture_view_id
    }

    fn create_buffer(&self, buffer_info: BufferInfo) -> BufferId {
        let buffer = BufferId::new();
        self.add_buffer_info(buffer, buffer_info);
        buffer
    }

    fn write_mapped_buffer(
        &self,
        id: BufferId,
        _range: Range<u64>,
        write: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) {
        let size = self.buffer_info.read().get(&id).unwrap().size;
        let mut buffer = vec![0; size];
        write(&mut buffer, self);
    }

    fn read_mapped_buffer(
        &self,
        id: BufferId,
        _range: Range<u64>,
        read: &dyn Fn(&[u8], &dyn RenderResourceContext),
    ) {
        let size = self.buffer_info.read().get(&id).unwrap().size;
        let buffer = vec![0; size];
        read(&buffer, self);
    }

    fn map_buffer(&self, _id: BufferId, _mode: BufferMapMode) {}

    fn unmap_buffer(&self, _id: BufferId) {}

    fn create_buffer_with_data(&self, buffer_info: BufferInfo, _data: &[u8]) -> BufferId {
        let buffer = BufferId::new();
        self.add_buffer_info(buffer, buffer_info);
        buffer
    }

    fn create_shader_module(&self, _shader: &Shader) -> ShaderId {
        ShaderId::new()
    }

    fn remove_buffer(&self, buffer: BufferId) {
        self.buffer_info.write().remove(&buffer);
    }

    fn remove_texture(&self, texture: TextureId) {
        self.texture_descriptors.write().remove(&texture);
    }

    fn remove_sampler(&self, _sampler: SamplerId) {}

    fn remove_texture_view(&self, texture_view: TextureViewId) {
        self.texture_view_descriptors.write().remove(&texture_view);
    }

    fn create_render_pipeline(
        &self,
        _pipeline_descriptor: &RenderPipelineDescriptor,
    ) -> PipelineId {
        PipelineId::new()
    }

    fn create_compute_pipeline(
        &self,
        _pipeline_descriptor: &ComputePipelineDescriptor,
    ) -> PipelineId {
        PipelineId::new()
    }

    fn create_bind_group(
        &self,
        _bind_group_descriptor_id: BindGroupDescriptorId,
        _bind_group: &BindGroup,
    ) {
    }

    fn clear_bind_groups(&self) {}

    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo> {
        self.buffer_info.read().get(&buffer).cloned()
    }

    fn bind_group_descriptor_exists(
        &self,
        _bind_group_descriptor_id: BindGroupDescriptorId,
    ) -> bool {
        false
    }

    fn get_aligned_uniform_size(&self, size: usize, _dynamic: bool) -> usize {
        size
    }

    fn get_aligned_texture_size(&self, size: usize) -> usize {
        size
    }

    fn remove_stale_bind_groups(&self) {}

    fn next_swap_chain_texture(&self, _descriptor: &SwapChainDescriptor) -> TextureViewId {
        TextureViewId::new()
    }
}
