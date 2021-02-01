use crate::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor, PipelineLayout},
    renderer::{BindGroup, BufferId, BufferInfo, RenderResourceId, SamplerId, TextureId},
    shader::{Shader, ShaderError, ShaderLayout, ShaderStages},
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_asset::{Asset, Assets, Handle, HandleUntyped};
use bevy_window::Window;
use downcast_rs::{impl_downcast, Downcast};
use std::ops::Range;

pub trait RenderResourceContext: Downcast + Send + Sync + 'static {
    fn create_swap_chain(&self, window: &Window);
    fn next_swap_chain_texture(&self, window: &Window) -> TextureId;
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
    fn map_buffer(&self, id: BufferId);
    fn unmap_buffer(&self, id: BufferId);
    fn create_buffer_with_data(&self, buffer_info: BufferInfo, data: &[u8]) -> BufferId;
    fn create_shader_module(&self, shader_handle: &Handle<Shader>, shaders: &Assets<Shader>);
    fn create_shader_module_from_source(&self, shader_handle: &Handle<Shader>, shader: &Shader);
    fn get_specialized_shader(
        &self,
        shader: &Shader,
        macros: Option<&[String]>,
    ) -> Result<Shader, ShaderError>;
    fn remove_buffer(&self, buffer: BufferId);
    fn remove_texture(&self, texture: TextureId);
    fn remove_sampler(&self, sampler: SamplerId);
    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo>;
    fn get_aligned_uniform_size(&self, size: usize, dynamic: bool) -> usize;
    fn get_aligned_texture_size(&self, data_size: usize) -> usize;
    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        resource: RenderResourceId,
        index: u64,
    );
    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: u64,
    ) -> Option<RenderResourceId>;
    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: u64);
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
    fn remove_stale_bind_groups(&self);
    /// Reflects the pipeline layout from its shaders.
    ///
    /// If `bevy_conventions` is true, it will be assumed that the shader follows "bevy shader conventions". These allow
    /// richer reflection, such as inferred Vertex Buffer names and inferred instancing.
    ///
    /// If `dynamic_bindings` has values, shader uniforms will be set to "dynamic" if there is a matching binding in the list
    ///
    /// If `vertex_buffer_descriptors` is set, the pipeline's vertex buffers
    /// will inherit their layouts from global descriptors, otherwise the layout will be assumed to be complete / local.
    fn reflect_pipeline_layout(
        &self,
        shaders: &Assets<Shader>,
        shader_stages: &ShaderStages,
        enforce_bevy_conventions: bool,
    ) -> PipelineLayout {
        // TODO: maybe move this default implementation to PipelineLayout?
        let mut shader_layouts: Vec<ShaderLayout> = shader_stages
            .iter()
            .map(|handle| {
                shaders
                    .get(&handle)
                    .unwrap()
                    .reflect_layout(enforce_bevy_conventions)
                    .unwrap()
            })
            .collect();
        PipelineLayout::from_shader_layouts(&mut shader_layouts)
    }
}

impl dyn RenderResourceContext {
    pub fn set_asset_resource<T>(&self, handle: &Handle<T>, resource: RenderResourceId, index: u64)
    where
        T: Asset,
    {
        self.set_asset_resource_untyped(handle.clone_weak_untyped(), resource, index);
    }

    pub fn get_asset_resource<T>(&self, handle: &Handle<T>, index: u64) -> Option<RenderResourceId>
    where
        T: Asset,
    {
        self.get_asset_resource_untyped(handle.clone_weak_untyped(), index)
    }

    pub fn remove_asset_resource<T>(&self, handle: &Handle<T>, index: u64)
    where
        T: Asset,
    {
        self.remove_asset_resource_untyped(handle.clone_weak_untyped(), index);
    }
}

impl_downcast!(RenderResourceContext);
