use super::RenderResourceContext;
use crate::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    renderer::{BindGroup, BufferId, BufferInfo, RenderResourceId, SamplerId, TextureId},
    shader::{Shader, ShaderError},
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_utils::HashMap;
use bevy_window::Window;
use parking_lot::RwLock;
use std::{ops::Range, sync::Arc};

#[derive(Debug, Default)]
pub struct HeadlessRenderResourceContext {
    buffer_info: Arc<RwLock<HashMap<BufferId, BufferInfo>>>,
    texture_descriptors: Arc<RwLock<HashMap<TextureId, TextureDescriptor>>>,
    pub asset_resources: Arc<RwLock<HashMap<(HandleUntyped, u64), RenderResourceId>>>,
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
    fn create_swap_chain(&self, _window: &Window) {}

    fn next_swap_chain_texture(&self, _window: &Window) -> TextureId {
        TextureId::new()
    }

    fn drop_swap_chain_texture(&self, _render_resource: TextureId) {}

    fn drop_all_swap_chain_textures(&self) {}

    fn create_sampler(&self, _sampler_descriptor: &SamplerDescriptor) -> SamplerId {
        SamplerId::new()
    }

    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> TextureId {
        let texture = TextureId::new();
        self.add_texture_descriptor(texture, texture_descriptor);
        texture
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

    fn map_buffer(&self, _id: BufferId) {}

    fn unmap_buffer(&self, _id: BufferId) {}

    fn create_buffer_with_data(&self, buffer_info: BufferInfo, _data: &[u8]) -> BufferId {
        let buffer = BufferId::new();
        self.add_buffer_info(buffer, buffer_info);
        buffer
    }

    fn create_shader_module(&self, _shader_handle: &Handle<Shader>, _shaders: &Assets<Shader>) {}

    fn remove_buffer(&self, buffer: BufferId) {
        self.buffer_info.write().remove(&buffer);
    }

    fn remove_texture(&self, texture: TextureId) {
        self.texture_descriptors.write().remove(&texture);
    }

    fn remove_sampler(&self, _sampler: SamplerId) {}

    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResourceId,
        index: u64,
    ) {
        self.asset_resources
            .write()
            .insert((handle, index), render_resource);
    }

    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: u64,
    ) -> Option<RenderResourceId> {
        self.asset_resources.write().get(&(handle, index)).cloned()
    }

    fn create_render_pipeline(
        &self,
        _pipeline_handle: Handle<PipelineDescriptor>,
        _pipeline_descriptor: &PipelineDescriptor,
        _shaders: &Assets<Shader>,
    ) {
    }

    fn create_bind_group(
        &self,
        _bind_group_descriptor_id: BindGroupDescriptorId,
        _bind_group: &BindGroup,
    ) {
    }

    fn create_shader_module_from_source(&self, _shader_handle: &Handle<Shader>, _shader: &Shader) {}

    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: u64) {
        self.asset_resources.write().remove(&(handle, index));
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

    fn get_specialized_shader(
        &self,
        shader: &Shader,
        _macros: Option<&[String]>,
    ) -> Result<Shader, ShaderError> {
        Ok(shader.clone())
    }

    fn remove_stale_bind_groups(&self) {}
}
