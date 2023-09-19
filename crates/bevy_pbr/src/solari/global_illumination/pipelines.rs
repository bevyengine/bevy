use super::{
    view_resources::create_bind_group_layouts, SolariGlobalIlluminationSettings,
    SOLARI_SCREEN_PROBES_FILTER_SHADER, SOLARI_SCREEN_PROBES_INTEPOLATE_SHADER,
    SOLARI_SCREEN_PROBES_REPROJECT_SHADER, SOLARI_SCREEN_PROBES_TRACE_SHADER,
    SOLARI_WORLD_CACHE_COMPACT_SHADER, SOLARI_WORLD_CACHE_UPDATE_SHADER,
};
use crate::solari::{
    global_illumination::SOLARI_SCREEN_PROBES_MERGE_CASCADES_SHADER,
    scene::SolariSceneBindGroupLayout,
};
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
    PushConstantRange, ShaderStages, SpecializedComputePipeline, SpecializedComputePipelines,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum SolariGlobalIlluminationPass {
    DecayWorldCache,
    CompactWorldCacheSingleBlock,
    CompactWorldCacheBlocks,
    CompactWorldWriteActiveCells,
    SampleForWorldCache,
    BlendNewWorldCacheSamples,
    ScreenProbesReproject,
    ScreenProbesTrace,
    ScreenProbesMergeCascades,
    ScreenProbesFilterFirstPass,
    ScreenProbesFilterSecondPass,
    ScreenProbesInterpolate,
}

#[derive(Resource)]
pub struct SolariGlobalIlluminationPipelines {
    scene_bind_group_layout: BindGroupLayout,
    pub view_bind_group_layout: BindGroupLayout,
    pub view_with_world_cache_dispatch_bind_group_layout: BindGroupLayout,
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
        use SolariGlobalIlluminationPass::*;

        let mut view_layout = &self.view_bind_group_layout;
        let mut push_constant_ranges = vec![];
        let mut shader_defs = vec![];

        let (entry_point, shader) = match pass {
            DecayWorldCache => {
                view_layout = &self.view_with_world_cache_dispatch_bind_group_layout;
                shader_defs.extend_from_slice(&[
                    "INCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH".into(),
                    "WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into(),
                ]);
                ("decay_world_cache", SOLARI_WORLD_CACHE_COMPACT_SHADER)
            }
            CompactWorldCacheSingleBlock => {
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
            CompactWorldCacheBlocks => {
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
            CompactWorldWriteActiveCells => {
                view_layout = &self.view_with_world_cache_dispatch_bind_group_layout;
                shader_defs.extend_from_slice(&[
                    "INCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH".into(),
                    "WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into(),
                ]);
                (
                    "compact_world_cache_write_active_cells",
                    SOLARI_WORLD_CACHE_COMPACT_SHADER,
                )
            }
            SampleForWorldCache => ("sample_irradiance", SOLARI_WORLD_CACHE_UPDATE_SHADER),
            BlendNewWorldCacheSamples => ("blend_new_samples", SOLARI_WORLD_CACHE_UPDATE_SHADER),
            ScreenProbesReproject => (
                "reproject_screen_probes",
                SOLARI_SCREEN_PROBES_REPROJECT_SHADER,
            ),
            ScreenProbesTrace => ("trace_screen_probes", SOLARI_SCREEN_PROBES_TRACE_SHADER),
            ScreenProbesFilterFirstPass => {
                shader_defs.push("FIRST_PASS".into());
                ("filter_screen_probes", SOLARI_SCREEN_PROBES_FILTER_SHADER)
            }
            ScreenProbesFilterSecondPass => {
                ("filter_screen_probes", SOLARI_SCREEN_PROBES_FILTER_SHADER)
            }
            ScreenProbesMergeCascades => {
                push_constant_ranges.push(PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                });
                (
                    "merge_screen_probe_cascades",
                    SOLARI_SCREEN_PROBES_MERGE_CASCADES_SHADER,
                )
            }
            ScreenProbesInterpolate => (
                "interpolate_screen_probes",
                SOLARI_SCREEN_PROBES_INTEPOLATE_SHADER,
            ),
        };

        ComputePipelineDescriptor {
            label: Some(format!("solari_global_illumination_{entry_point}_pipeline").into()),
            layout: vec![self.scene_bind_group_layout.clone(), view_layout.clone()],
            push_constant_ranges,
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
    pub compact_world_cache_write_active_cells: CachedComputePipelineId,
    pub sample_for_world_cache: CachedComputePipelineId,
    pub blend_new_world_cache_samples: CachedComputePipelineId,
    pub screen_probes_reproject: CachedComputePipelineId,
    pub screen_probes_trace: CachedComputePipelineId,
    pub screen_probes_merge_cascades: CachedComputePipelineId,
    pub screen_probes_filter_first_pass: CachedComputePipelineId,
    pub screen_probes_filter_second_pass: CachedComputePipelineId,
    pub screen_probes_interpolate: CachedComputePipelineId,
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
    use SolariGlobalIlluminationPass::*;

    let mut create_pipeline = |key| pipelines.specialize(&pipeline_cache, &pipeline, key);

    for entity in &views {
        commands
            .entity(entity)
            .insert(SolariGlobalIlluminationPipelineIds {
                decay_world_cache: create_pipeline(DecayWorldCache),
                compact_world_cache_single_block: create_pipeline(CompactWorldCacheSingleBlock),
                compact_world_cache_blocks: create_pipeline(CompactWorldCacheBlocks),
                compact_world_cache_write_active_cells: create_pipeline(
                    CompactWorldWriteActiveCells,
                ),
                sample_for_world_cache: create_pipeline(SampleForWorldCache),
                blend_new_world_cache_samples: create_pipeline(BlendNewWorldCacheSamples),
                screen_probes_reproject: create_pipeline(ScreenProbesReproject),
                screen_probes_trace: create_pipeline(ScreenProbesTrace),
                screen_probes_merge_cascades: create_pipeline(ScreenProbesMergeCascades),
                screen_probes_filter_first_pass: create_pipeline(ScreenProbesFilterFirstPass),
                screen_probes_filter_second_pass: create_pipeline(ScreenProbesFilterSecondPass),
                screen_probes_interpolate: create_pipeline(ScreenProbesInterpolate),
            });
    }
}
