use super::resource_manager::ResourceManager;
use bevy_asset::{weak_handle, Handle};
use bevy_core_pipeline::{
    core_3d::CORE_3D_DEPTH_FORMAT, experimental::mip_generation::DOWNSAMPLE_DEPTH_SHADER_HANDLE,
    FullscreenShader,
};
use bevy_ecs::{
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_render::render_resource::*;

pub const MESHLET_CLEAR_VISIBILITY_BUFFER_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("a4bf48e4-5605-4d1c-987e-29c7b1ec95dc");
pub const MESHLET_CULL_SHARED_HANDLE: Handle<Shader> =
    weak_handle!("6e05f79e-9633-4313-9a63-215dd67c6caf");
pub const MESHLET_INSTANCE_CULLING_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("479c1bc3-e220-44a4-a894-ddc116f59db5");
pub const MESHLET_BVH_CULLING_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("1191f402-e1be-4e8c-9da6-25c27cdaac05");
pub const MESHLET_MESHLET_CULLING_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c740c588-4d25-4a8a-a112-48757077e821");
pub const MESHLET_VISIBILITY_BUFFER_SOFTWARE_RASTER_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("68cc6826-8321-43d1-93d5-4f61f0456c13");
pub const MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("4b4e3020-748f-4baf-b011-87d9d2a12796");
pub const MESHLET_RESOLVE_RENDER_TARGETS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c218ce17-cf59-4268-8898-13ecf384f133");
pub const MESHLET_REMAP_1D_TO_2D_DISPATCH_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("f5b7edfc-2eac-4407-8f5c-1265d4d795c2");
pub const MESHLET_FILL_COUNTS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("1047cc44-4839-482b-aa9b-82dde28ee954");

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
}

impl FromWorld for MeshletPipelines {
    fn from_world(world: &mut World) -> Self {
        let resource_manager = world.resource::<ResourceManager>();
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

        let vertex_state = world.resource::<FullscreenShader>().to_vertex_state();
        let fill_counts_layout = resource_manager.fill_counts_bind_group_layout.clone();
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            clear_visibility_buffer: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_clear_visibility_buffer_pipeline".into()),
                    layout: vec![clear_visibility_buffer_bind_group_layout],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..8,
                    }],
                    shader: MESHLET_CLEAR_VISIBILITY_BUFFER_SHADER_HANDLE,
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
                    shader: MESHLET_CLEAR_VISIBILITY_BUFFER_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "clear_visibility_buffer".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            first_instance_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_first_instance_cull_pipeline".into()),
                layout: vec![first_instance_cull_bind_group_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: MESHLET_INSTANCE_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_INSTANCE_CULLING_PASS".into(),
                    "MESHLET_FIRST_CULLING_PASS".into(),
                ],
                entry_point: "cull_instances".into(),
                zero_initialize_workgroup_memory: false,
            }),

            second_instance_cull: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_second_instance_cull_pipeline".into()),
                    layout: vec![second_instance_cull_bind_group_layout.clone()],
                    push_constant_ranges: vec![PushConstantRange {
                        stages: ShaderStages::COMPUTE,
                        range: 0..4,
                    }],
                    shader: MESHLET_INSTANCE_CULLING_SHADER_HANDLE,
                    shader_defs: vec![
                        "MESHLET_INSTANCE_CULLING_PASS".into(),
                        "MESHLET_SECOND_CULLING_PASS".into(),
                    ],
                    entry_point: "cull_instances".into(),
                    zero_initialize_workgroup_memory: false,
                },
            ),

            first_bvh_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_first_bvh_cull_pipeline".into()),
                layout: vec![first_bvh_cull_bind_group_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: MESHLET_BVH_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_BVH_CULLING_PASS".into(),
                    "MESHLET_FIRST_CULLING_PASS".into(),
                ],
                entry_point: "cull_bvh".into(),
                zero_initialize_workgroup_memory: false,
            }),

            second_bvh_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_second_bvh_cull_pipeline".into()),
                layout: vec![second_bvh_cull_bind_group_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: MESHLET_BVH_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_BVH_CULLING_PASS".into(),
                    "MESHLET_SECOND_CULLING_PASS".into(),
                ],
                entry_point: "cull_bvh".into(),
                zero_initialize_workgroup_memory: false,
            }),

            first_meshlet_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_first_meshlet_cull_pipeline".into()),
                layout: vec![first_meshlet_cull_bind_group_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: MESHLET_MESHLET_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_CLUSTER_CULLING_PASS".into(),
                    "MESHLET_FIRST_CULLING_PASS".into(),
                ],
                entry_point: "cull_clusters".into(),
                zero_initialize_workgroup_memory: false,
            }),

            second_meshlet_cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_second_meshlet_cull_pipeline".into()),
                layout: vec![second_meshlet_cull_bind_group_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: MESHLET_MESHLET_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_CLUSTER_CULLING_PASS".into(),
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
                    shader: MESHLET_VISIBILITY_BUFFER_SOFTWARE_RASTER_SHADER_HANDLE,
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
                    shader: MESHLET_VISIBILITY_BUFFER_SOFTWARE_RASTER_SHADER_HANDLE,
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
                        shader: MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
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
                        shader: MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
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
                        shader: MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
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
                        shader: MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
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
                        shader: MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
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
                        shader: MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
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
                vertex: vertex_state.clone(),
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
                    shader: MESHLET_RESOLVE_RENDER_TARGETS_SHADER_HANDLE,
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
                    vertex: vertex_state.clone(),
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
                        shader: MESHLET_RESOLVE_RENDER_TARGETS_SHADER_HANDLE,
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
                    vertex: vertex_state,
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
                        shader: MESHLET_RESOLVE_RENDER_TARGETS_SHADER_HANDLE,
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
                    shader: MESHLET_REMAP_1D_TO_2D_DISPATCH_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "remap_dispatch".into(),
                    zero_initialize_workgroup_memory: false,
                })
            }),

            fill_counts: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_fill_counts_pipeline".into()),
                layout: vec![fill_counts_layout],
                push_constant_ranges: vec![],
                shader: MESHLET_FILL_COUNTS_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fill_counts".into(),
                zero_initialize_workgroup_memory: false,
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
