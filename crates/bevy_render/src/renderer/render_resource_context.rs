use crate::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    render_resource::{BindGroup, BufferId, BufferInfo, RenderResourceId, SamplerId, TextureId},
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_window::{Window, WindowId};
use downcast_rs::{impl_downcast, Downcast};
use std::ops::Range;

pub trait RenderResourceContext: Downcast + Send + Sync + 'static {
    fn create_swap_chain(&self, window: &Window);
    fn next_swap_chain_texture(&self, window_id: WindowId) -> TextureId;
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
    fn map_buffer(
        &self,
        id: BufferId,
    );
    fn unmap_buffer(
        &self,
        id: BufferId,
    );
    fn create_buffer_with_data(&self, buffer_info: BufferInfo, data: &[u8]) -> BufferId;
    fn create_shader_module(&self, shader_handle: Handle<Shader>, shaders: &Assets<Shader>);
    fn create_shader_module_from_source(&self, shader_handle: Handle<Shader>, shader: &Shader);
    fn remove_buffer(&self, buffer: BufferId);
    fn remove_texture(&self, texture: TextureId);
    fn remove_sampler(&self, sampler: SamplerId);
    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo>;

    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        resource: RenderResourceId,
        index: usize,
    );
    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: usize,
    ) -> Option<RenderResourceId>;
    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: usize);
    fn create_render_pipeline(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
        shaders: &Assets<Shader>,
    );
    fn bind_group_descriptor_exists(&self, bind_group_descriptor_id: BindGroupDescriptorId)
        -> bool;
    fn create_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: &BindGroup,
    );
    fn clear_bind_groups(&self);
}

impl dyn RenderResourceContext {
    pub fn set_asset_resource<T>(&self, handle: Handle<T>, resource: RenderResourceId, index: usize)
    where
        T: 'static,
    {
        self.set_asset_resource_untyped(handle.into(), resource, index);
    }
    pub fn get_asset_resource<T>(&self, handle: Handle<T>, index: usize) -> Option<RenderResourceId>
    where
        T: 'static,
    {
        self.get_asset_resource_untyped(handle.into(), index)
    }
    pub fn remove_asset_resource<T>(&self, handle: Handle<T>, index: usize)
    where
        T: 'static,
    {
        self.remove_asset_resource_untyped(handle.into(), index);
    }
}

impl_downcast!(RenderResourceContext);
