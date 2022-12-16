pub mod gpu_resource;
pub mod settings;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::gpu_resource::Shader;
}

use crate::{
    gpu_resource::*,
    settings::{WgpuSettings, WgpuSettingsPriority},
};
use bevy_app::{App, Plugin};
use bevy_asset::AddAsset;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::Resource;
use bevy_utils::{tracing::debug, tracing::info};
use bevy_window::Windows;
use std::sync::Arc;
use wgpu::{
    util::DeviceExt, Adapter, AdapterInfo, Backends, BufferAsyncError, BufferBindingType,
    CommandEncoder, Instance, Queue, RequestAdapterOptions,
};

/// Contains the default Bevy GPU abstraction based on wgpu.
#[derive(Default)]
pub struct GpuPlugin {
    pub wgpu_settings: WgpuSettings,
}

impl Plugin for GpuPlugin {
    /// Initializes the the wgpu backend.
    fn build(&self, app: &mut App) {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        if let Some(backends) = self.wgpu_settings.backends {
            let gpu_instance = GpuInstance::new(backends);
            let windows = app.world.resource_mut::<Windows>();

            let surface = windows
                .get_primary()
                .and_then(|window| window.raw_handle())
                .map(|wrapper| unsafe {
                    let handle = wrapper.get_handle();
                    gpu_instance.create_surface(&handle)
                });

            let request_adapter_options = RequestAdapterOptions {
                power_preference: self.wgpu_settings.power_preference,
                compatible_surface: surface.as_ref(),
                ..Default::default()
            };

            let (gpu_device, gpu_queue, gpu_adapter_info, gpu_adapter) =
                futures_lite::future::block_on(initialize_gpu(
                    &gpu_instance,
                    &self.wgpu_settings,
                    &request_adapter_options,
                ));
            debug!("Configured wgpu adapter Limits: {:#?}", gpu_device.limits());
            debug!(
                "Configured wgpu adapter Features: {:#?}",
                gpu_device.features()
            );
            app.insert_resource(gpu_instance)
                .insert_resource(gpu_device)
                .insert_resource(gpu_queue)
                .insert_resource(gpu_adapter)
                .insert_resource(gpu_adapter_info);
        }
    }
}

const GPU_NOT_FOUND_ERROR_MESSAGE: &str = if cfg!(target_os = "linux") {
    "Unable to find a GPU! Make sure you have installed required drivers! For extra information, see: https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md"
} else {
    "Unable to find a GPU! Make sure you have installed required drivers!"
};

/// The GPU instance is used to initialize the [`GpuQueue`] and [`GpuDevice`],
/// as well as to create surfaces.
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct GpuInstance(pub Arc<Instance>);

impl GpuInstance {
    /// Create an new instance of the wgpu API.
    ///
    /// # Arguments
    ///
    /// - `backends` - Controls from which [backends][Backends] wgpu will choose
    ///   during instantiation.
    pub fn new(backends: Backends) -> Self {
        Self(Arc::new(Instance::new(backends)))
    }
}

/// The handle to the physical device being used for rendering.
/// See [`Adapter`] for more info.
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct GpuAdapter(pub Arc<Adapter>);

/// The `AdapterInfo` of the adapter in use by the renderer.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct GpuAdapterInfo(pub AdapterInfo);

/// This queue is used to enqueue tasks for the GPU to execute asynchronously.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct GpuQueue(pub Arc<Queue>);

/// The context with all information required to interact with the GPU.
///
/// The [`GpuDevice`] is used to create render resources and the
/// the [`CommandEncoder`] is used to record a series of GPU operations.
pub struct GpuContext {
    pub gpu_device: GpuDevice,
    pub command_encoder: CommandEncoder,
}

gpu_resource_wrapper!(ErasedGpuDevice, wgpu::Device);

/// This GPU device is responsible for the creation of most rendering and compute resources.
#[derive(Resource, Clone)]
pub struct GpuDevice {
    device: ErasedGpuDevice,
}

impl From<wgpu::Device> for GpuDevice {
    fn from(device: wgpu::Device) -> Self {
        Self {
            device: ErasedGpuDevice::new(device),
        }
    }
}

impl GpuDevice {
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
    /// no-op on the web, device is automatically polled.
    #[inline]
    pub fn poll(&self, maintain: wgpu::Maintain) {
        self.device.poll(maintain);
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
    pub fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor) -> BindGroup {
        let wgpu_bind_group = self.device.create_bind_group(desc);
        BindGroup::from(wgpu_bind_group)
    }

    /// Creates a [`BindGroupLayout`](wgpu::BindGroupLayout).
    #[inline]
    pub fn create_bind_group_layout(
        &self,
        desc: &wgpu::BindGroupLayoutDescriptor,
    ) -> BindGroupLayout {
        BindGroupLayout::from(self.device.create_bind_group_layout(desc))
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
        gpu_queue: &GpuQueue,
        desc: &wgpu::TextureDescriptor,
        data: &[u8],
    ) -> Texture {
        let wgpu_texture = self
            .device
            .create_texture_with_data(gpu_queue.as_ref(), desc, data);
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

/// Initializes the GPU by retrieving and preparing the GPU instance, device and queue
/// for the specified backend.
pub async fn initialize_gpu(
    instance: &Instance,
    settings: &WgpuSettings,
    request_adapter_options: &RequestAdapterOptions<'_>,
) -> (GpuDevice, GpuQueue, GpuAdapterInfo, GpuAdapter) {
    let adapter = instance
        .request_adapter(request_adapter_options)
        .await
        .expect(GPU_NOT_FOUND_ERROR_MESSAGE);

    let adapter_info = adapter.get_info();
    info!("{:?}", adapter_info);

    #[cfg(feature = "wgpu_trace")]
    let trace_path = {
        let path = std::path::Path::new("wgpu_trace");
        // ignore potential error, wgpu will log it
        let _ = std::fs::create_dir(path);
        Some(path)
    };
    #[cfg(not(feature = "wgpu_trace"))]
    let trace_path = None;

    // Maybe get features and limits based on what is supported by the adapter/backend
    let mut features = wgpu::Features::empty();
    let mut limits = settings.limits.clone();
    if matches!(settings.priority, WgpuSettingsPriority::Functionality) {
        features = adapter.features() | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        if adapter_info.device_type == wgpu::DeviceType::DiscreteGpu {
            // `MAPPABLE_PRIMARY_BUFFERS` can have a significant, negative performance impact for
            // discrete GPUs due to having to transfer data across the PCI-E bus and so it
            // should not be automatically enabled in this case. It is however beneficial for
            // integrated GPUs.
            features -= wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
        }
        limits = adapter.limits();
    }

    // Enforce the disabled features
    if let Some(disabled_features) = settings.disabled_features {
        features -= disabled_features;
    }
    // NOTE: |= is used here to ensure that any explicitly-enabled features are respected.
    features |= settings.features;

    // Enforce the limit constraints
    if let Some(constrained_limits) = settings.constrained_limits.as_ref() {
        // NOTE: Respect the configured limits as an 'upper bound'. This means for 'max' limits, we
        // take the minimum of the calculated limits according to the adapter/backend and the
        // specified max_limits. For 'min' limits, take the maximum instead. This is intended to
        // err on the side of being conservative. We can't claim 'higher' limits that are supported
        // but we can constrain to 'lower' limits.
        limits = wgpu::Limits {
            max_texture_dimension_1d: limits
                .max_texture_dimension_1d
                .min(constrained_limits.max_texture_dimension_1d),
            max_texture_dimension_2d: limits
                .max_texture_dimension_2d
                .min(constrained_limits.max_texture_dimension_2d),
            max_texture_dimension_3d: limits
                .max_texture_dimension_3d
                .min(constrained_limits.max_texture_dimension_3d),
            max_texture_array_layers: limits
                .max_texture_array_layers
                .min(constrained_limits.max_texture_array_layers),
            max_bind_groups: limits
                .max_bind_groups
                .min(constrained_limits.max_bind_groups),
            max_dynamic_uniform_buffers_per_pipeline_layout: limits
                .max_dynamic_uniform_buffers_per_pipeline_layout
                .min(constrained_limits.max_dynamic_uniform_buffers_per_pipeline_layout),
            max_dynamic_storage_buffers_per_pipeline_layout: limits
                .max_dynamic_storage_buffers_per_pipeline_layout
                .min(constrained_limits.max_dynamic_storage_buffers_per_pipeline_layout),
            max_sampled_textures_per_shader_stage: limits
                .max_sampled_textures_per_shader_stage
                .min(constrained_limits.max_sampled_textures_per_shader_stage),
            max_samplers_per_shader_stage: limits
                .max_samplers_per_shader_stage
                .min(constrained_limits.max_samplers_per_shader_stage),
            max_storage_buffers_per_shader_stage: limits
                .max_storage_buffers_per_shader_stage
                .min(constrained_limits.max_storage_buffers_per_shader_stage),
            max_storage_textures_per_shader_stage: limits
                .max_storage_textures_per_shader_stage
                .min(constrained_limits.max_storage_textures_per_shader_stage),
            max_uniform_buffers_per_shader_stage: limits
                .max_uniform_buffers_per_shader_stage
                .min(constrained_limits.max_uniform_buffers_per_shader_stage),
            max_uniform_buffer_binding_size: limits
                .max_uniform_buffer_binding_size
                .min(constrained_limits.max_uniform_buffer_binding_size),
            max_storage_buffer_binding_size: limits
                .max_storage_buffer_binding_size
                .min(constrained_limits.max_storage_buffer_binding_size),
            max_vertex_buffers: limits
                .max_vertex_buffers
                .min(constrained_limits.max_vertex_buffers),
            max_vertex_attributes: limits
                .max_vertex_attributes
                .min(constrained_limits.max_vertex_attributes),
            max_vertex_buffer_array_stride: limits
                .max_vertex_buffer_array_stride
                .min(constrained_limits.max_vertex_buffer_array_stride),
            max_push_constant_size: limits
                .max_push_constant_size
                .min(constrained_limits.max_push_constant_size),
            min_uniform_buffer_offset_alignment: limits
                .min_uniform_buffer_offset_alignment
                .max(constrained_limits.min_uniform_buffer_offset_alignment),
            min_storage_buffer_offset_alignment: limits
                .min_storage_buffer_offset_alignment
                .max(constrained_limits.min_storage_buffer_offset_alignment),
            max_inter_stage_shader_components: limits
                .max_inter_stage_shader_components
                .min(constrained_limits.max_inter_stage_shader_components),
            max_compute_workgroup_storage_size: limits
                .max_compute_workgroup_storage_size
                .min(constrained_limits.max_compute_workgroup_storage_size),
            max_compute_invocations_per_workgroup: limits
                .max_compute_invocations_per_workgroup
                .min(constrained_limits.max_compute_invocations_per_workgroup),
            max_compute_workgroup_size_x: limits
                .max_compute_workgroup_size_x
                .min(constrained_limits.max_compute_workgroup_size_x),
            max_compute_workgroup_size_y: limits
                .max_compute_workgroup_size_y
                .min(constrained_limits.max_compute_workgroup_size_y),
            max_compute_workgroup_size_z: limits
                .max_compute_workgroup_size_z
                .min(constrained_limits.max_compute_workgroup_size_z),
            max_compute_workgroups_per_dimension: limits
                .max_compute_workgroups_per_dimension
                .min(constrained_limits.max_compute_workgroups_per_dimension),
            max_buffer_size: limits
                .max_buffer_size
                .min(constrained_limits.max_buffer_size),
        };
    }

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: settings.device_label.as_ref().map(|a| a.as_ref()),
                features,
                limits,
            },
            trace_path,
        )
        .await
        .unwrap();
    let queue = Arc::new(queue);
    let adapter = Arc::new(adapter);
    (
        GpuDevice::from(device),
        GpuQueue(queue),
        GpuAdapterInfo(adapter_info),
        GpuAdapter(adapter),
    )
}
