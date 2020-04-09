use crate::WgpuResources;

use bevy_render::{
    render_resource::{BufferInfo, RenderResource, RenderResources, ResourceInfo},
    renderer_2::RenderContext,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use std::sync::Arc;

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

pub struct WgpuRenderContext {
    pub device: Arc<wgpu::Device>,
    command_encoder: LazyCommandEncoder,
    wgpu_resources: WgpuResources,
}

impl WgpuRenderContext {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        WgpuRenderContext {
            device,
            command_encoder: LazyCommandEncoder::default(),
            wgpu_resources: WgpuResources::default(),
        }
    }

    pub fn finish(&mut self) -> Option<wgpu::CommandBuffer> {
        self.command_encoder.take().map(|encoder| encoder.finish())
    }
}

impl RenderContext for WgpuRenderContext {
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
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderContext),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped_render_context(
            &buffer_info,
            self,
            setup_data,
        );
        self.wgpu_resources.assign_buffer(buffer, buffer_info)
    }
    fn create_texture_with_data(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource {
        self.wgpu_resources.create_texture_with_data(
            &self.device,
            self.command_encoder.get_or_create(&self.device),
            texture_descriptor,
            bytes,
        )
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
    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.wgpu_resources.resource_info.get(&resource)
    }
    fn get_resource_info_mut(&mut self, resource: RenderResource) -> Option<&mut ResourceInfo> {
        self.wgpu_resources.resource_info.get_mut(&resource)
    }
    fn render_resources(&self) -> &RenderResources {
        &self.wgpu_resources.render_resources
    }
    fn render_resources_mut(&mut self) -> &mut RenderResources {
        &mut self.wgpu_resources.render_resources
    }
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        self.wgpu_resources.copy_buffer_to_buffer(
            self.command_encoder.get_or_create(&self.device),
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        );
    }
    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.wgpu_resources
            .create_buffer_with_data(&self.device, buffer_info, data)
    }
}
