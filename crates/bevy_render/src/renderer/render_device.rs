use crate::render_resource::{
    BindGroup, BindGroupLayout, Buffer, ComputePipeline, RawRenderPipelineDescriptor,
    RenderPipeline, Sampler, Texture,
};
use bevy_ecs::system::Resource;
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferAsyncError, BufferBindingType, MaintainResult,
};

use super::RenderQueue;

use crate::render_resource::resource_macros::*;

render_resource_wrapper!(ErasedRenderDevice, wgpu::Device);

/// This GPU device is responsible for the creation of most rendering and compute resources.
#[derive(Resource, Clone)]
pub struct RenderDevice {
    device: ErasedRenderDevice,
}

impl From<wgpu::Device> for RenderDevice {
    fn from(device: wgpu::Device) -> Self {
        Self {
            device: ErasedRenderDevice::new(device),
        }
    }
}

impl RenderDevice {
    /// List all [`Features`](wgpu::Features) that may be used with this device.
    ///
    /// Functions may panic if you use unsupported features.
    #[inline]
    pub fn features(&self) -> wgpu::Features {
        self.device.features()
    }

    /// List all [`Limits`](wgpu::Limits) that were requested of this device.
    ///
    /// If any of these limits are exceeded, functions may panic.
    #[inline]
    pub fn limits(&self) -> wgpu::Limits {
        self.device.limits()
    }

    /// Creates a [`ShaderModule`](wgpu::ShaderModule) from either SPIR-V or WGSL source code.
    #[inline]
    pub fn create_shader_module(&self, desc: wgpu::ShaderModuleDescriptor) -> wgpu::ShaderModule {
        self.device.create_shader_module(desc)
    }

    /// Check for resource cleanups and mapping callbacks.
    ///
    /// Return `true` if the queue is empty, or `false` if there are more queue
    /// submissions still in flight. (Note that, unless access to the [`wgpu::Queue`] is
    /// coordinated somehow, this information could be out of date by the time
    /// the caller receives it. `Queue`s can be shared between threads, so
    /// other threads could submit new work at any time.)
    ///
    /// no-op on the web, device is automatically polled.
    #[inline]
    pub fn poll(&self, maintain: wgpu::Maintain) -> MaintainResult {
        self.device.poll(maintain)
    }

    /// Creates an empty [`CommandEncoder`](wgpu::CommandEncoder).
    #[inline]
    pub fn create_command_encoder(
        &self,
        desc: &wgpu::CommandEncoderDescriptor,
    ) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(desc)
    }

    /// Creates an empty [`RenderBundleEncoder`](wgpu::RenderBundleEncoder).
    #[inline]
    pub fn create_render_bundle_encoder(
        &self,
        desc: &wgpu::RenderBundleEncoderDescriptor,
    ) -> wgpu::RenderBundleEncoder {
        self.device.create_render_bundle_encoder(desc)
    }

    /// Creates a new [`BindGroup`](wgpu::BindGroup).
    #[inline]
    pub fn create_bind_group<'a>(
        &self,
        label: impl Into<wgpu::Label<'a>>,
        layout: &'a BindGroupLayout,
        entries: &'a [BindGroupEntry<'a>],
    ) -> BindGroup {
        let wgpu_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: label.into(),
            layout,
            entries,
        });
        BindGroup::from(wgpu_bind_group)
    }

    /// Creates a [`BindGroupLayout`](wgpu::BindGroupLayout).
    #[inline]
    pub fn create_bind_group_layout<'a>(
        &self,
        label: impl Into<wgpu::Label<'a>>,
        entries: &'a [BindGroupLayoutEntry],
    ) -> BindGroupLayout {
        BindGroupLayout::from(
            self.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: label.into(),
                    entries,
                }),
        )
    }

    /// Creates a [`PipelineLayout`](wgpu::PipelineLayout).
    #[inline]
    pub fn create_pipeline_layout(
        &self,
        desc: &wgpu::PipelineLayoutDescriptor,
    ) -> wgpu::PipelineLayout {
        self.device.create_pipeline_layout(desc)
    }

    /// Creates a [`RenderPipeline`].
    #[inline]
    pub fn create_render_pipeline(&self, desc: &RawRenderPipelineDescriptor) -> RenderPipeline {
        let wgpu_render_pipeline = self.device.create_render_pipeline(desc);
        RenderPipeline::from(wgpu_render_pipeline)
    }

    /// Creates a [`ComputePipeline`].
    #[inline]
    pub fn create_compute_pipeline(
        &self,
        desc: &wgpu::ComputePipelineDescriptor,
    ) -> ComputePipeline {
        let wgpu_compute_pipeline = self.device.create_compute_pipeline(desc);
        ComputePipeline::from(wgpu_compute_pipeline)
    }

    /// Creates a [`Buffer`].
    pub fn create_buffer(&self, desc: &wgpu::BufferDescriptor) -> Buffer {
        let wgpu_buffer = self.device.create_buffer(desc);
        Buffer::from(wgpu_buffer)
    }

    /// Creates a [`Buffer`] and initializes it with the specified data.
    pub fn create_buffer_with_data(&self, desc: &wgpu::util::BufferInitDescriptor) -> Buffer {
        let wgpu_buffer = self.device.create_buffer_init(desc);
        Buffer::from(wgpu_buffer)
    }

    /// Creates a new [`Texture`] and initializes it with the specified data.
    ///
    /// `desc` specifies the general format of the texture.
    /// `data` is the raw data.
    pub fn create_texture_with_data(
        &self,
        render_queue: &RenderQueue,
        desc: &wgpu::TextureDescriptor,
        order: wgpu::util::TextureDataOrder,
        data: &[u8],
    ) -> Texture {
        let wgpu_texture =
            self.device
                .create_texture_with_data(render_queue.as_ref(), desc, order, data);
        Texture::from(wgpu_texture)
    }

    /// Creates a new [`Texture`].
    ///
    /// `desc` specifies the general format of the texture.
    pub fn create_texture(&self, desc: &wgpu::TextureDescriptor) -> Texture {
        let wgpu_texture = self.device.create_texture(desc);
        Texture::from(wgpu_texture)
    }

    /// Creates a new [`Sampler`].
    ///
    /// `desc` specifies the behavior of the sampler.
    pub fn create_sampler(&self, desc: &wgpu::SamplerDescriptor) -> Sampler {
        let wgpu_sampler = self.device.create_sampler(desc);
        Sampler::from(wgpu_sampler)
    }

    /// Initializes [`Surface`](wgpu::Surface) for presentation.
    ///
    /// # Panics
    ///
    /// - A old [`SurfaceTexture`](wgpu::SurfaceTexture) is still alive referencing an old surface.
    /// - Texture format requested is unsupported on the surface.
    pub fn configure_surface(&self, surface: &wgpu::Surface, config: &wgpu::SurfaceConfiguration) {
        surface.configure(&self.device, config);
    }

    /// Returns the wgpu [`Device`](wgpu::Device).
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn map_buffer(
        &self,
        buffer: &wgpu::BufferSlice,
        map_mode: wgpu::MapMode,
        callback: impl FnOnce(Result<(), BufferAsyncError>) + Send + 'static,
    ) {
        buffer.map_async(map_mode, callback);
    }

    pub fn align_copy_bytes_per_row(row_bytes: usize) -> usize {
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - row_bytes % align) % align;
        row_bytes + padded_bytes_per_row_padding
    }

    pub fn get_supported_read_only_binding_type(
        &self,
        buffers_per_shader_stage: u32,
    ) -> BufferBindingType {
        if self.limits().max_storage_buffers_per_shader_stage >= buffers_per_shader_stage {
            BufferBindingType::Storage { read_only: true }
        } else {
            BufferBindingType::Uniform
        }
    }
}
