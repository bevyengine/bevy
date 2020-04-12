use super::WgpuRenderResourceContextTrait;
use bevy_render::{
    render_resource::RenderResource,
    renderer_2::{RenderContext, RenderResourceContext},
    texture::TextureDescriptor,
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

pub struct WgpuRenderContext<T>
where
    T: RenderResourceContext,
{
    pub device: Arc<wgpu::Device>,
    command_encoder: LazyCommandEncoder,
    pub render_resources: T,
}

impl<T> WgpuRenderContext<T>
where
    T: RenderResourceContext,
{
    pub fn new(device: Arc<wgpu::Device>, resources: T) -> Self {
        WgpuRenderContext {
            device,
            render_resources: resources,
            command_encoder: LazyCommandEncoder::default(),
        }
    }

    /// Consume this context, finalize the current CommandEncoder (if it exists), and take the current WgpuResources.
    /// This is intended to be called from a worker thread right before synchronizing with the main thread.   
    pub fn finish(mut self) -> (Option<wgpu::CommandBuffer>, T) {
        (
            self.command_encoder.take().map(|encoder| encoder.finish()),
            self.render_resources,
        )
    }

    /// Consume this context, finalize the current CommandEncoder (if it exists), and take the current WgpuResources.
    /// This is intended to be called from a worker thread right before synchronizing with the main thread.   
    pub fn finish_encoder(&mut self) -> Option<wgpu::CommandBuffer> {
        self.command_encoder.take().map(|encoder| encoder.finish())
    }

    // fn get_buffer<'b>(
    //     render_resource: RenderResource,
    //     local_resources: &'b WgpuResources,
    //     global_resources: &'b WgpuResources,
    // ) -> Option<&'b wgpu::Buffer> {
    //     let buffer = local_resources.buffers.get(&render_resource);
    //     if buffer.is_some() {
    //         return buffer;
    //     }

    //     global_resources.buffers.get(&render_resource)
    // }
}

impl<T> RenderContext for WgpuRenderContext<T>
where
    T: RenderResourceContext + WgpuRenderResourceContextTrait,
{
    fn create_texture_with_data(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        self.render_resources.create_texture_with_data(
            self.command_encoder.get_or_create(&self.device),
            texture_descriptor,
            bytes,
        )
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
        let source = self.render_resources.get_buffer(source_buffer).unwrap();
        let destination = self
            .render_resources
            .get_buffer(destination_buffer)
            .unwrap();
        command_encoder.copy_buffer_to_buffer(
            source,
            source_offset,
            destination,
            destination_offset,
            size,
        );
    }
    fn resources(&self) -> &dyn RenderResourceContext {
        &self.render_resources
    }
    fn resources_mut(&mut self) -> &mut dyn RenderResourceContext {
        &mut self.render_resources
    }
}
