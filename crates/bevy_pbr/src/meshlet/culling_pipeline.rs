use super::gpu_scene::MeshletGpuScene;
use bevy_asset::Handle;
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_render::render_resource::{
    CachedComputePipelineId, ComputePipeline, ComputePipelineDescriptor, PipelineCache, Shader,
    ShaderDefVal,
};

pub const MESHLET_CULLING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4325134235233421);

#[derive(Resource)]
pub struct MeshletCullingPipeline {
    pipeline: CachedComputePipelineId,
}

impl FromWorld for MeshletCullingPipeline {
    fn from_world(world: &mut World) -> Self {
        let gpu_scene = world.resource::<MeshletGpuScene>();
        let layout = gpu_scene.culling_bind_group_layout().clone();
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            pipeline: pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
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
        }
    }
}

impl MeshletCullingPipeline {
    pub fn get(world: &World) -> Option<&ComputePipeline> {
        let pipeline_cache = world.get_resource::<PipelineCache>()?;
        let pipeline = world.get_resource::<Self>()?;
        pipeline_cache.get_compute_pipeline(pipeline.pipeline)
    }
}
