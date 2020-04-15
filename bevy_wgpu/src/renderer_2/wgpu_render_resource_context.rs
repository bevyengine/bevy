use crate::WgpuResources;

use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    mesh::Mesh,
    render_resource::{
        AssetResources, BufferInfo, RenderResource, ResourceInfo,
    },
    renderer_2::RenderResourceContext,
    shader::Shader,
    texture::{SamplerDescriptor, Texture, TextureDescriptor},
};
use std::sync::Arc;
use bevy_window::{WindowId, Window};

#[derive(Clone)]
pub struct WgpuRenderResourceContext {
    pub device: Arc<wgpu::Device>,
    pub wgpu_resources: Arc<WgpuResources>,
}


impl WgpuRenderResourceContext {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        WgpuRenderResourceContext {
            device,
            wgpu_resources: Arc::new(WgpuResources::default()),
        }
    }
}

impl RenderResourceContext for WgpuRenderResourceContext {
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_sampler(&self.device, sampler_descriptor)
    }
    fn create_texture(&mut self, texture_descriptor: &TextureDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_texture(&self.device, texture_descriptor)
    }
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource {
        self.wgpu_resources.create_buffer(&self.device, buffer_info)
    }

    // TODO: clean this up
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderResourceContext),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped_render_context(
            &buffer_info,
            self,
            setup_data,
        );
        self.wgpu_resources.assign_buffer(buffer, buffer_info)
    }

    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.wgpu_resources
            .create_buffer_with_data(&self.device, buffer_info, data)
    }

    fn remove_buffer(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_buffer(resource);
    }
    fn remove_texture(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_texture(resource);
    }
    fn remove_sampler(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_sampler(resource);
    }

    fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_texture_resource(texture)
    }

    fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_texture_sampler_resource(texture)
    }

    fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_mesh_vertices_resource(mesh)
    }

    fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_mesh_indices_resource(mesh)
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.wgpu_resources.get_resource_info(resource)
    }

    fn asset_resources(&self) -> &AssetResources {
        &self.wgpu_resources.asset_resources
    }
    fn asset_resources_mut(&mut self) -> &mut AssetResources {
        &mut self.wgpu_resources.asset_resources
    }
    fn create_shader_module(
        &mut self,
        shader_handle: Handle<Shader>,
        shader_storage: &AssetStorage<Shader>,
    ) {
        if self.wgpu_resources.shader_modules.read().unwrap().get(&shader_handle).is_some() {
            return;
        }

        let shader = shader_storage.get(&shader_handle).unwrap();
        self.wgpu_resources
            .create_shader_module(&self.device, shader_handle, shader);
    }
    fn create_swap_chain(&mut self, window: &Window) {
        self.wgpu_resources.create_window_swap_chain(&self.device, window)
    }
    fn next_swap_chain_texture(&mut self, window_id: bevy_window::WindowId) {
        self.wgpu_resources.next_swap_chain_texture(window_id);
    }
    fn drop_swap_chain_texture(&mut self, window_id: WindowId) {
        self.wgpu_resources.remove_swap_chain_texture(window_id);
    }
}
