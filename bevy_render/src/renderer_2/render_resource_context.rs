use crate::{
    render_resource::{BufferInfo, RenderResource, ResourceInfo},
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_asset::{AssetStorage, Handle, HandleUntyped};
use bevy_window::{Window, WindowId};
use downcast_rs::{impl_downcast, Downcast};

pub struct GlobalRenderResourceContext {
    pub context: Box<dyn RenderResourceContext>,
}

impl GlobalRenderResourceContext {
    pub fn new<T>(context: T) -> GlobalRenderResourceContext
    where
        T: RenderResourceContext,
    {
        GlobalRenderResourceContext {
            context: Box::new(context),
        }
    }
}

pub trait RenderResourceContext: Downcast + Send + Sync + 'static {
    fn create_swap_chain(&self, window: &Window);
    fn next_swap_chain_texture(&self, window_id: WindowId);
    fn drop_swap_chain_texture(&self, window_id: WindowId);
    fn create_sampler(&self, sampler_descriptor: &SamplerDescriptor) -> RenderResource;
    fn create_texture(&self, texture_descriptor: &TextureDescriptor) -> RenderResource;
    fn create_buffer(&self, buffer_info: BufferInfo) -> RenderResource;
    fn create_buffer_mapped(
        &self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) -> RenderResource;
    fn create_buffer_with_data(&self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource;
    fn create_shader_module(
        &mut self,
        shader_handle: Handle<Shader>,
        shader_storage: &AssetStorage<Shader>,
    );
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
}

impl_downcast!(RenderResourceContext);
