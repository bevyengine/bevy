use crate::WgpuResources;

use bevy_asset::{AssetStorage, Handle, HandleUntyped};
use bevy_render::{
    render_resource::{BufferInfo, RenderResource, ResourceInfo},
    renderer_2::RenderResourceContext,
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_window::{Window, WindowId};
use std::sync::Arc;

#[derive(Clone)]
pub struct WgpuRenderResourceContext {
    pub device: Arc<wgpu::Device>,
    pub wgpu_resources: WgpuResources,
}

impl WgpuRenderResourceContext {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        WgpuRenderResourceContext {
            device,
            wgpu_resources: WgpuResources::default(),
        }
    }
}

impl RenderResourceContext for WgpuRenderResourceContext {
    fn create_sampler(&self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_sampler(&self.device, sampler_descriptor)
    }
    fn create_texture(&self, texture_descriptor: &TextureDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_texture(&self.device, texture_descriptor)
    }
    fn create_buffer(&self, buffer_info: BufferInfo) -> RenderResource {
        self.wgpu_resources.create_buffer(&self.device, buffer_info)
    }

    // TODO: clean this up
    fn create_buffer_mapped(
        &self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped_render_context(
            &buffer_info,
            self,
            setup_data,
        );
        self.wgpu_resources.assign_buffer(buffer, buffer_info)
    }

    fn create_buffer_with_data(&self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.wgpu_resources
            .create_buffer_with_data(&self.device, buffer_info, data)
    }

    fn remove_buffer(&self, resource: RenderResource) {
        self.wgpu_resources.remove_buffer(resource);
    }
    fn remove_texture(&self, resource: RenderResource) {
        self.wgpu_resources.remove_texture(resource);
    }
    fn remove_sampler(&self, resource: RenderResource) {
        self.wgpu_resources.remove_sampler(resource);
    }

    fn get_resource_info(
        &self,
        resource: RenderResource,
        handle_info: &mut dyn FnMut(Option<&ResourceInfo>),
    ) {
        self.wgpu_resources.get_resource_info(resource, handle_info);
    }

    fn create_shader_module(
        &mut self,
        shader_handle: Handle<Shader>,
        shader_storage: &AssetStorage<Shader>,
    ) {
        if self
            .wgpu_resources
            .shader_modules
            .read()
            .unwrap()
            .get(&shader_handle)
            .is_some()
        {
            return;
        }

        let shader = shader_storage.get(&shader_handle).unwrap();
        self.wgpu_resources
            .create_shader_module(&self.device, shader_handle, shader);
    }
    fn create_swap_chain(&self, window: &Window) {
        self.wgpu_resources
            .create_window_swap_chain(&self.device, window)
    }
    fn next_swap_chain_texture(&self, window_id: bevy_window::WindowId) {
        self.wgpu_resources.next_swap_chain_texture(window_id);
    }
    fn drop_swap_chain_texture(&self, window_id: WindowId) {
        self.wgpu_resources.remove_swap_chain_texture(window_id);
    }
    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResource,
        index: usize,
    ) {
        self.wgpu_resources
            .set_asset_resource_untyped(handle, render_resource, index);
    }
    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: usize,
    ) -> Option<RenderResource> {
        self.wgpu_resources
            .get_asset_resource_untyped(handle, index)
    }
}
