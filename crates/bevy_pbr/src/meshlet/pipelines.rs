use super::resource_manager::ResourceManager;
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::{
    core_3d::CORE_3D_DEPTH_FORMAT, experimental::mip_generation::DownsampleDepthShader,
    FullscreenShader,
};
use bevy_ecs::{
    resource::Resource,
    system::{Commands, Res},
    world::World,
};
use bevy_render::render_resource::*;
use bevy_shader::Shader;
use bevy_utils::default;

#[derive(Resource)]
pub struct MeshletPipelines {
    clear_visibility_buffer: CachedComputePipelineId,
    clear_visibility_buffer_shadow_view: CachedComputePipelineId,
    first_instance_cull: CachedComputePipelineId,
    second_instance_cull: CachedComputePipelineId,
    first_bvh_cull: CachedComputePipelineId,
    second_bvh_cull: CachedComputePipelineId,
    first_meshlet_cull: CachedComputePipelineId,
    second_meshlet_cull: CachedComputePipelineId,
    downsample_depth_first: CachedComputePipelineId,
    downsample_depth_second: CachedComputePipelineId,
    downsample_depth_first_shadow_view: CachedComputePipelineId,
    downsample_depth_second_shadow_view: CachedComputePipelineId,
    visibility_buffer_software_raster: CachedComputePipelineId,
    visibility_buffer_software_raster_shadow_view: CachedComputePipelineId,
    visibility_buffer_hardware_raster: CachedRenderPipelineId,
    visibility_buffer_hardware_raster_shadow_view: CachedRenderPipelineId,
    visibility_buffer_hardware_raster_shadow_view_unclipped: CachedRenderPipelineId,
    resolve_depth: CachedRenderPipelineId,
    resolve_depth_shadow_view: CachedRenderPipelineId,
    resolve_material_depth: CachedRenderPipelineId,
    remap_1d_to_2d_dispatch: Option<CachedComputePipelineId>,
    fill_counts: CachedComputePipelineId,
    pub(crate) meshlet_mesh_material: Handle<Shader>,
}

pub fn init_meshlet_pipelines(
    mut commands: Commands,
    resource_manager: Res<ResourceManager>,
    fullscreen_shader: Res<FullscreenShader>,
    downsample_depth_shader: Res<DownsampleDepthShader>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    let clear_visibility_buffer_bind_group_layout = resource_manager
        .clear_visibility_buffer_bind_group_layout
        .clone();
    let clear_visibility_buffer_shadow_view_bind_group_layout = resource_manager
        .clear_visibility_buffer_shadow_view_bind_group_layout
        .clone();
    let first_instance_cull_bind_group_layout = resource_manager
        .first_instance_cull_bind_group_layout
        .clone();
    let second_instance_cull_bind_group_layout = resource_manager
        .second_instance_cull_bind_group_layout
        .clone();
    let first_bvh_cull_bind_group_layout =
        resource_manager.first_bvh_cull_bind_group_layout.clone();
    let second_bvh_cull_bind_group_layout =
        resource_manager.second_bvh_cull_bind_group_layout.clone();
    let first_meshlet_cull_bind_group_layout = resource_manager
        .first_meshlet_cull_bind_group_layout
        .clone();
    let second_meshlet_cull_bind_group_layout = resource_manager
        .second_meshlet_cull_bind_group_layout
        .clone();
    let downsample_depth_layout = resource_manager.downsample_depth_bind_group_layout.clone();
    let downsample_depth_shadow_view_layout = resource_manager
        .downsample_depth_shadow_view_bind_group_layout
        .clone();
    let visibility_buffer_raster_layout = resource_manager
        .visibility_buffer_raster_bind_group_layout
        .clone();
    let visibility_buffer_raster_shadow_view_layout = resource_manager
        .visibility_buffer_raster_shadow_view_bind_group_layout
        .clone();
    let resolve_depth_layout = resource_manager.resolve_depth_bind_group_layout.clone();
    let resolve_depth_shadow_view_layout = resource_manager
        .resolve_depth_shadow_view_bind_group_layout
        .clone();
    let resolve_material_depth_layout = resource_manager
        .resolve_material_depth_bind_group_layout
        .clone();
    let remap_1d_to_2d_dispatch_layout = resource_manager
        .remap_1d_to_2d_dispatch_bind_group_layout
        .clone();

    let downsample_depth_shader = (*downsample_depth_shader).clone();
    let vertex_state = fullscreen_shader.to_vertex_state();
    let fill_counts_layout = resource_manager.fill_counts_bind_group_layout.clone();

    let clear_visibility_buffer =
        load_embedded_asset!(asset_server.as_ref(), "clear_visibility_buffer.wgsl");
    let cull_instances = load_embedded_asset!(asset_server.as_ref(), "cull_instances.wgsl");
    let cull_bvh = load_embedded_asset!(asset_server.as_ref(), "cull_bvh.wgsl");
    let cull_clusters = load_embedded_asset!(asset_server.as_ref(), "cull_clusters.wgsl");
    let visibility_buffer_software_raster = load_embedded_asset!(
        asset_server.as_ref(),
        "visibility_buffer_software_raster.wgsl"
    );
    let visibility_buffer_hardware_raster = load_embedded_asset!(
        asset_server.as_ref(),
        "visibility_buffer_hardware_raster.wgsl"
    );
    let resolve_render_targets =
        load_embedded_asset!(asset_server.as_ref(), "resolve_render_targets.wgsl");
    let remap_1d_to_2d_dispatch =
        load_embedded_asset!(asset_server.as_ref(), "remap_1d_to_2d_dispatch.wgsl");
    let fill_counts = load_embedded_asset!(asset_server.as_ref(), "fill_counts.wgsl");
    let meshlet_mesh_material =
        load_embedded_asset!(asset_server.as_ref(), "meshlet_mesh_material.wgsl");

    commands.insert_resource(MeshletPipelines {
        clear_visibility_buffer: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_clear_visibility_buffer_pipeline".into()),
            layout: vec![clear_visibility_buffer_bind_group_layout],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..8,
            }],
            shader: clear_visibility_buffer.clone(),
            shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into()],
            ..default()
        }),

        clear_visibility_buffer_shadow_view: pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some("meshlet_clear_visibility_buffer_shadow_view_pipeline".into()),
                layout: vec![clear_visibility_buffer_shadow_view_bind_group_layout],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: clear_visibility_buffer,
                ..default()
            },
        ),

        first_instance_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_first_instance_cull_pipeline".into()),
            layout: vec![first_instance_cull_bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: cull_instances.clone(),
            shader_defs: vec![
                "MESHLET_INSTANCE_CULLING_PASS".into(),
                "MESHLET_FIRST_CULLING_PASS".into(),
            ],
            ..default()
        }),

        second_instance_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_second_instance_cull_pipeline".into()),
            layout: vec![second_instance_cull_bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: cull_instances,
            shader_defs: vec![
                "MESHLET_INSTANCE_CULLING_PASS".into(),
                "MESHLET_SECOND_CULLING_PASS".into(),
            ],
            ..default()
        }),

        first_bvh_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_first_bvh_cull_pipeline".into()),
            layout: vec![first_bvh_cull_bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..8,
            }],
            shader: cull_bvh.clone(),
            shader_defs: vec![
                "MESHLET_BVH_CULLING_PASS".into(),
                "MESHLET_FIRST_CULLING_PASS".into(),
            ],
            ..default()
        }),

        second_bvh_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_second_bvh_cull_pipeline".into()),
            layout: vec![second_bvh_cull_bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..8,
            }],
            shader: cull_bvh,
            shader_defs: vec![
                "MESHLET_BVH_CULLING_PASS".into(),
                "MESHLET_SECOND_CULLING_PASS".into(),
            ],
            ..default()
        }),

        first_meshlet_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_first_meshlet_cull_pipeline".into()),
            layout: vec![first_meshlet_cull_bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: cull_clusters.clone(),
            shader_defs: vec![
                "MESHLET_CLUSTER_CULLING_PASS".into(),
                "MESHLET_FIRST_CULLING_PASS".into(),
            ],
            ..default()
        }),

        second_meshlet_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_second_meshlet_cull_pipeline".into()),
            layout: vec![second_meshlet_cull_bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: cull_clusters,
            shader_defs: vec![
                "MESHLET_CLUSTER_CULLING_PASS".into(),
                "MESHLET_SECOND_CULLING_PASS".into(),
            ],
            ..default()
        }),

        downsample_depth_first: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_downsample_depth_first_pipeline".into()),
            layout: vec![downsample_depth_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: downsample_depth_shader.clone(),
            shader_defs: vec![
                "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                "MESHLET".into(),
            ],
            entry_point: Some("downsample_depth_first".into()),
            ..default()
        }),

        downsample_depth_second: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_downsample_depth_second_pipeline".into()),
            layout: vec![downsample_depth_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: downsample_depth_shader.clone(),
            shader_defs: vec![
                "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                "MESHLET".into(),
            ],
            entry_point: Some("downsample_depth_second".into()),
            ..default()
        }),

        downsample_depth_first_shadow_view: pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some("meshlet_downsample_depth_first_pipeline".into()),
                layout: vec![downsample_depth_shadow_view_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: downsample_depth_shader.clone(),
                shader_defs: vec!["MESHLET".into()],
                entry_point: Some("downsample_depth_first".into()),
                ..default()
            },
        ),

        downsample_depth_second_shadow_view: pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some("meshlet_downsample_depth_second_pipeline".into()),
                layout: vec![downsample_depth_shadow_view_layout],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: downsample_depth_shader,
                shader_defs: vec!["MESHLET".into()],
                entry_point: Some("downsample_depth_second".into()),
                zero_initialize_workgroup_memory: false,
            },
        ),

        visibility_buffer_software_raster: pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some("meshlet_visibility_buffer_software_raster_pipeline".into()),
                layout: vec![visibility_buffer_raster_layout.clone()],
                push_constant_ranges: vec![],
                shader: visibility_buffer_software_raster.clone(),
                shader_defs: vec![
                    "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                    "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                    if remap_1d_to_2d_dispatch_layout.is_some() {
                        "MESHLET_2D_DISPATCH"
                    } else {
                        ""
                    }
                    .into(),
                ],
                ..default()
            },
        ),

        visibility_buffer_software_raster_shadow_view: pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some(
                    "meshlet_visibility_buffer_software_raster_shadow_view_pipeline".into(),
                ),
                layout: vec![visibility_buffer_raster_shadow_view_layout.clone()],
                push_constant_ranges: vec![],
                shader: visibility_buffer_software_raster,
                shader_defs: vec![
                    "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                    if remap_1d_to_2d_dispatch_layout.is_some() {
                        "MESHLET_2D_DISPATCH"
                    } else {
                        ""
                    }
                    .into(),
                ],
                ..default()
            },
        ),

        visibility_buffer_hardware_raster: pipeline_cache.queue_render_pipeline(
            RenderPipelineDescriptor {
                label: Some("meshlet_visibility_buffer_hardware_raster_pipeline".into()),
                layout: vec![visibility_buffer_raster_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..4,
                }],
                vertex: VertexState {
                    shader: visibility_buffer_hardware_raster.clone(),
                    shader_defs: vec![
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                    ],
                    ..default()
                },
                fragment: Some(FragmentState {
                    shader: visibility_buffer_hardware_raster.clone(),
                    shader_defs: vec![
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                    ],
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::R8Uint,
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    })],
                    ..default()
                }),
                ..default()
            },
        ),

        visibility_buffer_hardware_raster_shadow_view: pipeline_cache.queue_render_pipeline(
            RenderPipelineDescriptor {
                label: Some(
                    "meshlet_visibility_buffer_hardware_raster_shadow_view_pipeline".into(),
                ),
                layout: vec![visibility_buffer_raster_shadow_view_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..4,
                }],
                vertex: VertexState {
                    shader: visibility_buffer_hardware_raster.clone(),
                    shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                    ..default()
                },
                fragment: Some(FragmentState {
                    shader: visibility_buffer_hardware_raster.clone(),
                    shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::R8Uint,
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    })],
                    ..default()
                }),
                ..default()
            },
        ),

        visibility_buffer_hardware_raster_shadow_view_unclipped: pipeline_cache
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(
                    "meshlet_visibility_buffer_hardware_raster_shadow_view_unclipped_pipeline"
                        .into(),
                ),
                layout: vec![visibility_buffer_raster_shadow_view_layout],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..4,
                }],
                vertex: VertexState {
                    shader: visibility_buffer_hardware_raster.clone(),
                    shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                    ..default()
                },
                fragment: Some(FragmentState {
                    shader: visibility_buffer_hardware_raster,
                    shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::R8Uint,
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    })],
                    ..default()
                }),
                ..default()
            }),

        resolve_depth: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("meshlet_resolve_depth_pipeline".into()),
            layout: vec![resolve_depth_layout],
            vertex: vertex_state.clone(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Always,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            fragment: Some(FragmentState {
                shader: resolve_render_targets.clone(),
                shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into()],
                entry_point: Some("resolve_depth".into()),
                ..default()
            }),
            ..default()
        }),

        resolve_depth_shadow_view: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("meshlet_resolve_depth_pipeline".into()),
            layout: vec![resolve_depth_shadow_view_layout],
            vertex: vertex_state.clone(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Always,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            fragment: Some(FragmentState {
                shader: resolve_render_targets.clone(),
                entry_point: Some("resolve_depth".into()),
                ..default()
            }),
            ..default()
        }),

        resolve_material_depth: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("meshlet_resolve_material_depth_pipeline".into()),
            layout: vec![resolve_material_depth_layout],
            vertex: vertex_state,
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth16Unorm,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Always,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            fragment: Some(FragmentState {
                shader: resolve_render_targets,
                shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into()],
                entry_point: Some("resolve_material_depth".into()),
                targets: vec![],
            }),
            ..default()
        }),

        fill_counts: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("meshlet_fill_counts_pipeline".into()),
            layout: vec![fill_counts_layout],
            shader: fill_counts,
            shader_defs: vec![if remap_1d_to_2d_dispatch_layout.is_some() {
                "MESHLET_2D_DISPATCH"
            } else {
                ""
            }
            .into()],
            ..default()
        }),

        remap_1d_to_2d_dispatch: remap_1d_to_2d_dispatch_layout.map(|layout| {
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_remap_1d_to_2d_dispatch_pipeline".into()),
                layout: vec![layout],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: remap_1d_to_2d_dispatch,
                ..default()
            })
        }),

        meshlet_mesh_material,
    });
}

impl MeshletPipelines {
    pub fn get(
        world: &World,
    ) -> Option<(
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        Option<&ComputePipeline>,
        &ComputePipeline,
    )> {
        let pipeline_cache = world.get_resource::<PipelineCache>()?;
        let pipeline = world.get_resource::<Self>()?;
        Some((
            pipeline_cache.get_compute_pipeline(pipeline.clear_visibility_buffer)?,
            pipeline_cache.get_compute_pipeline(pipeline.clear_visibility_buffer_shadow_view)?,
            pipeline_cache.get_compute_pipeline(pipeline.first_instance_cull)?,
            pipeline_cache.get_compute_pipeline(pipeline.second_instance_cull)?,
            pipeline_cache.get_compute_pipeline(pipeline.first_bvh_cull)?,
            pipeline_cache.get_compute_pipeline(pipeline.second_bvh_cull)?,
            pipeline_cache.get_compute_pipeline(pipeline.first_meshlet_cull)?,
            pipeline_cache.get_compute_pipeline(pipeline.second_meshlet_cull)?,
            pipeline_cache.get_compute_pipeline(pipeline.downsample_depth_first)?,
            pipeline_cache.get_compute_pipeline(pipeline.downsample_depth_second)?,
            pipeline_cache.get_compute_pipeline(pipeline.downsample_depth_first_shadow_view)?,
            pipeline_cache.get_compute_pipeline(pipeline.downsample_depth_second_shadow_view)?,
            pipeline_cache.get_compute_pipeline(pipeline.visibility_buffer_software_raster)?,
            pipeline_cache
                .get_compute_pipeline(pipeline.visibility_buffer_software_raster_shadow_view)?,
            pipeline_cache.get_render_pipeline(pipeline.visibility_buffer_hardware_raster)?,
            pipeline_cache
                .get_render_pipeline(pipeline.visibility_buffer_hardware_raster_shadow_view)?,
            pipeline_cache.get_render_pipeline(
                pipeline.visibility_buffer_hardware_raster_shadow_view_unclipped,
            )?,
            pipeline_cache.get_render_pipeline(pipeline.resolve_depth)?,
            pipeline_cache.get_render_pipeline(pipeline.resolve_depth_shadow_view)?,
            pipeline_cache.get_render_pipeline(pipeline.resolve_material_depth)?,
            match pipeline.remap_1d_to_2d_dispatch {
                Some(id) => Some(pipeline_cache.get_compute_pipeline(id)?),
                None => None,
            },
            pipeline_cache.get_compute_pipeline(pipeline.fill_counts)?,
        ))
    }
}
