//! Functionality relevant to GPU occlusion culling.
//!
//! Currently, there's no support for GPU occlusion culling in Bevy; however,
//! these functions lay the groundwork for one.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_color::LinearRgba;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryItem, With, Without},
    schedule::IntoSystemConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::{UVec2, Vec4Swizzles as _};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_phase::BinnedRenderPhase,
    render_resource::{
        binding_types, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, CommandEncoderDescriptor, Extent3d,
        FragmentState, LoadOp, MultisampleState, Operations, PipelineCache, PrimitiveState,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
        SamplerBindingType, SamplerDescriptor, Shader, ShaderStages, StoreOp, TextureAspect,
        TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
        TextureView, TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, ViewDepthTexture},
    Render, RenderApp, RenderSet,
};
use bevy_utils::{prelude::default, previous_power_of_2};

use crate::{
    core_3d::graph::{Core3d, Node3d},
    fullscreen_vertex_shader,
    prepass::{node::PrepassRunner, AlphaMask3dPrepass, Opaque3dPrepass},
};

pub const DOWNSAMPLE_DEPTH_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(11295947011526841734);
pub const RESOLVE_DEPTH_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(15894811689345116803);

/// Supplies functionality relating to GPU occlusion culling.
///
/// Bevy doesn't currently support GPU occlusion culling outside of meshlets,
/// but this functionality may be useful for those wishing to implement their
/// own occlusion culling systems.
pub struct OcclusionCullingPlugin;

impl Plugin for OcclusionCullingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            DOWNSAMPLE_DEPTH_SHADER_HANDLE,
            "downsample_depth.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            RESOLVE_DEPTH_SHADER_HANDLE,
            "resolve_depth.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(ExtractComponentPlugin::<HierarchicalDepthBuffer>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DownsampleDepthPipelineId>()
            .init_resource::<ResolveDepthPipelineId>()
            .add_systems(
                Render,
                (
                    (
                        prepare_downsample_depth_pipeline,
                        prepare_resolve_depth_pipeline,
                    )
                        .in_set(RenderSet::Prepare),
                    prepare_culling_view_resources.in_set(RenderSet::PrepareBindGroups),
                ),
            );
        render_app
            .add_render_graph_node::<ViewNodeRunner<EarlyDownsampleDepthBufferNode>>(
                Core3d,
                Node3d::EarlyDownsampleDepthBuffer,
            )
            .add_render_graph_node::<ViewNodeRunner<LateDownsampleDepthBufferNode>>(
                Core3d,
                Node3d::LateDownsampleDepthBuffer,
            )
            .add_render_graph_node::<ViewNodeRunner<OcclusionCullingDepthPrepassNode>>(
                Core3d,
                Node3d::OcclusionCullingDepthPrepass,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::OcclusionCullingDepthPrepass,
                    Node3d::EarlyDownsampleDepthBuffer,
                    Node3d::Prepass,
                ),
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::LateDownsampleDepthBuffer,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DownsampleDepthPipeline>()
            .init_resource::<ResolveDepthPipeline>();
    }
}

/// Place this component on a camera to request that Bevy build a hierarchical
/// depth buffer, which can be used for two-phase occlusion culling.
#[derive(Component, Reflect)]
pub struct HierarchicalDepthBuffer;

impl ExtractComponent for HierarchicalDepthBuffer {
    type QueryData = ();

    type QueryFilter = ();

    type Out = HierarchicalDepthBuffer;

    fn extract_component(_: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(HierarchicalDepthBuffer)
    }
}

/// A render graph node for running the prepass early, before downsampling.
#[derive(Default)]
pub struct OcclusionCullingDepthPrepassNode;

impl ViewNode for OcclusionCullingDepthPrepassNode {
    type ViewQuery = (
        Read<ExtractedCamera>,
        Read<BinnedRenderPhase<Opaque3dPrepass>>,
        Read<BinnedRenderPhase<AlphaMask3dPrepass>>,
        Read<ViewDepthTexture>,
        Has<HierarchicalDepthBuffer>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            camera,
            opaque_prepass_phase,
            alpha_mask_prepass_phase,
            view_depth_texture,
            hierarchical_depth_buffer,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        if !hierarchical_depth_buffer {
            return Ok(());
        }

        let diagnostics = render_context.diagnostic_recorder();
        let prepass_runner = PrepassRunner::new(view_depth_texture, None);
        let view_entity = graph.view_entity();

        render_context.add_command_buffer_generation_task(move |render_device| {
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("occlusion culling depth prepass command encoder"),
                });

            prepass_runner.run_prepass(
                world,
                &render_device,
                diagnostics,
                &mut command_encoder,
                view_entity,
                camera,
                opaque_prepass_phase,
                alpha_mask_prepass_phase,
                "occlusion culling depth prepass",
            );

            command_encoder.finish()
        });

        Ok(())
    }
}

struct DownsampleDebugLabels {
    downsample_group: &'static str,
    downsample_pass: &'static str,
    resolve_pass: &'static str,
}

/// A render graph node for generating a downsampled depth buffer.
///
/// This pass runs right after the occlusion culling prepass, before the main
/// phase or the prepass if any.
#[derive(Default)]
pub struct EarlyDownsampleDepthBufferNode;

impl ViewNode for EarlyDownsampleDepthBufferNode {
    type ViewQuery = (
        Option<Read<HierarchicalDepthBufferViewResources>>,
        Has<HierarchicalDepthBuffer>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (culling_view_resources, gpu_occlusion_culling): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        static DEBUG_LABELS: DownsampleDebugLabels = DownsampleDebugLabels {
            downsample_group: "early downsample depth",
            downsample_pass: "early downsample depth pass",
            resolve_pass: "early resolve depth pass",
        };

        run_downsample_depth_buffer_node(
            render_context,
            culling_view_resources,
            gpu_occlusion_culling,
            world,
            &DEBUG_LABELS,
        )
    }
}

/// A render graph node for generating a downsampled depth buffer.
///
/// This pass runs at the end of the frame, in preparation for the next frame.
#[derive(Default)]
pub struct LateDownsampleDepthBufferNode;

impl ViewNode for LateDownsampleDepthBufferNode {
    type ViewQuery = (
        Option<Read<HierarchicalDepthBufferViewResources>>,
        Has<HierarchicalDepthBuffer>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (culling_view_resources, gpu_occlusion_culling): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        static DEBUG_LABELS: DownsampleDebugLabels = DownsampleDebugLabels {
            downsample_group: "late downsample depth",
            downsample_pass: "late downsample depth pass",
            resolve_pass: "late resolve depth pass",
        };

        run_downsample_depth_buffer_node(
            render_context,
            culling_view_resources,
            gpu_occlusion_culling,
            world,
            &DEBUG_LABELS,
        )
    }
}

/// Runs a single downsample pass, either early or late.
fn run_downsample_depth_buffer_node(
    render_context: &mut RenderContext,
    culling_view_resources: Option<&HierarchicalDepthBufferViewResources>,
    gpu_occlusion_culling: bool,
    world: &World,
    debug_labels: &DownsampleDebugLabels,
) -> Result<(), NodeRunError> {
    let pipeline_cache = world.resource::<PipelineCache>();
    if !gpu_occlusion_culling {
        return Ok(());
    }
    let Some(culling_view_resources) = culling_view_resources else {
        return Ok(());
    };
    let (Some(downsample_depth_pipeline), Some(resolve_depth_pipeline)) = (
        **world.resource::<DownsampleDepthPipelineId>(),
        **world.resource::<ResolveDepthPipelineId>(),
    ) else {
        return Ok(());
    };

    // If the depth buffer is multisampled, resolve it now.
    if let Some(multisample_resources) = &culling_view_resources.multisample_resources {
        resolve_depth_buffer(
            render_context,
            multisample_resources,
            pipeline_cache,
            resolve_depth_pipeline,
            debug_labels,
        );
    }

    // Downsample the depth buffer repeatedly to produce the hierarchical
    // Z-buffer.
    downsample_depth(
        render_context,
        &culling_view_resources.depth_pyramid_mips,
        &culling_view_resources.downsample_depth_bind_groups,
        pipeline_cache,
        downsample_depth_pipeline,
        debug_labels,
    );

    Ok(())
}

/// The [`CachedRenderPipelineId`] for the shader that downsamples the depth
/// buffer to produce a hierarchical Z-buffer.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct DownsampleDepthPipelineId(Option<CachedRenderPipelineId>);

/// The [`CachedRenderPipelineId`] for the multisampled depth buffer resolution
/// shader.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct ResolveDepthPipelineId(Option<CachedRenderPipelineId>);

#[derive(Resource)]
pub struct DownsampleDepthPipeline {
    bind_group_layout: BindGroupLayout,
    depth_pyramid_sampler: Sampler,
}

/// Holds the bind group layout for the shader that resolves multisampled depth
/// buffers in preparation for hierarchical Z-buffer building.
#[derive(Resource)]
pub struct ResolveDepthPipeline {
    /// The bind group layout for the multisampled depth buffer resolution
    /// shader.
    bind_group_layout: BindGroupLayout,
}

/// A component, attached to each view in the render world that has a
/// [`HierarchicalDepthBuffer`] component, that holds the generated hierarchical
/// Z buffer for that view.
#[derive(Component)]
pub struct HierarchicalDepthBufferViewResources {
    /// The actual hierarchical Z buffer.
    ///
    /// This is a mipmapped `R32Float` texture.
    pub depth_pyramid: CachedTexture,
    /// One [`TextureView`] for each mip level of the texture.
    depth_pyramid_mips: Box<[TextureView]>,
    /// Bind groups for each downsampling operation.
    ///
    /// There will be one such operation per mip level.
    downsample_depth_bind_groups: Box<[BindGroup]>,
    /// If the depth buffer is multisampled, holds information needed to resolve
    /// it.
    multisample_resources: Option<MultisampleCullingViewResources>,
}

/// Information needed to resolve a multisampled depth buffer.
struct MultisampleCullingViewResources {
    /// The non-multisampled texture that the multisampled depth buffer is to be
    /// resolved to.
    resolved_depth_texture: CachedTexture,
    /// The bind group for the shader that does this resolving.
    resolve_depth_bind_group: BindGroup,
}

impl FromWorld for DownsampleDepthPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource_mut::<RenderDevice>();

        DownsampleDepthPipeline {
            bind_group_layout: render_device.create_bind_group_layout(
                "downsample depth bind group layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        binding_types::texture_2d(TextureSampleType::Float { filterable: false }),
                        binding_types::sampler(SamplerBindingType::NonFiltering),
                    ),
                ),
            ),
            depth_pyramid_sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("depth pyramid sampler"),
                ..default()
            }),
        }
    }
}

impl FromWorld for ResolveDepthPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource_mut::<RenderDevice>();

        ResolveDepthPipeline {
            bind_group_layout: render_device.create_bind_group_layout(
                "resolve depth bind group layout",
                &BindGroupLayoutEntries::single(
                    ShaderStages::FRAGMENT,
                    binding_types::texture_2d_multisampled(TextureSampleType::Float {
                        filterable: false,
                    }),
                ),
            ),
        }
    }
}

/// Creates the pipeline needed to produce a hierarchical Z-buffer.
pub fn prepare_downsample_depth_pipeline(
    pipeline_cache: ResMut<PipelineCache>,
    mut downsample_depth_pipeline_id: ResMut<DownsampleDepthPipelineId>,
    downsample_depth_pipeline: Res<DownsampleDepthPipeline>,
) {
    if downsample_depth_pipeline_id.is_some() {
        return;
    }

    let render_pipeline_descriptor = RenderPipelineDescriptor {
        label: Some("downsample depth".into()),
        layout: vec![downsample_depth_pipeline.bind_group_layout.clone()],
        push_constant_ranges: vec![],
        vertex: fullscreen_vertex_shader::fullscreen_shader_vertex_state(),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            shader: DOWNSAMPLE_DEPTH_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "downsample_depth".into(),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::R32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
    };

    **downsample_depth_pipeline_id =
        Some(pipeline_cache.queue_render_pipeline(render_pipeline_descriptor));
}

/// Creates the pipeline that resolves multisampled depth buffers, taking the
/// minimum depth of each pixel sample.
///
/// In theory, we could use a Vulkan 1.3 extension [1] for this, but we can't
/// rely on that being available, and it isn't exposed through `wgpu` anyway. So
/// we spin up the raster hardware and do a draw instead.
///
/// [1]: https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkSubpassDescriptionDepthStencilResolveKHR.html
pub fn prepare_resolve_depth_pipeline(
    pipeline_cache: ResMut<PipelineCache>,
    mut resolve_depth_pipeline_id: ResMut<ResolveDepthPipelineId>,
    resolve_depth_pipeline: Res<ResolveDepthPipeline>,
) {
    if resolve_depth_pipeline_id.is_some() {
        return;
    }

    let base_fragment_state = FragmentState {
        shader: RESOLVE_DEPTH_SHADER_HANDLE,
        shader_defs: vec![],
        entry_point: "main".into(),
        targets: vec![Some(ColorTargetState {
            format: TextureFormat::R32Float,
            blend: None,
            write_mask: ColorWrites::ALL,
        })],
    };

    let base_render_pipeline_descriptor = RenderPipelineDescriptor {
        label: Some("resolve depth".into()),
        layout: vec![],
        push_constant_ranges: vec![],
        vertex: fullscreen_vertex_shader::fullscreen_shader_vertex_state(),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: None,
    };

    let multisample_render_pipeline_descriptor = RenderPipelineDescriptor {
        layout: vec![resolve_depth_pipeline.bind_group_layout.clone()],
        fragment: Some(FragmentState {
            shader_defs: vec![],
            ..base_fragment_state
        }),
        ..base_render_pipeline_descriptor
    };

    **resolve_depth_pipeline_id =
        Some(pipeline_cache.queue_render_pipeline(multisample_render_pipeline_descriptor));
}

/// A system that prepares the downsample and resolve pipelines for hierarchical
/// Z buffer creation.
pub fn prepare_culling_view_resources(
    mut commands: Commands,
    views: Query<
        (Entity, &ExtractedView, &ViewDepthTexture),
        (
            With<HierarchicalDepthBuffer>,
            Without<HierarchicalDepthBufferViewResources>,
        ),
    >,
    render_device: ResMut<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    downsample_depth_pipeline: Res<DownsampleDepthPipeline>,
    resolve_depth_pipeline: Res<ResolveDepthPipeline>,
) {
    // We do this for each view because views may have different depth buffers.
    for (view_entity, extracted_view, view_depth_texture) in views.iter() {
        // Determine the size and number of mips.
        let depth_size = Extent3d {
            // If not a power of 2, round down to the nearest power of 2 to
            // ensure depth is conservative.
            width: previous_power_of_2(extracted_view.viewport.z),
            height: previous_power_of_2(extracted_view.viewport.w),
            depth_or_array_layers: 1,
        };
        let depth_mip_count = depth_size.width.max(depth_size.height).ilog2() + 1;

        // Create the depth pyramid.
        let depth_pyramid = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("depth pyramid"),
                size: depth_size,
                mip_level_count: depth_mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        // If we have a multisampled depth texture, prepare the intermediate
        // buffer for resolution.
        let multisample_resources = (view_depth_texture.texture.sample_count() > 1).then(|| {
            prepare_multisample_culling_view_resources(
                &mut texture_cache,
                &render_device,
                extracted_view.viewport.zw(),
                &resolve_depth_pipeline,
                view_depth_texture,
            )
        });

        // Create the views for the mip levels and the bind groups for each pass.
        let depth_pyramid_mips = create_downsample_depth_pyramid_mips(&depth_pyramid);
        let downsample_depth_bind_groups = create_downsample_depth_bind_groups(
            &render_device,
            &downsample_depth_pipeline,
            &multisample_resources,
            view_depth_texture,
            &depth_pyramid_mips,
        );

        // Record the results.
        commands
            .entity(view_entity)
            .insert(HierarchicalDepthBufferViewResources {
                depth_pyramid,
                depth_pyramid_mips,
                downsample_depth_bind_groups,
                multisample_resources,
            });
    }
}

/// Creates [`MultisampleCullingViewResources`] for a single view.
///
/// This is only used for views that render to multisampled targets.
fn prepare_multisample_culling_view_resources(
    texture_cache: &mut TextureCache,
    render_device: &RenderDevice,
    depth_buffer_size: UVec2,
    resolve_depth_pipeline: &ResolveDepthPipeline,
    view_depth_texture: &ViewDepthTexture,
) -> MultisampleCullingViewResources {
    // Create the texture.
    let resolved_depth_texture = texture_cache.get(
        render_device,
        TextureDescriptor {
            label: Some("resolved depth"),
            size: Extent3d {
                width: depth_buffer_size.x,
                height: depth_buffer_size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
    );

    // Create the bind group.
    let resolve_depth_bind_group = render_device.create_bind_group(
        "resolve depth bind group",
        &resolve_depth_pipeline.bind_group_layout,
        &BindGroupEntries::single(view_depth_texture.view()),
    );

    MultisampleCullingViewResources {
        resolved_depth_texture,
        resolve_depth_bind_group,
    }
}

/// Creates the texture views for each mip level of the depth pyramid.
fn create_downsample_depth_pyramid_mips(depth_pyramid: &CachedTexture) -> Box<[TextureView]> {
    (0..depth_pyramid.texture.mip_level_count())
        .map(|i| {
            depth_pyramid.texture.create_view(&TextureViewDescriptor {
                label: Some("depth pyramid texture view"),
                format: Some(TextureFormat::R32Float),
                dimension: Some(TextureViewDimension::D2),
                aspect: TextureAspect::All,
                base_mip_level: i,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: None,
            })
        })
        .collect()
}

/// Creates the bind groups for each mip level of the depth.
fn create_downsample_depth_bind_groups(
    render_device: &RenderDevice,
    downsample_depth_pipeline: &DownsampleDepthPipeline,
    multisample_culling_resources: &Option<MultisampleCullingViewResources>,
    view_depth_texture: &ViewDepthTexture,
    depth_pyramid_mips: &[TextureView],
) -> Box<[BindGroup]> {
    (0..depth_pyramid_mips.len())
        .map(|i| {
            if i == 0 {
                render_device.create_bind_group(
                    "downsample depth bind group (initial)",
                    &downsample_depth_pipeline.bind_group_layout,
                    &BindGroupEntries::sequential((
                        match multisample_culling_resources {
                            Some(multisample_resources) => {
                                &multisample_resources.resolved_depth_texture.default_view
                            }
                            None => view_depth_texture.view(),
                        },
                        &downsample_depth_pipeline.depth_pyramid_sampler,
                    )),
                )
            } else {
                render_device.create_bind_group(
                    "downsample depth bind group",
                    &downsample_depth_pipeline.bind_group_layout,
                    &BindGroupEntries::sequential((
                        &depth_pyramid_mips[i - 1],
                        &downsample_depth_pipeline.depth_pyramid_sampler,
                    )),
                )
            }
        })
        .collect()
}

fn resolve_depth_buffer(
    render_context: &mut RenderContext,
    multisample_culling_resources: &MultisampleCullingViewResources,
    pipeline_cache: &PipelineCache,
    resolve_depth_pipeline_id: CachedRenderPipelineId,
    debug_labels: &DownsampleDebugLabels,
) {
    let Some(resolve_depth_pipeline) =
        pipeline_cache.get_render_pipeline(resolve_depth_pipeline_id)
    else {
        return;
    };

    let resolve_depth_pass = RenderPassDescriptor {
        label: Some(debug_labels.resolve_pass),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &multisample_culling_resources
                .resolved_depth_texture
                .default_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(LinearRgba::BLACK.into()),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    {
        let mut resolve_depth_pass = render_context.begin_tracked_render_pass(resolve_depth_pass);
        resolve_depth_pass.set_bind_group(
            0,
            &multisample_culling_resources.resolve_depth_bind_group,
            &[],
        );
        resolve_depth_pass.set_render_pipeline(resolve_depth_pipeline);
        resolve_depth_pass.draw(0..3, 0..1);
    }
}

/// Performs repeated draw operations to build a hierarchical Z buffer from a
/// depth buffer.
fn downsample_depth(
    render_context: &mut RenderContext,
    depth_pyramid_mips: &[TextureView],
    downsample_depth_bind_groups: &[BindGroup],
    pipeline_cache: &PipelineCache,
    downsample_depth_pipeline_id: CachedRenderPipelineId,
    debug_labels: &DownsampleDebugLabels,
) {
    let Some(downsample_pipeline) =
        pipeline_cache.get_render_pipeline(downsample_depth_pipeline_id)
    else {
        return;
    };

    render_context
        .command_encoder()
        .push_debug_group(debug_labels.downsample_group);

    for (depth_pyramid_mip, downsample_depth_bind_group) in
        depth_pyramid_mips.iter().zip(downsample_depth_bind_groups)
    {
        let downsample_pass = RenderPassDescriptor {
            label: Some(debug_labels.downsample_pass),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: depth_pyramid_mip,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(LinearRgba::BLACK.into()),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut downsample_pass = render_context.begin_tracked_render_pass(downsample_pass);
        downsample_pass.set_bind_group(0, downsample_depth_bind_group, &[]);
        downsample_pass.set_render_pipeline(downsample_pipeline);
        downsample_pass.draw(0..3, 0..1);
    }

    render_context.command_encoder().pop_debug_group();
}
