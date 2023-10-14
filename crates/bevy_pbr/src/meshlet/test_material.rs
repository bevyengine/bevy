use super::gpu_scene::MeshletGpuScene;
use crate::MeshPipeline;
use bevy_asset::Handle;
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_render::{render_resource::*, texture::BevyDefault};

pub const MESHLET_TEST_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(8525634235233421);

#[derive(Resource)]
pub struct MeshletTestMaterial {
    pipeline: CachedRenderPipelineId,
}

impl FromWorld for MeshletTestMaterial {
    fn from_world(world: &mut World) -> Self {
        let gpu_scene = world.resource::<MeshletGpuScene>();
        let mesh_pipeline = world.resource::<MeshPipeline>();
        let view_layout = mesh_pipeline.view_layout_multisampled.clone();
        let meshlet_layout = gpu_scene.draw_bind_group_layout().clone();
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        let shader_defs = vec![ShaderDefVal::UInt("MESHLET_BIND_GROUP".into(), 1)];

        Self {
            pipeline: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("meshlet_test_material_pipeline".into()),
                layout: vec![view_layout, meshlet_layout],
                push_constant_ranges: vec![],
                vertex: VertexState {
                    shader: MESHLET_TEST_MATERIAL_SHADER_HANDLE,
                    entry_point: "vertex".into(),
                    shader_defs: shader_defs.clone(),
                    buffers: vec![],
                },
                primitive: PrimitiveState {
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::GreaterEqual,
                    stencil: StencilState {
                        front: StencilFaceState::IGNORE,
                        back: StencilFaceState::IGNORE,
                        read_mask: 0,
                        write_mask: 0,
                    },
                    bias: DepthBiasState {
                        constant: 0,
                        slope_scale: 0.0,
                        clamp: 0.0,
                    },
                }),
                multisample: MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    shader: MESHLET_TEST_MATERIAL_SHADER_HANDLE,
                    shader_defs,
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
            }),
        }
    }
}

impl MeshletTestMaterial {
    pub fn get(world: &World) -> Option<&RenderPipeline> {
        let pipeline_cache = world.get_resource::<PipelineCache>()?;
        let pipeline = world.get_resource::<Self>()?;
        pipeline_cache.get_render_pipeline(pipeline.pipeline)
    }
}
