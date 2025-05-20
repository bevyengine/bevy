mod render_device;

use bevy_derive::{Deref, DerefMut};
pub use render_device::*;
use tracing::{error, info, info_span, warn};

use crate::{
    diagnostic::internal::DiagnosticsRecorder,
    frame_graph::{FrameGraph, FrameGraphRunner, TransientResourceCache},
    render_graph::{RenderGraph, RenderGraphRunner},
    render_resource::PipelineCache,
    settings::{WgpuSettings, WgpuSettingsPriority},
    view::{ExtractedWindows, ViewTarget},
};
use alloc::sync::Arc;
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_platform::time::Instant;
use bevy_time::TimeSender;
use wgpu::{Adapter, AdapterInfo, DeviceType, Instance, Queue, RequestAdapterOptions};

pub fn setup_frame_graph_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });

    let Some(mut frame_graph) = world.remove_resource::<FrameGraph>() else {
        return;
    };

    let graph = world.resource::<RenderGraph>();

    let res = RenderGraphRunner::run(graph, &mut frame_graph, world);

    match res {
        Ok(_) => {
            world.insert_resource(frame_graph);
        }
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
}

pub fn compiled_frame_graph_system(mut frame_graph: ResMut<FrameGraph>) {
    frame_graph.compile();
}

pub fn render_system(world: &mut World, state: &mut SystemState<Query<Entity, With<ViewTarget>>>) {
    let diagnostics_recorder = world.remove_resource::<DiagnosticsRecorder>();
    let Some(mut graph) = world.remove_resource::<FrameGraph>() else {
        return;
    };

    let Some(mut transient_resource_cache) = world.remove_resource::<TransientResourceCache>()
    else {
        return;
    };

    let render_device = world.resource::<RenderDevice>();
    let render_queue = world.resource::<RenderQueue>();
    let render_adapter = world.resource::<RenderAdapter>();
    let pipeline_cache = world.resource::<PipelineCache>();

    let res = FrameGraphRunner::run(
        &mut graph,
        render_device.clone(), // TODO: is this clone really necessary?
        &mut transient_resource_cache,
        pipeline_cache,
        diagnostics_recorder,
        &render_queue.0,
        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        &render_adapter.0,
        world,
        |encoder| {
            crate::view::screenshot::submit_screenshot_commands(world, encoder);
            crate::gpu_readback::submit_readback_commands(world, encoder);
        },
    );

    world.insert_resource(graph);
    world.insert_resource(transient_resource_cache);

    match res {
        Some(diagnostics_recorder) => {
            world.insert_resource(diagnostics_recorder);
        }
        None => {}
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

/// A wrapper to safely make `wgpu` types Send / Sync on web with atomics enabled.
///
/// On web with `atomics` enabled the inner value can only be accessed
/// or dropped on the `wgpu` thread or else a panic will occur.
/// On other platforms the wrapper simply contains the wrapped value.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
#[derive(Debug, Clone, Deref, DerefMut)]
pub struct WgpuWrapper<T>(T);
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
#[derive(Debug, Clone, Deref, DerefMut)]
pub struct WgpuWrapper<T>(send_wrapper::SendWrapper<T>);

// SAFETY: SendWrapper is always Send + Sync.
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
unsafe impl<T> Send for WgpuWrapper<T> {}
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
unsafe impl<T> Sync for WgpuWrapper<T> {}

#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
impl<T> WgpuWrapper<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
impl<T> WgpuWrapper<T> {
    pub fn new(t: T) -> Self {
        Self(send_wrapper::SendWrapper::new(t))
    }

    pub fn into_inner(self) -> T {
        self.0.take()
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

/// Initializes the renderer by retrieving and preparing the GPU instance, device and queue
/// for the specified backend.
pub async fn initialize_renderer(
    instance: &Instance,
    options: &WgpuSettings,
    request_adapter_options: &RequestAdapterOptions<'_, '_>,
) -> (RenderDevice, RenderQueue, RenderAdapterInfo, RenderAdapter) {
    let adapter = instance
        .request_adapter(request_adapter_options)
        .await
        .expect(GPU_NOT_FOUND_ERROR_MESSAGE);

    let adapter_info = adapter.get_info();
    info!("{:?}", adapter_info);

    if adapter_info.device_type == DeviceType::Cpu {
        warn!(
            "The selected adapter is using a driver that only supports software rendering. \
             This is likely to be very slow. See https://bevyengine.org/learn/errors/b0006/"
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
            features -= wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
        }

        // RAY_QUERY and RAY_TRACING_ACCELERATION STRUCTURE will sometimes cause DeviceLost failures on platforms
        // that report them as supported:
        // <https://github.com/gfx-rs/wgpu/issues/5488>
        features -= wgpu::Features::EXPERIMENTAL_RAY_QUERY;
        features -= wgpu::Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE;

        limits = adapter.limits();
    }

    // Enforce the disabled features
    if let Some(disabled_features) = options.disabled_features {
        features -= disabled_features;
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
        };
    }

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: options.device_label.as_ref().map(AsRef::as_ref),
                required_features: features,
                required_limits: limits,
                memory_hints: options.memory_hints.clone(),
            },
            options.trace_path.as_deref(),
        )
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
