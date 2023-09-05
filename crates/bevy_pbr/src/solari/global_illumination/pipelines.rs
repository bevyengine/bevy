use super::{
    view_resources::create_bind_group_layouts, SolariGlobalIlluminationSettings,
    SOLARI_DENOISE_DIFFUSE_SHADER, SOLARI_FILTER_SCREEN_PROBES_SHADER,
    SOLARI_INTEPOLATE_SCREEN_PROBES_SHADER, SOLARI_UPDATE_SCREEN_PROBES_SHADER,
    SOLARI_WORLD_CACHE_COMPACT_SHADER, SOLARI_WORLD_CACHE_UPDATE_SHADER,
};
use crate::solari::scene::SolariSceneBindGroupLayout;
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::render_resource::{
    BindGroupLayout, CachedComputePipelineId, ComputePipelineDescriptor, PipelineCache,
    SpecializedComputePipeline, SpecializedComputePipelines,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum SolariGlobalIlluminationPass {
    DecayWorldCache,
    CompactWorldCacheSingleBlock,
    CompactWorldCacheBlocks,
    SampleForWorldCache,
    BlendNewWorldCacheSamples,
    UpdateScreenProbes,
    FilterScreenProbes,
    InterpolateScreenProbes,
    DenoiseDiffuseTemporal,
    DenoiseDiffuseSpatial,
}

#[derive(Resource)]
pub struct SolariGlobalIlluminationPipelines {
    scene_bind_group_layout: BindGroupLayout,
    view_bind_group_layout: BindGroupLayout,
    view_with_world_cache_dispatch_bind_group_layout: BindGroupLayout,
}

impl FromWorld for SolariGlobalIlluminationPipelines {
    fn from_world(world: &mut World) -> Self {
        let scene_bind_group_layout = world.resource::<SolariSceneBindGroupLayout>();
        let (view_bind_group_layout, view_with_world_cache_dispatch_bind_group_layout) =
            create_bind_group_layouts(world.resource());

        Self {
            scene_bind_group_layout: scene_bind_group_layout.0.clone(),
            view_bind_group_layout,
            view_with_world_cache_dispatch_bind_group_layout,
        }
    }
}

impl SpecializedComputePipeline for SolariGlobalIlluminationPipelines {
    type Key = SolariGlobalIlluminationPass;

    fn specialize(&self, pass: Self::Key) -> ComputePipelineDescriptor {
        let mut view_layout = &self.view_bind_group_layout;
        let mut shader_defs = vec![];
        let (entry_point, shader) = match pass {
            SolariGlobalIlluminationPass::DecayWorldCache => {
                view_layout = &self.view_with_world_cache_dispatch_bind_group_layout;
                shader_defs.extend_from_slice(&[
                    "INCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH".into(),
                    "WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into(),
                ]);
                ("decay_world_cache", SOLARI_WORLD_CACHE_COMPACT_SHADER)
            }
            SolariGlobalIlluminationPass::CompactWorldCacheSingleBlock => {
                view_layout = &self.view_with_world_cache_dispatch_bind_group_layout;
                shader_defs.extend_from_slice(&[
                    "INCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH".into(),
                    "WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into(),
                ]);
                (
                    "compact_world_cache_single_block",
                    SOLARI_WORLD_CACHE_COMPACT_SHADER,
                )
            }
            SolariGlobalIlluminationPass::CompactWorldCacheBlocks => {
                view_layout = &self.view_with_world_cache_dispatch_bind_group_layout;
                shader_defs.extend_from_slice(&[
                    "INCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH".into(),
                    "WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into(),
                ]);
                (
                    "compact_world_cache_blocks",
                    SOLARI_WORLD_CACHE_COMPACT_SHADER,
                )
            }
            SolariGlobalIlluminationPass::SampleForWorldCache => {
                ("sample_irradiance", SOLARI_WORLD_CACHE_UPDATE_SHADER)
            }
            SolariGlobalIlluminationPass::BlendNewWorldCacheSamples => {
                ("blend_new_samples", SOLARI_WORLD_CACHE_UPDATE_SHADER)
            }
            SolariGlobalIlluminationPass::UpdateScreenProbes => {
                ("update_screen_probes", SOLARI_UPDATE_SCREEN_PROBES_SHADER)
            }
            SolariGlobalIlluminationPass::FilterScreenProbes => {
                ("filter_screen_probes", SOLARI_FILTER_SCREEN_PROBES_SHADER)
            }
            SolariGlobalIlluminationPass::InterpolateScreenProbes => (
                "interpolate_screen_probes",
                SOLARI_INTEPOLATE_SCREEN_PROBES_SHADER,
            ),
            SolariGlobalIlluminationPass::DenoiseDiffuseTemporal => {
                ("denoise_diffuse_temporal", SOLARI_DENOISE_DIFFUSE_SHADER)
            }
            SolariGlobalIlluminationPass::DenoiseDiffuseSpatial => {
                ("denoise_diffuse_spatial", SOLARI_DENOISE_DIFFUSE_SHADER)
            }
        };

        ComputePipelineDescriptor {
            label: Some(format!("solari_global_illumination_{entry_point}_pipeline").into()),
            layout: vec![self.scene_bind_group_layout.clone(), view_layout.clone()],
            push_constant_ranges: vec![],
            shader: shader.typed(),
            shader_defs,
            entry_point: entry_point.into(),
        }
    }
}

#[derive(Component)]
pub struct SolariGlobalIlluminationPipelineIds {
    pub decay_world_cache: CachedComputePipelineId,
    pub compact_world_cache_single_block: CachedComputePipelineId,
    pub compact_world_cache_blocks: CachedComputePipelineId,
    pub sample_for_world_cache: CachedComputePipelineId,
    pub blend_new_world_cache_samples: CachedComputePipelineId,
    pub update_screen_probes: CachedComputePipelineId,
    pub filter_screen_probes: CachedComputePipelineId,
    pub denoise_diffuse_temporal: CachedComputePipelineId,
    pub denoiser_diffuse_spatial: CachedComputePipelineId,
}

pub fn prepare_pipelines(
    views: Query<
        Entity,
        (
            With<SolariGlobalIlluminationSettings>,
            With<DepthPrepass>,
            With<NormalPrepass>,
            With<MotionVectorPrepass>,
        ),
    >,
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<SolariGlobalIlluminationPipelines>>,
    pipeline: Res<SolariGlobalIlluminationPipelines>,
) {
    let mut create_pipeline = |key| pipelines.specialize(&pipeline_cache, &pipeline, key);

    for entity in &views {
        commands
            .entity(entity)
            .insert(SolariGlobalIlluminationPipelineIds {
                decay_world_cache: create_pipeline(SolariGlobalIlluminationPass::DecayWorldCache),
                compact_world_cache_single_block: create_pipeline(
                    SolariGlobalIlluminationPass::CompactWorldCacheSingleBlock,
                ),
                compact_world_cache_blocks: create_pipeline(
                    SolariGlobalIlluminationPass::CompactWorldCacheBlocks,
                ),
                sample_for_world_cache: create_pipeline(
                    SolariGlobalIlluminationPass::SampleForWorldCache,
                ),
                blend_new_world_cache_samples: create_pipeline(
                    SolariGlobalIlluminationPass::BlendNewWorldCacheSamples,
                ),
                update_screen_probes: create_pipeline(
                    SolariGlobalIlluminationPass::UpdateScreenProbes,
                ),
                filter_screen_probes: create_pipeline(
                    SolariGlobalIlluminationPass::FilterScreenProbes,
                ),
                denoise_diffuse_temporal: create_pipeline(
                    SolariGlobalIlluminationPass::DenoiseDiffuseTemporal,
                ),
                denoiser_diffuse_spatial: create_pipeline(
                    SolariGlobalIlluminationPass::DenoiseDiffuseSpatial,
                ),
            });
    }
}
