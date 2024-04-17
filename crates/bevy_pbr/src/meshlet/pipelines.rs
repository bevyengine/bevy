use super::gpu_scene::MeshletGpuScene;
use bevy_asset::Handle;
use bevy_core_pipeline::{
    core_3d::CORE_3D_DEPTH_FORMAT, fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_render::render_resource::*;

pub const MESHLET_CULLING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4325134235233421);
pub const MESHLET_WRITE_INDEX_BUFFER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(5325134235233421);
pub const MESHLET_DOWNSAMPLE_DEPTH_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(6325134235233421);
pub const MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(7325134235233421);
pub const MESHLET_COPY_MATERIAL_DEPTH_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(8325134235233421);

#[derive(Resource)]
pub struct MeshletPipelines {
    cull_first: CachedComputePipelineId,
    cull_second: CachedComputePipelineId,
    write_index_buffer_first: CachedComputePipelineId,
    write_index_buffer_second: CachedComputePipelineId,
    downsample_depth: CachedRenderPipelineId,
    visibility_buffer_raster: CachedRenderPipelineId,
    visibility_buffer_raster_depth_only: CachedRenderPipelineId,
    visibility_buffer_raster_depth_only_clamp_ortho: CachedRenderPipelineId,
    copy_material_depth: CachedRenderPipelineId,
}

impl FromWorld for MeshletPipelines {
    fn from_world(world: &mut World) -> Self {
        let gpu_scene = world.resource::<MeshletGpuScene>();
        let cull_layout = gpu_scene.culling_bind_group_layout();
        let write_index_buffer_layout = gpu_scene.write_index_buffer_bind_group_layout();
        let downsample_depth_layout = gpu_scene.downsample_depth_bind_group_layout();
        let visibility_buffer_layout = gpu_scene.visibility_buffer_raster_bind_group_layout();
        let copy_material_depth_layout = gpu_scene.copy_material_depth_bind_group_layout();
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            cull_first: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_culling_first_pipeline".into()),
                layout: vec![cull_layout.clone()],
                push_constant_ranges: vec![],
                shader: MESHLET_CULLING_SHADER_HANDLE,
                shader_defs: vec!["MESHLET_CULLING_PASS".into()],
                entry_point: "cull_meshlets".into(),
            }),

            cull_second: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_culling_second_pipeline".into()),
                layout: vec![cull_layout],
                push_constant_ranges: vec![],
                shader: MESHLET_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_CULLING_PASS".into(),
                    "MESHLET_SECOND_CULLING_PASS".into(),
                ],
                entry_point: "cull_meshlets".into(),
            }),

            write_index_buffer_first: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_write_index_buffer_first_pipeline".into()),
                    layout: vec![write_index_buffer_layout.clone()],
                    push_constant_ranges: vec![],
                    shader: MESHLET_WRITE_INDEX_BUFFER_SHADER_HANDLE,
                    shader_defs: vec!["MESHLET_WRITE_INDEX_BUFFER_PASS".into()],
                    entry_point: "write_index_buffer".into(),
                },
            ),

            write_index_buffer_second: pipeline_cache.queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: Some("meshlet_write_index_buffer_second_pipeline".into()),
                    layout: vec![write_index_buffer_layout],
                    push_constant_ranges: vec![],
                    shader: MESHLET_WRITE_INDEX_BUFFER_SHADER_HANDLE,
                    shader_defs: vec![
                        "MESHLET_WRITE_INDEX_BUFFER_PASS".into(),
                        "MESHLET_SECOND_WRITE_INDEX_BUFFER_PASS".into(),
                    ],
                    entry_point: "write_index_buffer".into(),
                },
            ),

            downsample_depth: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("meshlet_downsample_depth".into()),
                layout: vec![downsample_depth_layout],
                push_constant_ranges: vec![],
                vertex: fullscreen_shader_vertex_state(),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    shader: MESHLET_DOWNSAMPLE_DEPTH_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "downsample_depth".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::R32Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
            }),

            visibility_buffer_raster: pipeline_cache.queue_render_pipeline(
                RenderPipelineDescriptor {
                    label: Some("meshlet_visibility_buffer_raster_pipeline".into()),
                    layout: vec![visibility_buffer_layout.clone()],
                    push_constant_ranges: vec![],
                    vertex: VertexState {
                        shader: MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
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
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: Some(DepthStencilState {
                        format: CORE_3D_DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::GreaterEqual,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
                        shader_defs: vec![
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT".into(),
                        ],
                        entry_point: "fragment".into(),
                        targets: vec![
                            Some(ColorTargetState {
                                format: TextureFormat::R32Uint,
                                blend: None,
                                write_mask: ColorWrites::ALL,
                            }),
                            Some(ColorTargetState {
                                format: TextureFormat::R16Uint,
                                blend: None,
                                write_mask: ColorWrites::ALL,
                            }),
                        ],
                    }),
                },
            ),

            visibility_buffer_raster_depth_only: pipeline_cache.queue_render_pipeline(
                RenderPipelineDescriptor {
                    label: Some("meshlet_visibility_buffer_raster_depth_only_pipeline".into()),
                    layout: vec![visibility_buffer_layout.clone()],
                    push_constant_ranges: vec![],
                    vertex: VertexState {
                        shader: MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
                        shader_defs: vec!["MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into()],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: Some(DepthStencilState {
                        format: CORE_3D_DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::GreaterEqual,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    fragment: None,
                },
            ),

            visibility_buffer_raster_depth_only_clamp_ortho: pipeline_cache.queue_render_pipeline(
                RenderPipelineDescriptor {
                    label: Some("visibility_buffer_raster_depth_only_clamp_ortho_pipeline".into()),
                    layout: vec![visibility_buffer_layout],
                    push_constant_ranges: vec![],
                    vertex: VertexState {
                        shader: MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
                        shader_defs: vec![
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                            "DEPTH_CLAMP_ORTHO".into(),
                        ],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: Some(DepthStencilState {
                        format: CORE_3D_DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::GreaterEqual,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
                        shader_defs: vec![
                            "MESHLET_VISIBILITY_BUFFER_RASTER_PASS".into(),
                            "DEPTH_CLAMP_ORTHO".into(),
                        ],
                        entry_point: "fragment".into(),
                        targets: vec![],
                    }),
                },
            ),

            copy_material_depth: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("meshlet_copy_material_depth".into()),
                layout: vec![copy_material_depth_layout],
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
                    shader: MESHLET_COPY_MATERIAL_DEPTH_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "copy_material_depth".into(),
                    targets: vec![],
                }),
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
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
        &RenderPipeline,
    )> {
        let pipeline_cache = world.get_resource::<PipelineCache>()?;
        let pipeline = world.get_resource::<Self>()?;
        Some((
            pipeline_cache.get_compute_pipeline(pipeline.cull_first)?,
            pipeline_cache.get_compute_pipeline(pipeline.cull_second)?,
            pipeline_cache.get_compute_pipeline(pipeline.write_index_buffer_first)?,
            pipeline_cache.get_compute_pipeline(pipeline.write_index_buffer_second)?,
            pipeline_cache.get_render_pipeline(pipeline.downsample_depth)?,
            pipeline_cache.get_render_pipeline(pipeline.visibility_buffer_raster)?,
            pipeline_cache.get_render_pipeline(pipeline.visibility_buffer_raster_depth_only)?,
            pipeline_cache
                .get_render_pipeline(pipeline.visibility_buffer_raster_depth_only_clamp_ortho)?,
            pipeline_cache.get_render_pipeline(pipeline.copy_material_depth)?,
        ))
    }
}
