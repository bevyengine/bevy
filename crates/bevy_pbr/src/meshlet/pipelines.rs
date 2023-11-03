use super::gpu_scene::MeshletGpuScene;
use bevy_asset::Handle;
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_render::render_resource::{
    CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline, ComputePipelineDescriptor,
    MultisampleState, PipelineCache, PrimitiveState, RenderPipeline, RenderPipelineDescriptor,
    Shader, ShaderDefVal,
};

pub const MESHLET_CULLING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4325134235233421);
pub const MESHLET_VISIBILITY_BUFFER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(5325134235233421);

#[derive(Resource)]
pub struct MeshletPipelines {
    cull: CachedComputePipelineId,
    visibility_buffer: CachedRenderPipelineId,
}

impl FromWorld for MeshletPipelines {
    fn from_world(world: &mut World) -> Self {
        let gpu_scene = world.resource::<MeshletGpuScene>();
        let layout = gpu_scene.culling_bind_group_layout().clone();
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            cull: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("meshlet_culling_pipeline".into()),
                layout: vec![layout],
                push_constant_ranges: vec![],
                shader: MESHLET_CULLING_SHADER_HANDLE,
                shader_defs: vec![
                    "MESHLET_CULLING_BINDINGS".into(),
                    ShaderDefVal::UInt("MESHLET_BIND_GROUP".into(), 0),
                ],
                entry_point: "cull_meshlets".into(),
            }),

            visibility_buffer: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("meshlet_visibility_buffer_pipeline"),
                layout: vec![layout],
                push_constant_ranges: vec![],
                vertex: todo!(),
                primitive: PrimitiveState {
                    topology: todo!(),
                    strip_index_format: todo!(),
                    front_face: todo!(),
                    cull_mode: todo!(),
                    unclipped_depth: todo!(),
                    polygon_mode: todo!(),
                    conservative: todo!(),
                },
                depth_stencil: todo!(),
                multisample: todo!(),
                fragment: todo!(),
            }),
        }
    }
}

impl MeshletPipelines {
    pub fn get(world: &World) -> (Option<&ComputePipeline>, Option<&RenderPipeline>) {
        let pipeline_cache = world.get_resource::<PipelineCache>()?;
        let pipeline = world.get_resource::<Self>()?;
        (
            pipeline_cache.get_compute_pipeline(pipeline.cull),
            pipeline_cache.get_render_pipeline(pipeline.visibility_buffer),
        )
    }
}
