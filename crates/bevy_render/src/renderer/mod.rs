#[cfg(feature = "raw_vulkan_init")]
pub mod raw_vulkan_init;
mod render_context;
mod render_device;
mod wgpu_wrapper;

pub use render_context::{
    CurrentView, CurrentViewEntity, FlushCommands, PendingCommandBuffers, RenderContext,
    RenderContextState, ViewQuery,
};
pub use render_device::*;
pub use wgpu_wrapper::WgpuWrapper;

use crate::{
    settings::{RenderResources, WgpuSettings, WgpuSettingsPriority},
    view::{ExtractedWindows, ViewTarget},
};
use alloc::sync::Arc;
use bevy_camera::NormalizedRenderTarget;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_platform::time::Instant;
use bevy_render::camera::ExtractedCamera;
use bevy_time::TimeSender;
use bevy_window::RawHandleWrapperHolder;
use tracing::{debug, info, info_span, warn};
use wgpu::{
    Adapter, AdapterInfo, Backends, DeviceType, Instance, Queue, RequestAdapterOptions, Trace,
};

/// Schedule label for the root render graph schedule. This schedule runs once per frame
/// in the [`render_system`] system and is responsible for driving the entire rendering process.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RenderGraph;

impl RenderGraph {
    pub fn base_schedule() -> Schedule {
        Schedule::new(Self)
    }
}

/// The main render system that drives the rendering process. This system runs the [`RenderGraph`]
/// schedule, runs any finalization commands like screenshot captures and GPU readbacks, and
/// calls present on swap chains that need to be presented.
pub fn render_system(
    world: &mut World,
    state: &mut SystemState<Query<(&ViewTarget, &ExtractedCamera)>>,
) {
    #[cfg(feature = "trace")]
    let _span = info_span!("main_render_schedule").entered();

    world.run_schedule(RenderGraph);

    {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let mut encoder =
            render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        crate::view::screenshot::submit_screenshot_commands(world, &mut encoder);
        crate::gpu_readback::submit_readback_commands(world, &mut encoder);

        render_queue.submit([encoder.finish()]);
    }

    {
        let _span = info_span!("present_frames").entered();

        world.resource_scope(|world, mut windows: Mut<ExtractedWindows>| {
            let views = state.get(world);
            for window in windows.values_mut() {
                let view_needs_present = views.iter().any(|(view_target, camera)| {
                    matches!(
                        camera.target,
                        Some(NormalizedRenderTarget::Window(w)) if w.entity() == window.entity
                    ) && view_target.needs_present()
                });

                if view_needs_present || window.needs_initial_present {
                    window.present();
                    window.needs_initial_present = false;
                }
            }
        });

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
                panic!("The TimeSender channel should always be empty during render. You might need to add the bevy::core::time_system to your app.");
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

        if info
            .name
            .to_lowercase()
            .contains(&adapter_name.to_lowercase())
        {
            return Some(adapter);
        }
    }
    None
}

/// Initializes the renderer by retrieving and preparing the GPU instance, device and queue
/// for the specified backend.
pub async fn initialize_renderer(
    backends: Backends,
    primary_window: Option<RawHandleWrapperHolder>,
    options: &WgpuSettings,
    #[cfg(feature = "raw_vulkan_init")]
    raw_vulkan_init_settings: raw_vulkan_init::RawVulkanInitSettings,
) -> RenderResources {
    let instance_descriptor = wgpu::InstanceDescriptor {
        backends,
        flags: options.instance_flags,
        memory_budget_thresholds: options.instance_memory_budget_thresholds,
        backend_options: wgpu::BackendOptions {
            gl: wgpu::GlBackendOptions {
                gles_minor_version: options.gles3_minor_version,
                fence_behavior: wgpu::GlFenceBehavior::Normal,
            },
            dx12: wgpu::Dx12BackendOptions {
                shader_compiler: options.dx12_shader_compiler.clone(),
                presentation_system: wgpu::wgt::Dx12SwapchainKind::from_env().unwrap_or_default(),
                latency_waitable_object: wgpu::wgt::Dx12UseFrameLatencyWaitableObject::from_env()
                    .unwrap_or_default(),
            },
            noop: wgpu::NoopBackendOptions { enable: false },
        },
    };

    #[cfg(not(feature = "raw_vulkan_init"))]
    let instance = Instance::new(&instance_descriptor);
    #[cfg(feature = "raw_vulkan_init")]
    let mut additional_vulkan_features = raw_vulkan_init::AdditionalVulkanFeatures::default();
    #[cfg(feature = "raw_vulkan_init")]
    let instance = raw_vulkan_init::create_raw_vulkan_instance(
        &instance_descriptor,
        &raw_vulkan_init_settings,
        &mut additional_vulkan_features,
    );

    let surface = primary_window.and_then(|wrapper| {
        let maybe_handle = wrapper
            .0
            .lock()
            .expect("Couldn't get the window handle in time for renderer initialization");
        if let Some(wrapper) = maybe_handle.as_ref() {
            // SAFETY: Plugins should be set up on the main thread.
            let handle = unsafe { wrapper.get_handle() };
            Some(
                instance
                    .create_surface(handle)
                    .expect("Failed to create wgpu surface"),
            )
        } else {
            None
        }
    });

    let force_fallback_adapter = std::env::var("WGPU_FORCE_FALLBACK_ADAPTER")
        .map_or(options.force_fallback_adapter, |v| {
            !(v.is_empty() || v == "0" || v == "false")
        });

    let desired_adapter_name = std::env::var("WGPU_ADAPTER_NAME")
        .as_deref()
        .map_or(options.adapter_name.clone(), |x| Some(x.to_lowercase()));

    let request_adapter_options = RequestAdapterOptions {
        power_preference: options.power_preference,
        compatible_surface: surface.as_ref(),
        force_fallback_adapter,
    };

    #[cfg(not(target_family = "wasm"))]
    let mut selected_adapter = desired_adapter_name.and_then(|adapter_name| {
        find_adapter_by_name(
            &instance,
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
        selected_adapter = instance
            .request_adapter(&request_adapter_options)
            .await
            .ok();
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
            max_acceleration_structures_per_shader_stage: limits
                .max_acceleration_structures_per_shader_stage
                .min(constrained_limits.max_acceleration_structures_per_shader_stage),
            max_task_workgroup_total_count: limits
                .max_task_workgroup_total_count
                .min(constrained_limits.max_task_workgroup_total_count),
            max_task_workgroups_per_dimension: limits
                .max_task_workgroups_per_dimension
                .min(constrained_limits.max_task_workgroups_per_dimension),
            max_mesh_output_layers: limits
                .max_mesh_output_layers
                .min(constrained_limits.max_mesh_output_layers),
            max_mesh_multiview_count: limits
                .max_mesh_multiview_count
                .min(constrained_limits.max_mesh_multiview_count),
        };
    }

    let device_descriptor = wgpu::DeviceDescriptor {
        label: options.device_label.as_ref().map(AsRef::as_ref),
        required_features: features,
        required_limits: limits,
        // SAFETY: TODO, see https://github.com/bevyengine/bevy/issues/22082
        experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
        memory_hints: options.memory_hints.clone(),
        // See https://github.com/gfx-rs/wgpu/issues/5974
        trace: Trace::Off,
    };

    #[cfg(not(feature = "raw_vulkan_init"))]
    let (device, queue) = adapter.request_device(&device_descriptor).await.unwrap();

    #[cfg(feature = "raw_vulkan_init")]
    let (device, queue) = raw_vulkan_init::create_raw_device(
        &adapter,
        &device_descriptor,
        &raw_vulkan_init_settings,
        &mut additional_vulkan_features,
    )
    .await
    .unwrap();

    debug!("Configured wgpu adapter Limits: {:#?}", device.limits());
    debug!("Configured wgpu adapter Features: {:#?}", device.features());

    RenderResources(
        RenderDevice::from(device),
        RenderQueue(Arc::new(WgpuWrapper::new(queue))),
        RenderAdapterInfo(WgpuWrapper::new(adapter_info)),
        RenderAdapter(Arc::new(WgpuWrapper::new(adapter))),
        RenderInstance(Arc::new(WgpuWrapper::new(instance))),
        #[cfg(feature = "raw_vulkan_init")]
        additional_vulkan_features,
    )
}
