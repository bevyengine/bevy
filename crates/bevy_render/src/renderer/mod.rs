mod graph_runner;
mod render_device;

use bevy_derive::{Deref, DerefMut};
use bevy_utils::tracing::{error, info, info_span};
pub use graph_runner::*;
pub use render_device::*;

use crate::{
    render_graph::RenderGraph,
    settings::{WgpuSettings, WgpuSettingsPriority},
    view::{ExtractedWindows, ViewTarget},
};
use bevy_ecs::prelude::*;
use bevy_time::TimeSender;
use bevy_utils::Instant;
use std::sync::Arc;
use wgpu::{AdapterInfo, CommandEncoder, Instance, Queue, RequestAdapterOptions};

/// Updates the [`RenderGraph`] with all of its nodes and then runs it to render the entire frame.
pub fn render_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });
    let graph = world.resource::<RenderGraph>();
    let render_device = world.resource::<RenderDevice>();
    let render_queue = world.resource::<RenderQueue>();

    if let Err(e) = RenderGraphRunner::run(
        graph,
        render_device.clone(), // TODO: is this clone really necessary?
        &render_queue.0,
        world,
    ) {
        error!("Error running render graph:");
        {
            let mut src: &dyn std::error::Error = &e;
            loop {
                error!("> {}", src);
                match src.source() {
                    Some(s) => src = s,
                    None => break,
                }
            }
        }

        panic!("Error running render graph: {}", e);
    }

    {
        let _span = info_span!("present_frames").entered();

        // Remove ViewTarget components to ensure swap chain TextureViews are dropped.
        // If all TextureViews aren't dropped before present, acquiring the next swap chain texture will fail.
        let view_entities = world
            .query_filtered::<Entity, With<ViewTarget>>()
            .iter(world)
            .collect::<Vec<_>>();
        for view_entity in view_entities {
            world.entity_mut(view_entity).remove::<ViewTarget>();
        }

        let mut windows = world.resource_mut::<ExtractedWindows>();
        for window in windows.values_mut() {
            if let Some(texture_view) = window.swap_chain_texture.take() {
                if let Some(surface_texture) = texture_view.take_surface_texture() {
                    surface_texture.present();
                }
            }
        }

        #[cfg(feature = "tracing-tracy")]
        bevy_utils::tracing::event!(
            bevy_utils::tracing::Level::INFO,
            message = "finished frame",
            tracy.frame_mark = true
        );
    }

    // update the time and send it to the app world
    let time_sender = world.resource::<TimeSender>();
    time_sender.0.try_send(Instant::now()).expect(
        "The TimeSender channel should always be empty during render. You might need to add the bevy::core::time_system to your app.",
    );
}

/// This queue is used to enqueue tasks for the GPU to execute asynchronously.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderQueue(pub Arc<Queue>);

/// The GPU instance is used to initialize the [`RenderQueue`] and [`RenderDevice`],
/// as well as to create [`WindowSurfaces`](crate::view::window::WindowSurfaces).
#[derive(Resource, Deref, DerefMut)]
pub struct RenderInstance(pub Instance);

/// The `AdapterInfo` of the adapter in use by the renderer.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderAdapterInfo(pub AdapterInfo);

/// Initializes the renderer by retrieving and preparing the GPU instance, device and queue
/// for the specified backend.
pub async fn initialize_renderer(
    instance: &Instance,
    options: &WgpuSettings,
    request_adapter_options: &RequestAdapterOptions<'_>,
) -> (RenderDevice, RenderQueue, RenderAdapterInfo) {
    let adapter = instance
        .request_adapter(request_adapter_options)
        .await
        .expect("Unable to find a GPU! Make sure you have installed required drivers!");

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
    let mut limits = options.limits.clone();
    if matches!(options.priority, WgpuSettingsPriority::Functionality) {
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
        };
    }

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: options.device_label.as_ref().map(|a| a.as_ref()),
                features,
                limits,
            },
            trace_path,
        )
        .await
        .unwrap();
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    (
        RenderDevice::from(device),
        RenderQueue(queue),
        RenderAdapterInfo(adapter_info),
    )
}

/// The context with all information required to interact with the GPU.
///
/// The [`RenderDevice`] is used to create render resources and the
/// the [`CommandEncoder`] is used to record a series of GPU operations.
pub struct RenderContext {
    pub render_device: RenderDevice,
    pub command_encoder: CommandEncoder,
}
