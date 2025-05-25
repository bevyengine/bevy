use super::resource_manager::ResourceManager;
use bevy_asset::load_embedded_asset;
use bevy_core_pipeline::{
    core_3d::CORE_3D_DEPTH_FORMAT, experimental::mip_generation::DOWNSAMPLE_DEPTH_SHADER_HANDLE,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy_ecs::{
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_render::render_resource::*;

#[derive(Resource)]
pub struct MeshletPipelines {
    fill_cluster_buffers: CachedComputePipelineId,
    clear_visibility_buffer: CachedComputePipelineId,
    clear_visibility_buffer_shadow_view: CachedComputePipelineId,
    cull_first: CachedComputePipelineId,
    cull_second: CachedComputePipelineId,
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
}

impl FromWorld for MeshletPipelines {
    fn from_world(world: &mut World) -> Self {
        let clear_visibility_buffer_shader =
            load_embedded_asset!(world, "clear_visibility_buffer.wgsl");
        let fill_cluster_buffers_shader = load_embedded_asset!(world, "fill_cluster_buffers.wgsl");
        let cull_clusters_shader = load_embedded_asset!(world, "cull_clusters.wgsl");
        let visibility_buffer_software_raster_shader =
            load_embedded_asset!(world, "visibility_buffer_software_raster.wgsl");
        let visibility_buffer_hardware_raster_shader =
            load_embedded_asset!(world, "visibility_buffer_hardware_raster.wgsl");
        let resolve_render_targets_shader =
            load_embedded_asset!(world, "resolve_render_targets.wgsl");
        let remap_1d_to_2d_dispatch_shader =
            load_embedded_asset!(world, "remap_1d_to_2d_dispatch.wgsl");

        let resource_manager = world.resource::<ResourceManager>();
        let fill_cluster_buffers_bind_group_layout = resource_manager
            .fill_cluster_buffers_bind_group_layout
            .clone();
        let clear_visibility_buffer_bind_group_layout = resource_manager
            .clear_visibility_buffer_bind_group_layout
            .clone();
        let clear_visibility_buffer_shadow_view_bind_group_layout = resource_manager
            .clear_visibility_buffer_shadow_view_bind_group_layout
            .clone();
        let cull_layout = resource_manager.culling_bind_group_layout.clone();
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
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            fill_cluster_buffers: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_fill_cluster_buffers_pipeline".into()),
                    layout: vec![fill_cluster_buffers_bind_group_layout],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..4,
                    }],
                    shader: fill_cluster_buffers_shader,
                    shader_defs: vec!["MESHLET_FILL_CLUSTER_BUFFERS_PASS".into()],
                    entry_point: "fill_cluster_buffers".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            clear_visibility_buffer: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_clear_visibility_buffer_pipeline".into()),
                    layout: vec![clear_visibility_buffer_bind_group_layout],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..8,
                    }],
                    shader: clear_visibility_buffer_shader.clone(),
                    shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into()],
                    entry_point: "clear_visibility_buffer".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            clear_visibility_buffer_shadow_view: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_clear_visibility_buffer_shadow_view_pipeline".into()),
                    layout: vec![clear_visibility_buffer_shadow_view_bind_group_layout],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..8,
                    }],
                    shader: clear_visibility_buffer_shader.clone(),
                    shader_defs: vec![],
                    entry_point: "clear_visibility_buffer".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            cull_first: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_culling_first_pipeline".into()),
                layout: vec![cull_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: cull_clusters_shader.clone(),
                shader_defs: vec![
                    "MESHLET_CULLING_PASS".into(),
                    "MESHLET_FIRST_CULLING_PASS".into(),
                ],
                entry_point: "cull_clusters".into(),
                zero_initialize_workgroup_memory: false,
            }),

            cull_second: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_culling_second_pipeline".into()),
                layout: vec![cull_layout],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: cull_clusters_shader,
                shader_defs: vec![
                    "MESHLET_CULLING_PASS".into(),
                    "MESHLET_SECOND_CULLING_PASS".into(),
                ],
                entry_point: "cull_clusters".into(),
                zero_initialize_workgroup_memory: false,
            }),

            downsample_depth_first: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_downsample_depth_first_pipeline".into()),
                    layout: vec![downsample_depth_layout.clone()],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..4,
                    }],
                    shader: DOWNSAMPLE_DEPTH_SHADER_HANDLE,
                    shader_defs: vec![
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                        "MESHLET".into(),
                    ],
                    entry_point: "downsample_depth_first".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            downsample_depth_second: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_downsample_depth_second_pipeline".into()),
                    layout: vec![downsample_depth_layout.clone()],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..4,
                    }],
                    shader: DOWNSAMPLE_DEPTH_SHADER_HANDLE,
                    shader_defs: vec![
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                        "MESHLET".into(),
                    ],
                    entry_point: "downsample_depth_second".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            downsample_depth_first_shadow_view: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_downsample_depth_first_pipeline".into()),
                    layout: vec![downsample_depth_shadow_view_layout.clone()],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..4,
                    }],
                    shader: DOWNSAMPLE_DEPTH_SHADER_HANDLE,
                    shader_defs: vec!["MESHLET".into()],
                    entry_point: "downsample_depth_first".into(),
                    zero_initialize_workgroup_memory: false,
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
                    shader: DOWNSAMPLE_DEPTH_SHADER_HANDLE,
                    shader_defs: vec!["MESHLET".into()],
                    entry_point: "downsample_depth_second".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            visibility_buffer_software_raster: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_visibility_buffer_software_raster_pipeline".into()),
                    layout: vec![visibility_buffer_raster_layout.clone()],
                    push_constant_ranges: vec![],
                    shader: visibility_buffer_software_raster_shader.clone(),
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
                    entry_point: "rasterize_cluster".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            visibility_buffer_software_raster_shadow_view: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some(
                        "meshlet_visibility_buffer_software_raster_shadow_view_pipeline".into(),
                    ),
                    layout: vec![visibility_buffer_raster_shadow_view_layout.clone()],
                    push_constant_ranges: vec![],
                    shader: visibility_buffer_software_raster_shader,
                    shader_defs: vec![
                        "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                        if remap_1d_to_2d_dispatch_layout.is_some() {
                            "MESHLET_2D_DISPATCH"
                        } else {
                            ""
                        }
                        .into(),
                    ],
                    entry_point: "rasterize_cluster".into(),
                    zero_initialize_workgroup_memory: false,
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
                        shader: visibility_buffer_hardware_raster_shader.clone(),
                        shader_defs: vec![
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                        ],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Ccw,
                        cull_mode: Some(Face::Back),
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: visibility_buffer_hardware_raster_shader.clone(),
                        shader_defs: vec![
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                        ],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::R8Uint,
                            blend: None,
                            write_mask: ColorWrites::empty(),
                        })],
                    }),
                    zero_initialize_workgroup_memory: false,
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
                        shader: visibility_buffer_hardware_raster_shader.clone(),
                        shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Ccw,
                        cull_mode: Some(Face::Back),
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: visibility_buffer_hardware_raster_shader.clone(),
                        shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::R8Uint,
                            blend: None,
                            write_mask: ColorWrites::empty(),
                        })],
                    }),
                    zero_initialize_workgroup_memory: false,
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
                        shader: visibility_buffer_hardware_raster_shader.clone(),
                        shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Ccw,
                        cull_mode: Some(Face::Back),
                        unclipped_depth: true,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: visibility_buffer_hardware_raster_shader,
                        shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::R8Uint,
                            blend: None,
                            write_mask: ColorWrites::empty(),
                        })],
                    }),
                    zero_initialize_workgroup_memory: false,
                }),

            resolve_depth: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("meshlet_resolve_depth_pipeline".into()),
                layout: vec![resolve_depth_layout],
                push_constant_ranges: vec![],
                vertex: fullscreen_shader_vertex_state(),
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: CORE_3D_DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Always,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    shader: resolve_render_targets_shader.clone(),
                    shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into()],
                    entry_point: "resolve_depth".into(),
                    targets: vec![],
                }),
                zero_initialize_workgroup_memory: false,
            }),

            resolve_depth_shadow_view: pipeline_cache.queue_render_pipeline(
                RenderPipelineDescriptor {
                    label: Some("meshlet_resolve_depth_pipeline".into()),
                    layout: vec![resolve_depth_shadow_view_layout],
                    push_constant_ranges: vec![],
                    vertex: fullscreen_shader_vertex_state(),
                    primitive: PrimitiveState::default(),
                    depth_stencil: Some(DepthStencilState {
                        format: CORE_3D_DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Always,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: resolve_render_targets_shader.clone(),
                        shader_defs: vec![],
                        entry_point: "resolve_depth".into(),
                        targets: vec![],
                    }),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            resolve_material_depth: pipeline_cache.queue_render_pipeline(
                RenderPipelineDescriptor {
                    label: Some("meshlet_resolve_material_depth_pipeline".into()),
                    layout: vec![resolve_material_depth_layout],
                    push_constant_ranges: vec![],
                    vertex: fullscreen_shader_vertex_state(),
                    primitive: PrimitiveState::default(),
                    depth_stencil: Some(DepthStencilState {
                        format: TextureFormat::Depth16Unorm,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Always,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: resolve_render_targets_shader,
                        shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into()],
                        entry_point: "resolve_material_depth".into(),
                        targets: vec![],
                    }),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            remap_1d_to_2d_dispatch: remap_1d_to_2d_dispatch_layout.map(|layout| {
                pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                    label: Some("meshlet_remap_1d_to_2d_dispatch_pipeline".into()),
                    layout: vec![layout],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..4,
                    }],
                    shader: remap_1d_to_2d_dispatch_shader,
                    shader_defs: vec![],
                    entry_point: "remap_dispatch".into(),
                    zero_initialize_workgroup_memory: false,
                })
            }),
        }
    }
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
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        Option<&ComputePipeline>,
    )> {
        let pipeline_cache = world.get_resource::<PipelineCache>()?;
        let pipeline = world.get_resource::<Self>()?;
        Some((
            pipeline_cache.get_compute_pipeline(pipeline.fill_cluster_buffers)?,
            pipeline_cache.get_compute_pipeline(pipeline.clear_visibility_buffer)?,
            pipeline_cache.get_compute_pipeline(pipeline.clear_visibility_buffer_shadow_view)?,
            pipeline_cache.get_compute_pipeline(pipeline.cull_first)?,
            pipeline_cache.get_compute_pipeline(pipeline.cull_second)?,
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
        ))
    }
}
