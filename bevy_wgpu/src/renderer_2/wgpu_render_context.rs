use crate::WgpuResources;

use bevy_render::{
    render_resource::{BufferInfo, RenderResource, RenderResources, ResourceInfo},
    renderer_2::RenderContext,
    texture::{SamplerDescriptor, TextureDescriptor, Texture}, mesh::Mesh,
};
use std::sync::Arc;
use bevy_asset::Handle;

#[derive(Default)]
struct LazyCommandEncoder {
    command_encoder: Option<wgpu::CommandEncoder>,
}

impl LazyCommandEncoder {
    pub fn get_or_create(&mut self, device: &wgpu::Device) -> &mut wgpu::CommandEncoder {
        match self.command_encoder {
            Some(ref mut command_encoder) => command_encoder,
            None => {
                let command_encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                self.command_encoder = Some(command_encoder);
                self.command_encoder.as_mut().unwrap()
            }
        }
    }

    pub fn take(&mut self) -> Option<wgpu::CommandEncoder> {
        self.command_encoder.take()
    }
}

pub struct WgpuRenderContext<'a> {
    pub device: Arc<wgpu::Device>,
    local_wgpu_resources: WgpuResources,
    command_encoder: LazyCommandEncoder,
    global_wgpu_resources: &'a WgpuResources,
    removed_resources: Vec<RenderResource>,
}

impl<'a> WgpuRenderContext<'a> {
    pub fn new(device: Arc<wgpu::Device>, global_wgpu_resources: &'a WgpuResources) -> Self {
        WgpuRenderContext {
            device,
            global_wgpu_resources: global_wgpu_resources,
            command_encoder: LazyCommandEncoder::default(),
            local_wgpu_resources: WgpuResources::default(),
            removed_resources: Vec::new(),
        }
    }

    pub fn finish(mut self) -> (Option<wgpu::CommandBuffer>, WgpuResources) {
        (self.command_encoder.take().map(|encoder| encoder.finish()), self.local_wgpu_resources)
    }

    fn get_buffer<'b>(render_resource: RenderResource, local_resources: &'b WgpuResources, global_resources: &'b WgpuResources) -> Option<&'b wgpu::Buffer> {
        let buffer = local_resources.buffers.get(&render_resource);
        if buffer.is_some() {
            return buffer;
        }

        global_resources.buffers.get(&render_resource)
    }
}

impl<'a> RenderContext for WgpuRenderContext<'a> {
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.local_wgpu_resources
            .create_sampler(&self.device, sampler_descriptor)
    }
    fn create_texture(&mut self, texture_descriptor: &TextureDescriptor) -> RenderResource {
        self.local_wgpu_resources
            .create_texture(&self.device, texture_descriptor)
    }
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource {
        self.local_wgpu_resources.create_buffer(&self.device, buffer_info)
    }

    // TODO: clean this up
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderContext),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped_render_context(
            &buffer_info,
            self,
            setup_data,
        );
        self.local_wgpu_resources.assign_buffer(buffer, buffer_info)
    }

    fn create_texture_with_data(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        self.local_wgpu_resources.create_texture_with_data(
            &self.device,
            self.command_encoder.get_or_create(&self.device),
            texture_descriptor,
            bytes,
        )
    }
    fn remove_buffer(&mut self, resource: RenderResource) {
        self.local_wgpu_resources.remove_buffer(resource);
        self.removed_resources.push(resource);
    }
    fn remove_texture(&mut self, resource: RenderResource) {
        self.local_wgpu_resources.remove_texture(resource);
        self.removed_resources.push(resource);
    }
    fn remove_sampler(&mut self, resource: RenderResource) {
        self.local_wgpu_resources.remove_sampler(resource);
        self.removed_resources.push(resource);
    }

    // TODO: this pattern is redundant and a bit confusing. make this cleaner if you can
    fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        let local = self.local_wgpu_resources.render_resources.get_texture_resource(texture);
        if local.is_some() {
            return local;
        }

        self.global_wgpu_resources.render_resources.get_texture_resource(texture)
    }

    fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        let local = self.local_wgpu_resources.render_resources.get_texture_sampler_resource(texture);
        if local.is_some() {
            return local;
        }

        self.global_wgpu_resources.render_resources.get_texture_sampler_resource(texture)
    }


    fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        let local = self.local_wgpu_resources.render_resources.get_mesh_vertices_resource(mesh);
        if local.is_some() {
            return local;
        }

        self.global_wgpu_resources.render_resources.get_mesh_vertices_resource(mesh)
    }

    fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        let local = self.local_wgpu_resources.render_resources.get_mesh_indices_resource(mesh);
        if local.is_some() {
            return local;
        }

        self.global_wgpu_resources.render_resources.get_mesh_indices_resource(mesh)
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        let local_info = self.local_wgpu_resources.get_resource_info(resource);
        if local_info.is_some() {
            return local_info;
        }

        self.global_wgpu_resources.get_resource_info(resource)
    }


    fn get_local_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.local_wgpu_resources.resource_info.get(&resource)
    }

    fn local_render_resources(&self) -> &RenderResources {
        &self.local_wgpu_resources.render_resources
    }
    fn local_render_resources_mut(&mut self) -> &mut RenderResources {
        &mut self.local_wgpu_resources.render_resources
    }
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        let command_encoder = self.command_encoder.get_or_create(&self.device);
        let source_buffer = Self::get_buffer(source_buffer, &self.local_wgpu_resources, &self.global_wgpu_resources).unwrap();
        let destination_buffer = Self::get_buffer(destination_buffer, &self.local_wgpu_resources, &self.global_wgpu_resources).unwrap();
        command_encoder.copy_buffer_to_buffer(source_buffer, source_offset, destination_buffer, destination_offset, size);
    }
    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.local_wgpu_resources
            .create_buffer_with_data(&self.device, buffer_info, data)
    }
}
