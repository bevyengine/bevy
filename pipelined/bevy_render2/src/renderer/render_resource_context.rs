use crate::{
    pipeline::{BindGroupDescriptorId, ComputePipelineDescriptor, PipelineDescriptor, PipelineId},
    render_resource::{
        BindGroup, BufferId, BufferInfo, BufferMapMode, SamplerId, SwapChainDescriptor, TextureId,
    },
    shader::{Shader, ShaderId},
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_window::Window;
use downcast_rs::{impl_downcast, Downcast};
use std::ops::{Deref, DerefMut, Range};

pub struct RenderResources(Box<dyn RenderResourceContext>);

impl RenderResources {
    pub fn new(context: Box<dyn RenderResourceContext>) -> Self {
        Self(context)
    }
}

impl Deref for RenderResources {
    type Target = dyn RenderResourceContext;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for RenderResources {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

pub trait RenderResourceContext: Downcast + Send + Sync + 'static {
    // TODO: remove me
    fn create_swap_chain(&self, window: &Window);
    fn next_swap_chain_texture(&self, descriptor: &SwapChainDescriptor) -> TextureId;
    fn drop_swap_chain_texture(&self, resource: TextureId);
    fn drop_all_swap_chain_textures(&self);
    fn create_sampler(&self, sampler_descriptor: &SamplerDescriptor) -> SamplerId;
    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> TextureId;
    fn create_buffer(&self, buffer_info: BufferInfo) -> BufferId;
    // TODO: remove RenderResourceContext here
    fn write_mapped_buffer(
        &self,
        id: BufferId,
        range: Range<u64>,
        write: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    );
    fn read_mapped_buffer(
        &self,
        id: BufferId,
        range: Range<u64>,
        read: &dyn Fn(&[u8], &dyn RenderResourceContext),
    );
    fn map_buffer(&self, id: BufferId, mode: BufferMapMode);
    fn unmap_buffer(&self, id: BufferId);
    fn create_buffer_with_data(&self, buffer_info: BufferInfo, data: &[u8]) -> BufferId;
    fn create_shader_module(&self, shader: &Shader) -> ShaderId;
    fn remove_buffer(&self, buffer: BufferId);
    fn remove_texture(&self, texture: TextureId);
    fn remove_sampler(&self, sampler: SamplerId);
    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo>;
    fn get_aligned_uniform_size(&self, size: usize, dynamic: bool) -> usize;
    fn get_aligned_texture_size(&self, data_size: usize) -> usize;
    fn create_render_pipeline(&self, pipeline_descriptor: &PipelineDescriptor) -> PipelineId;
    fn create_compute_pipeline(
        &self,
        _pipeline_descriptor: &ComputePipelineDescriptor,
    ) -> PipelineId;
    fn bind_group_descriptor_exists(&self, bind_group_descriptor_id: BindGroupDescriptorId)
        -> bool;
    fn create_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: &BindGroup,
    );
    fn clear_bind_groups(&self);
    fn remove_stale_bind_groups(&self);
}

impl_downcast!(RenderResourceContext);
