use crate::{
    pipeline::{BindGroupDescriptor, PipelineDescriptor},
    render_resource::{
        BufferInfo, RenderResource, RenderResourceAssignments, RenderResourceSetId, ResourceInfo,
    },
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_window::{Window, WindowId};
use downcast_rs::{impl_downcast, Downcast};

pub struct RenderResources {
    pub context: Box<dyn RenderResourceContext>,
}

impl RenderResources {
    pub fn new<T>(context: T) -> RenderResources
    where
        T: RenderResourceContext,
    {
        RenderResources {
            context: Box::new(context),
        }
    }
}

pub trait RenderResourceContext: Downcast + Send + Sync + 'static {
    fn create_swap_chain(&self, window: &Window);
    fn next_swap_chain_texture(&self, window_id: WindowId) -> RenderResource;
    fn drop_swap_chain_texture(&self, render_resource: RenderResource);
    fn drop_all_swap_chain_textures(&self);
    fn create_sampler(&self, sampler_descriptor: &SamplerDescriptor) -> RenderResource;
    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> RenderResource;
    fn create_buffer(&self, buffer_info: BufferInfo) -> RenderResource;
    // TODO: remove RenderResourceContext here
    fn create_buffer_mapped(
        &self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) -> RenderResource;
    fn create_buffer_with_data(&self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource;
    fn create_shader_module(&self, shader_handle: Handle<Shader>, shaders: &Assets<Shader>);
    fn create_shader_module_from_source(&self, shader_handle: Handle<Shader>, shader: &Shader);
    fn remove_buffer(&self, resource: RenderResource);
    fn remove_texture(&self, resource: RenderResource);
    fn remove_sampler(&self, resource: RenderResource);
    fn get_resource_info(
        &self,
        resource: RenderResource,
        handle_info: &mut dyn FnMut(Option<&ResourceInfo>),
    );
    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResource,
        index: usize,
    );
    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: usize,
    ) -> Option<RenderResource>;
    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: usize);
    fn create_render_pipeline(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
        shaders: &Assets<Shader>,
    );
    fn create_bind_group(
        &self,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<RenderResourceSetId>;
    fn setup_bind_groups(
        &self,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            self.create_bind_group(bind_group, render_resource_assignments);
        }
    }
    fn clear_bind_groups(&self);
}

impl dyn RenderResourceContext {
    pub fn set_asset_resource<T>(
        &self,
        handle: Handle<T>,
        render_resource: RenderResource,
        index: usize,
    ) where
        T: 'static,
    {
        self.set_asset_resource_untyped(handle.into(), render_resource, index);
    }
    pub fn get_asset_resource<T>(&self, handle: Handle<T>, index: usize) -> Option<RenderResource>
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
