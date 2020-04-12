use crate::WgpuResources;

use bevy_asset::Handle;
use bevy_render::{
    mesh::Mesh,
    render_resource::{AssetResources, BufferInfo, RenderResource, ResourceInfo},
    renderer_2::RenderResourceContext,
    texture::{SamplerDescriptor, Texture, TextureDescriptor},
};
use std::sync::Arc;

pub struct WgpuRenderResourceContext {
    pub device: Arc<wgpu::Device>,
    pub wgpu_resources: WgpuResources,
}

// TODO: make this name not terrible
pub trait WgpuRenderResourceContextTrait {
    fn get_buffer(&self, render_resource: RenderResource) -> Option<&wgpu::Buffer>;
    fn create_texture_with_data(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource;
}

impl WgpuRenderResourceContextTrait for WgpuRenderResourceContext {
    fn get_buffer(&self, render_resource: RenderResource) -> Option<&wgpu::Buffer> {
        self.wgpu_resources.buffers.get(&render_resource)
    }
    fn create_texture_with_data(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        self.wgpu_resources
            .create_texture_with_data(
                &self.device,
                command_encoder,
                texture_descriptor,
                bytes,
            )
    }
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
}
