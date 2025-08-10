mod graph_runner;
mod render_device;

use crate::WgpuWrapper;
use bevy_derive::{Deref, DerefMut};
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
use bevy_tasks::ComputeTaskPool;
pub use graph_runner::*;
pub use render_device::*;
use tracing::{debug, error, info, info_span, warn};

use crate::{
    diagnostic::{internal::DiagnosticsRecorder, RecordDiagnostics},
    render_graph::RenderGraph,
    render_phase::TrackedRenderPass,
    render_resource::RenderPassDescriptor,
    settings::{WgpuSettings, WgpuSettingsPriority},
    view::{ExtractedWindows, ViewTarget},
};
use alloc::sync::Arc;
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_platform::time::Instant;
use bevy_time::TimeSender;
use wgpu::{
    Adapter, AdapterInfo, CommandBuffer, CommandEncoder, DeviceType, Instance, Queue,
    RequestAdapterOptions, Trace,
};

/// Updates the [`RenderGraph`] with all of its nodes and then runs it to render the entire frame.
pub fn render_system(world: &mut World, state: &mut SystemState<Query<Entity, With<ViewTarget>>>) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });

    let diagnostics_recorder = world.remove_resource::<DiagnosticsRecorder>();

    let graph = world.resource::<RenderGraph>();
    let render_device = world.resource::<RenderDevice>();
    let render_queue = world.resource::<RenderQueue>();

    let res = RenderGraphRunner::run(
        graph,
        render_device.clone(), // TODO: is this clone really necessary?
        diagnostics_recorder,
        &render_queue.0,
        world,
        |encoder| {
            crate::view::screenshot::submit_screenshot_commands(world, encoder);
            crate::gpu_readback::submit_readback_commands(world, encoder);
        },
    );

    match res {
        Ok(Some(diagnostics_recorder)) => {
            world.insert_resource(diagnostics_recorder);
        }
        Ok(None) => {}
        Err(e) => {
            error!("Error running render graph:");
            {
                let mut src: &dyn core::error::Error = &e;
                loop {
                    error!("> {}", src);
                    match src.source() {
                        Some(s) => src = s,
                        None => break,
                    }
                }
            }

            panic!("Error running render graph: {e}");
        }
    }

    {
        let _span = info_span!("present_frames").entered();

        // Remove ViewTarget components to ensure swap chain TextureViews are dropped.
        // If all TextureViews aren't dropped before present, acquiring the next swap chain texture will fail.
        let view_entities = state.get(world).iter().collect::<Vec<_>>();
        for view_entity in view_entities {
            world.entity_mut(view_entity).remove::<ViewTarget>();
        }

        let mut windows = world.resource_mut::<ExtractedWindows>();
        for window in windows.values_mut() {
            if let Some(surface_texture) = window.swap_chain_texture.take() {
                // TODO(clean): winit docs recommends calling pre_present_notify before this.
                // though `present()` doesn't present the frame, it schedules it to be presented
                // by wgpu.
                // https://docs.rs/winit/0.29.9/wasm32-unknown-unknown/winit/window/struct.Window.html#method.pre_present_notify
                surface_texture.present();
            }
        }

        #[cfg(feature = "tracing-tracy")]
        tracing::event!(
            tracing::Level::INFO,
            message = "finished frame",
            tracy.frame_mark = true
        );
    }

    crate::view::screenshot::collect_screenshots(world);

    // update the time and send it to the app world
    let time_sender = world.resource::<TimeSender>();
    if let Err(error) = time_sender.0.try_send(Instant::now()) {
        match error {
            bevy_time::TrySendError::Full(_) => {
                panic!("The TimeSender channel should always be empty during render. You might need to add the bevy::core::time_system to your app.",);
            }
            bevy_time::TrySendError::Disconnected(_) => {
                // ignore disconnected errors, the main world probably just got dropped during shutdown
            }
        }
    }
}

/// This queue is used to enqueue tasks for the GPU to execute asynchronously.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderQueue(pub Arc<WgpuWrapper<Queue>>);

/// The handle to the physical device being used for rendering.
/// See [`Adapter`] for more info.
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct RenderAdapter(pub Arc<WgpuWrapper<Adapter>>);

/// The GPU instance is used to initialize the [`RenderQueue`] and [`RenderDevice`],
/// as well as to create [`WindowSurfaces`](crate::view::window::WindowSurfaces).
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderInstance(pub Arc<WgpuWrapper<Instance>>);

/// The [`AdapterInfo`] of the adapter in use by the renderer.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderAdapterInfo(pub WgpuWrapper<AdapterInfo>);

const GPU_NOT_FOUND_ERROR_MESSAGE: &str = if cfg!(target_os = "linux") {
    "Unable to find a GPU! Make sure you have installed required drivers! For extra information, see: https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md"
} else {
    "Unable to find a GPU! Make sure you have installed required drivers!"
};

#[cfg(not(target_family = "wasm"))]
fn find_adapter_by_name(
    instance: &Instance,
    options: &WgpuSettings,
    compatible_surface: Option<&wgpu::Surface<'_>>,
    adapter_name: &str,
) -> Option<Adapter> {
    for adapter in
        instance.enumerate_adapters(options.backends.expect(
            "The `backends` field of `WgpuSettings` must be set to use a specific adapter.",
        ))
    {
        tracing::trace!("Checking adapter: {:?}", adapter.get_info());
        let info = adapter.get_info();
        if let Some(surface) = compatible_surface
            && !adapter.is_surface_supported(surface)
        {
            continue;
        }

        if info.name.eq_ignore_ascii_case(adapter_name) {
            return Some(adapter);
        }
    }
    None
}

/// Initializes the renderer by retrieving and preparing the GPU instance, device and queue
/// for the specified backend.
pub async fn initialize_renderer(
    instance: &Instance,
    options: &WgpuSettings,
    request_adapter_options: &RequestAdapterOptions<'_, '_>,
    desired_adapter_name: Option<String>,
) -> (RenderDevice, RenderQueue, RenderAdapterInfo, RenderAdapter) {
    #[cfg(not(target_family = "wasm"))]
    let mut selected_adapter = desired_adapter_name.and_then(|adapter_name| {
        find_adapter_by_name(
            instance,
            options,
            request_adapter_options.compatible_surface,
            &adapter_name,
        )
    });
    #[cfg(target_family = "wasm")]
    let mut selected_adapter = None;

    #[cfg(target_family = "wasm")]
    if desired_adapter_name.is_some() {
        warn!("Choosing an adapter is not supported on wasm.");
    }

    if selected_adapter.is_none() {
        debug!(
            "Searching for adapter with options: {:?}",
            request_adapter_options
        );
        selected_adapter = instance.request_adapter(request_adapter_options).await.ok();
    }

    let adapter = selected_adapter.expect(GPU_NOT_FOUND_ERROR_MESSAGE);
    let adapter_info = adapter.get_info();
    info!("{:?}", adapter_info);

    if adapter_info.device_type == DeviceType::Cpu {
        warn!(
            "The selected adapter is using a driver that only supports software rendering. \
             This is likely to be very slow. See https://bevy.org/learn/errors/b0006/"
        );
    }

    // Maybe get features and limits based on what is supported by the adapter/backend
    let mut features = wgpu::Features::empty();
    let mut limits = options.limits.clone();
    if matches!(options.priority, WgpuSettingsPriority::Functionality) {
        features = adapter.features();
        if adapter_info.device_type == DeviceType::DiscreteGpu {
            // `MAPPABLE_PRIMARY_BUFFERS` can have a significant, negative performance impact for
            // discrete GPUs due to having to transfer data across the PCI-E bus and so it
            // should not be automatically enabled in this case. It is however beneficial for
            // integrated GPUs.
            features.remove(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS);
        }

        limits = adapter.limits();
    }

    // Enforce the disabled features
    if let Some(disabled_features) = options.disabled_features {
        features.remove(disabled_features);
    }
    // NOTE: |= is used here to ensure that any explicitly-enabled features are respected.
    features |= options.features;

    // Enforce the limit constraints
    if let Some(constrained_limits) = options.constrained_limits.as_ref() {
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
            max_binding_array_elements_per_shader_stage: limits
                .max_binding_array_elements_per_shader_stage
                .min(constrained_limits.max_binding_array_elements_per_shader_stage),
            max_binding_array_sampler_elements_per_shader_stage: limits
                .max_binding_array_sampler_elements_per_shader_stage
                .min(constrained_limits.max_binding_array_sampler_elements_per_shader_stage),
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
            max_bindings_per_bind_group: limits
                .max_bindings_per_bind_group
                .min(constrained_limits.max_bindings_per_bind_group),
            max_non_sampler_bindings: limits
                .max_non_sampler_bindings
                .min(constrained_limits.max_non_sampler_bindings),
            max_blas_primitive_count: limits
                .max_blas_primitive_count
                .min(constrained_limits.max_blas_primitive_count),
            max_blas_geometry_count: limits
                .max_blas_geometry_count
                .min(constrained_limits.max_blas_geometry_count),
            max_tlas_instance_count: limits
                .max_tlas_instance_count
                .min(constrained_limits.max_tlas_instance_count),
            max_color_attachments: limits
                .max_color_attachments
                .min(constrained_limits.max_color_attachments),
            max_color_attachment_bytes_per_sample: limits
                .max_color_attachment_bytes_per_sample
                .min(constrained_limits.max_color_attachment_bytes_per_sample),
            min_subgroup_size: limits
                .min_subgroup_size
                .max(constrained_limits.min_subgroup_size),
            max_subgroup_size: limits
                .max_subgroup_size
                .min(constrained_limits.max_subgroup_size),
            max_acceleration_structures_per_shader_stage: 0,
        };
    }

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: options.device_label.as_ref().map(AsRef::as_ref),
            required_features: features,
            required_limits: limits,
            memory_hints: options.memory_hints.clone(),
            // See https://github.com/gfx-rs/wgpu/issues/5974
            trace: Trace::Off,
        })
        .await
        .unwrap();
    let queue = Arc::new(WgpuWrapper::new(queue));
    let adapter = Arc::new(WgpuWrapper::new(adapter));
    (
        RenderDevice::from(device),
        RenderQueue(queue),
        RenderAdapterInfo(WgpuWrapper::new(adapter_info)),
        RenderAdapter(adapter),
    )
}

/// The context with all information required to interact with the GPU.
///
/// The [`RenderDevice`] is used to create render resources and the
/// the [`CommandEncoder`] is used to record a series of GPU operations.
pub struct RenderContext<'w> {
    render_device: RenderDevice,
    command_encoder: Option<CommandEncoder>,
    command_buffer_queue: Vec<QueuedCommandBuffer<'w>>,
    diagnostics_recorder: Option<Arc<DiagnosticsRecorder>>,
}

impl<'w> RenderContext<'w> {
    /// Creates a new [`RenderContext`] from a [`RenderDevice`].
    pub fn new(
        render_device: RenderDevice,
        diagnostics_recorder: Option<DiagnosticsRecorder>,
    ) -> Self {
        Self {
            render_device,
            command_encoder: None,
            command_buffer_queue: Vec::new(),
            diagnostics_recorder: diagnostics_recorder.map(Arc::new),
        }
    }

    /// Gets the underlying [`RenderDevice`].
    pub fn render_device(&self) -> &RenderDevice {
        &self.render_device
    }

    /// Gets the diagnostics recorder, used to track elapsed time and pipeline statistics
    /// of various render and compute passes.
    pub fn diagnostic_recorder(&self) -> impl RecordDiagnostics + use<> {
        self.diagnostics_recorder.clone()
    }

    /// Gets the current [`CommandEncoder`].
    pub fn command_encoder(&mut self) -> &mut CommandEncoder {
        self.command_encoder.get_or_insert_with(|| {
            self.render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        })
    }

    /// Creates a new [`TrackedRenderPass`] for the context,
    /// configured using the provided `descriptor`.
    pub fn begin_tracked_render_pass<'a>(
        &'a mut self,
        descriptor: RenderPassDescriptor<'_>,
    ) -> TrackedRenderPass<'a> {
        // Cannot use command_encoder() as we need to split the borrow on self
        let command_encoder = self.command_encoder.get_or_insert_with(|| {
            self.render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        });

        let render_pass = command_encoder.begin_render_pass(&descriptor);
        TrackedRenderPass::new(&self.render_device, render_pass)
    }

    /// Append a [`CommandBuffer`] to the command buffer queue.
    ///
    /// If present, this will flush the currently unflushed [`CommandEncoder`]
    /// into a [`CommandBuffer`] into the queue before appending the provided
    /// buffer.
    pub fn add_command_buffer(&mut self, command_buffer: CommandBuffer) {
        self.flush_encoder();

        self.command_buffer_queue
            .push(QueuedCommandBuffer::Ready(command_buffer));
    }

    /// Append a function that will generate a [`CommandBuffer`] to the
    /// command buffer queue, to be ran later.
    ///
    /// If present, this will flush the currently unflushed [`CommandEncoder`]
    /// into a [`CommandBuffer`] into the queue before appending the provided
    /// buffer.
    pub fn add_command_buffer_generation_task(
        &mut self,
        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        task: impl FnOnce(RenderDevice) -> CommandBuffer + 'w + Send,
        #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
        task: impl FnOnce(RenderDevice) -> CommandBuffer + 'w,
    ) {
        self.flush_encoder();

        self.command_buffer_queue
            .push(QueuedCommandBuffer::Task(Box::new(task)));
    }

    /// Finalizes and returns the queue of [`CommandBuffer`]s.
    ///
    /// This function will wait until all command buffer generation tasks are complete
    /// by running them in parallel (where supported).
    ///
    /// The [`CommandBuffer`]s will be returned in the order that they were added.
    pub fn finish(
        mut self,
    ) -> (
        Vec<CommandBuffer>,
        RenderDevice,
        Option<DiagnosticsRecorder>,
    ) {
        self.flush_encoder();

        let mut command_buffers = Vec::with_capacity(self.command_buffer_queue.len());

        #[cfg(feature = "trace")]
        let _command_buffer_generation_tasks_span =
            info_span!("command_buffer_generation_tasks").entered();

        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        {
            let mut task_based_command_buffers = ComputeTaskPool::get().scope(|task_pool| {
                for (i, queued_command_buffer) in self.command_buffer_queue.into_iter().enumerate()
                {
                    match queued_command_buffer {
                        QueuedCommandBuffer::Ready(command_buffer) => {
                            command_buffers.push((i, command_buffer));
                        }
                        QueuedCommandBuffer::Task(command_buffer_generation_task) => {
                            let render_device = self.render_device.clone();
                            task_pool.spawn(async move {
                                (i, command_buffer_generation_task(render_device))
                            });
                        }
                    }
                }
            });
            command_buffers.append(&mut task_based_command_buffers);
        }

        #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
        for (i, queued_command_buffer) in self.command_buffer_queue.into_iter().enumerate() {
            match queued_command_buffer {
                QueuedCommandBuffer::Ready(command_buffer) => {
                    command_buffers.push((i, command_buffer));
                }
                QueuedCommandBuffer::Task(command_buffer_generation_task) => {
                    let render_device = self.render_device.clone();
                    command_buffers.push((i, command_buffer_generation_task(render_device)));
                }
            }
        }

        #[cfg(feature = "trace")]
        drop(_command_buffer_generation_tasks_span);

        command_buffers.sort_unstable_by_key(|(i, _)| *i);

        let mut command_buffers = command_buffers
            .into_iter()
            .map(|(_, cb)| cb)
            .collect::<Vec<CommandBuffer>>();

        let mut diagnostics_recorder = self.diagnostics_recorder.take().map(|v| {
            Arc::try_unwrap(v)
                .ok()
                .expect("diagnostic recorder shouldn't be held longer than necessary")
        });

        if let Some(recorder) = &mut diagnostics_recorder {
            let mut command_encoder = self
                .render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            recorder.resolve(&mut command_encoder);
            command_buffers.push(command_encoder.finish());
        }

        (command_buffers, self.render_device, diagnostics_recorder)
    }

    fn flush_encoder(&mut self) {
        if let Some(encoder) = self.command_encoder.take() {
            self.command_buffer_queue
                .push(QueuedCommandBuffer::Ready(encoder.finish()));
        }
    }
}

enum QueuedCommandBuffer<'w> {
    Ready(CommandBuffer),
    #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
    Task(Box<dyn FnOnce(RenderDevice) -> CommandBuffer + 'w + Send>),
    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    Task(Box<dyn FnOnce(RenderDevice) -> CommandBuffer + 'w>),
}
