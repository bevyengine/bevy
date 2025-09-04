//! GPU mesh preprocessing.
//!
//! This is an optional pass that uses a compute shader to reduce the amount of
//! data that has to be transferred from the CPU to the GPU. When enabled,
//! instead of transferring [`MeshUniform`]s to the GPU, we transfer the smaller
//! [`MeshInputUniform`]s instead and use the GPU to calculate the remaining
//! derived fields in [`MeshUniform`].

use core::num::{NonZero, NonZeroU64};

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, Handle};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    experimental::mip_generation::ViewDepthPyramid,
    prepass::{DepthPrepass, PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::resource_exists,
    query::{Has, Or, QueryState, With, Without},
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_render::{
    batching::gpu_preprocessing::{
        BatchedInstanceBuffers, GpuOcclusionCullingWorkItemBuffers, GpuPreprocessingMode,
        GpuPreprocessingSupport, IndirectBatchSet, IndirectParametersBuffers,
        IndirectParametersCpuMetadata, IndirectParametersGpuMetadata, IndirectParametersIndexed,
        IndirectParametersNonIndexed, LatePreprocessWorkItemIndirectParameters, PreprocessWorkItem,
        PreprocessWorkItemBuffers, UntypedPhaseBatchedInstanceBuffers,
        UntypedPhaseIndirectParametersBuffers,
    },
    diagnostic::RecordDiagnostics,
    experimental::occlusion_culling::OcclusionCulling,
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderGraphExt},
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only, texture_2d, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindingResource, Buffer, BufferBinding,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor,
        DynamicBindGroupLayoutEntries, PipelineCache, PushConstantRange, RawBufferVec,
        ShaderStages, ShaderType, SpecializedComputePipeline, SpecializedComputePipelines,
        TextureSampleType, UninitBufferVec,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    view::{ExtractedView, NoIndirectDrawing, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::{default, TypeIdMap};
use bitflags::bitflags;
use smallvec::{smallvec, SmallVec};
use tracing::warn;

use crate::{
    graph::NodePbr, MeshCullingData, MeshCullingDataBuffer, MeshInputUniform, MeshUniform,
};

use super::{ShadowView, ViewLightEntities};

/// The GPU workgroup size.
const WORKGROUP_SIZE: usize = 64;

/// A plugin that builds mesh uniforms on GPU.
///
/// This will only be added if the platform supports compute shaders (e.g. not
/// on WebGL 2).
pub struct GpuMeshPreprocessPlugin {
    /// Whether we're building [`MeshUniform`]s on GPU.
    ///
    /// This requires compute shader support and so will be forcibly disabled if
    /// the platform doesn't support those.
    pub use_gpu_instance_buffer_builder: bool,
}

/// The render node that clears out the GPU-side indirect metadata buffers.
///
/// This is only used when indirect drawing is enabled.
#[derive(Default)]
pub struct ClearIndirectParametersMetadataNode;

/// The render node for the first mesh preprocessing pass.
///
/// This pass runs a compute shader to cull meshes outside the view frustum (if
/// that wasn't done by the CPU), cull meshes that weren't visible last frame
/// (if occlusion culling is on), transform them, and, if indirect drawing is
/// on, populate indirect draw parameter metadata for the subsequent
/// [`EarlyPrepassBuildIndirectParametersNode`].
pub struct EarlyGpuPreprocessNode {
    view_query: QueryState<
        (
            Read<ExtractedView>,
            Option<Read<PreprocessBindGroups>>,
            Option<Read<ViewUniformOffset>>,
            Has<NoIndirectDrawing>,
            Has<OcclusionCulling>,
        ),
        Without<SkipGpuPreprocess>,
    >,
    main_view_query: QueryState<Read<ViewLightEntities>>,
}

/// The render node for the second mesh preprocessing pass.
///
/// This pass runs a compute shader to cull meshes outside the view frustum (if
/// that wasn't done by the CPU), cull meshes that were neither visible last
/// frame nor visible this frame (if occlusion culling is on), transform them,
/// and, if indirect drawing is on, populate the indirect draw parameter
/// metadata for the subsequent [`LatePrepassBuildIndirectParametersNode`].
pub struct LateGpuPreprocessNode {
    view_query: QueryState<
        (
            Read<ExtractedView>,
            Read<PreprocessBindGroups>,
            Read<ViewUniformOffset>,
        ),
        (
            Without<SkipGpuPreprocess>,
            Without<NoIndirectDrawing>,
            With<OcclusionCulling>,
            With<DepthPrepass>,
        ),
    >,
}

/// The render node for the part of the indirect parameter building pass that
/// draws the meshes visible from the previous frame.
///
/// This node runs a compute shader on the output of the
/// [`EarlyGpuPreprocessNode`] in order to transform the
/// [`IndirectParametersGpuMetadata`] into properly-formatted
/// [`IndirectParametersIndexed`] and [`IndirectParametersNonIndexed`].
pub struct EarlyPrepassBuildIndirectParametersNode {
    view_query: QueryState<
        Read<PreprocessBindGroups>,
        (
            Without<SkipGpuPreprocess>,
            Without<NoIndirectDrawing>,
            Or<(With<DepthPrepass>, With<ShadowView>)>,
        ),
    >,
}

/// The render node for the part of the indirect parameter building pass that
/// draws the meshes that are potentially visible on this frame but weren't
/// visible on the previous frame.
///
/// This node runs a compute shader on the output of the
/// [`LateGpuPreprocessNode`] in order to transform the
/// [`IndirectParametersGpuMetadata`] into properly-formatted
/// [`IndirectParametersIndexed`] and [`IndirectParametersNonIndexed`].
pub struct LatePrepassBuildIndirectParametersNode {
    view_query: QueryState<
        Read<PreprocessBindGroups>,
        (
            Without<SkipGpuPreprocess>,
            Without<NoIndirectDrawing>,
            Or<(With<DepthPrepass>, With<ShadowView>)>,
            With<OcclusionCulling>,
        ),
    >,
}

/// The render node for the part of the indirect parameter building pass that
/// draws all meshes, both those that are newly-visible on this frame and those
/// that were visible last frame.
///
/// This node runs a compute shader on the output of the
/// [`EarlyGpuPreprocessNode`] and [`LateGpuPreprocessNode`] in order to
/// transform the [`IndirectParametersGpuMetadata`] into properly-formatted
/// [`IndirectParametersIndexed`] and [`IndirectParametersNonIndexed`].
pub struct MainBuildIndirectParametersNode {
    view_query: QueryState<
        Read<PreprocessBindGroups>,
        (Without<SkipGpuPreprocess>, Without<NoIndirectDrawing>),
    >,
}

/// The compute shader pipelines for the GPU mesh preprocessing and indirect
/// parameter building passes.
#[derive(Resource)]
pub struct PreprocessPipelines {
    /// The pipeline used for CPU culling. This pipeline doesn't populate
    /// indirect parameter metadata.
    pub direct_preprocess: PreprocessPipeline,
    /// The pipeline used for mesh preprocessing when GPU frustum culling is in
    /// use, but occlusion culling isn't.
    ///
    /// This pipeline populates indirect parameter metadata.
    pub gpu_frustum_culling_preprocess: PreprocessPipeline,
    /// The pipeline used for the first phase of occlusion culling.
    ///
    /// This pipeline culls, transforms meshes, and populates indirect parameter
    /// metadata.
    pub early_gpu_occlusion_culling_preprocess: PreprocessPipeline,
    /// The pipeline used for the second phase of occlusion culling.
    ///
    /// This pipeline culls, transforms meshes, and populates indirect parameter
    /// metadata.
    pub late_gpu_occlusion_culling_preprocess: PreprocessPipeline,
    /// The pipeline that builds indirect draw parameters for indexed meshes,
    /// when frustum culling is enabled but occlusion culling *isn't* enabled.
    pub gpu_frustum_culling_build_indexed_indirect_params: BuildIndirectParametersPipeline,
    /// The pipeline that builds indirect draw parameters for non-indexed
    /// meshes, when frustum culling is enabled but occlusion culling *isn't*
    /// enabled.
    pub gpu_frustum_culling_build_non_indexed_indirect_params: BuildIndirectParametersPipeline,
    /// Compute shader pipelines for the early prepass phase that draws meshes
    /// visible in the previous frame.
    pub early_phase: PreprocessPhasePipelines,
    /// Compute shader pipelines for the late prepass phase that draws meshes
    /// that weren't visible in the previous frame, but became visible this
    /// frame.
    pub late_phase: PreprocessPhasePipelines,
    /// Compute shader pipelines for the main color phase.
    pub main_phase: PreprocessPhasePipelines,
}

/// Compute shader pipelines for a specific phase: early, late, or main.
///
/// The distinction between these phases is relevant for occlusion culling.
#[derive(Clone)]
pub struct PreprocessPhasePipelines {
    /// The pipeline that resets the indirect draw counts used in
    /// `multi_draw_indirect_count` to 0 in preparation for a new pass.
    pub reset_indirect_batch_sets: ResetIndirectBatchSetsPipeline,
    /// The pipeline used for indexed indirect parameter building.
    ///
    /// This pipeline converts indirect parameter metadata into indexed indirect
    /// parameters.
    pub gpu_occlusion_culling_build_indexed_indirect_params: BuildIndirectParametersPipeline,
    /// The pipeline used for non-indexed indirect parameter building.
    ///
    /// This pipeline converts indirect parameter metadata into non-indexed
    /// indirect parameters.
    pub gpu_occlusion_culling_build_non_indexed_indirect_params: BuildIndirectParametersPipeline,
}

/// The pipeline for the GPU mesh preprocessing shader.
pub struct PreprocessPipeline {
    /// The bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The shader asset handle.
    pub shader: Handle<Shader>,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in `prepare_preprocess_pipelines`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

/// The pipeline for the batch set count reset shader.
///
/// This shader resets the indirect batch set count to 0 for each view. It runs
/// in between every phase (early, late, and main).
#[derive(Clone)]
pub struct ResetIndirectBatchSetsPipeline {
    /// The bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The shader asset handle.
    pub shader: Handle<Shader>,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in `prepare_preprocess_pipelines`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

/// The pipeline for the indirect parameter building shader.
#[derive(Clone)]
pub struct BuildIndirectParametersPipeline {
    /// The bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The shader asset handle.
    pub shader: Handle<Shader>,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in `prepare_preprocess_pipelines`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

bitflags! {
    /// Specifies variants of the mesh preprocessing shader.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PreprocessPipelineKey: u8 {
        /// Whether GPU frustum culling is in use.
        ///
        /// This `#define`'s `FRUSTUM_CULLING` in the shader.
        const FRUSTUM_CULLING = 1;
        /// Whether GPU two-phase occlusion culling is in use.
        ///
        /// This `#define`'s `OCCLUSION_CULLING` in the shader.
        const OCCLUSION_CULLING = 2;
        /// Whether this is the early phase of GPU two-phase occlusion culling.
        ///
        /// This `#define`'s `EARLY_PHASE` in the shader.
        const EARLY_PHASE = 4;
    }

    /// Specifies variants of the indirect parameter building shader.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BuildIndirectParametersPipelineKey: u8 {
        /// Whether the indirect parameter building shader is processing indexed
        /// meshes (those that have index buffers).
        ///
        /// This defines `INDEXED` in the shader.
        const INDEXED = 1;
        /// Whether the GPU and driver supports `multi_draw_indirect_count`.
        ///
        /// This defines `MULTI_DRAW_INDIRECT_COUNT_SUPPORTED` in the shader.
        const MULTI_DRAW_INDIRECT_COUNT_SUPPORTED = 2;
        /// Whether GPU two-phase occlusion culling is in use.
        ///
        /// This `#define`'s `OCCLUSION_CULLING` in the shader.
        const OCCLUSION_CULLING = 4;
        /// Whether this is the early phase of GPU two-phase occlusion culling.
        ///
        /// This `#define`'s `EARLY_PHASE` in the shader.
        const EARLY_PHASE = 8;
        /// Whether this is the late phase of GPU two-phase occlusion culling.
        ///
        /// This `#define`'s `LATE_PHASE` in the shader.
        const LATE_PHASE = 16;
        /// Whether this is the phase that runs after the early and late phases,
        /// and right before the main drawing logic, when GPU two-phase
        /// occlusion culling is in use.
        ///
        /// This `#define`'s `MAIN_PHASE` in the shader.
        const MAIN_PHASE = 32;
    }
}

/// The compute shader bind group for the mesh preprocessing pass for each
/// render phase.
///
/// This goes on the view. It maps the [`core::any::TypeId`] of a render phase
/// (e.g.  [`bevy_core_pipeline::core_3d::Opaque3d`]) to the
/// [`PhasePreprocessBindGroups`] for that phase.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct PreprocessBindGroups(pub TypeIdMap<PhasePreprocessBindGroups>);

/// The compute shader bind group for the mesh preprocessing step for a single
/// render phase on a single view.
#[derive(Clone)]
pub enum PhasePreprocessBindGroups {
    /// The bind group used for the single invocation of the compute shader when
    /// indirect drawing is *not* being used.
    ///
    /// Because direct drawing doesn't require splitting the meshes into indexed
    /// and non-indexed meshes, there's only one bind group in this case.
    Direct(BindGroup),

    /// The bind groups used for the compute shader when indirect drawing is
    /// being used, but occlusion culling isn't being used.
    ///
    /// Because indirect drawing requires splitting the meshes into indexed and
    /// non-indexed meshes, there are two bind groups here.
    IndirectFrustumCulling {
        /// The bind group for indexed meshes.
        indexed: Option<BindGroup>,
        /// The bind group for non-indexed meshes.
        non_indexed: Option<BindGroup>,
    },

    /// The bind groups used for the compute shader when indirect drawing is
    /// being used, but occlusion culling isn't being used.
    ///
    /// Because indirect drawing requires splitting the meshes into indexed and
    /// non-indexed meshes, and because occlusion culling requires splitting
    /// this phase into early and late versions, there are four bind groups
    /// here.
    IndirectOcclusionCulling {
        /// The bind group for indexed meshes during the early mesh
        /// preprocessing phase.
        early_indexed: Option<BindGroup>,
        /// The bind group for non-indexed meshes during the early mesh
        /// preprocessing phase.
        early_non_indexed: Option<BindGroup>,
        /// The bind group for indexed meshes during the late mesh preprocessing
        /// phase.
        late_indexed: Option<BindGroup>,
        /// The bind group for non-indexed meshes during the late mesh
        /// preprocessing phase.
        late_non_indexed: Option<BindGroup>,
    },
}

/// The bind groups for the compute shaders that reset indirect draw counts and
/// build indirect parameters.
///
/// There's one set of bind group for each phase. Phases are keyed off their
/// [`core::any::TypeId`].
#[derive(Resource, Default, Deref, DerefMut)]
pub struct BuildIndirectParametersBindGroups(pub TypeIdMap<PhaseBuildIndirectParametersBindGroups>);

impl BuildIndirectParametersBindGroups {
    /// Creates a new, empty [`BuildIndirectParametersBindGroups`] table.
    pub fn new() -> BuildIndirectParametersBindGroups {
        Self::default()
    }
}

/// The per-phase set of bind groups for the compute shaders that reset indirect
/// draw counts and build indirect parameters.
pub struct PhaseBuildIndirectParametersBindGroups {
    /// The bind group for the `reset_indirect_batch_sets.wgsl` shader, for
    /// indexed meshes.
    reset_indexed_indirect_batch_sets: Option<BindGroup>,
    /// The bind group for the `reset_indirect_batch_sets.wgsl` shader, for
    /// non-indexed meshes.
    reset_non_indexed_indirect_batch_sets: Option<BindGroup>,
    /// The bind group for the `build_indirect_params.wgsl` shader, for indexed
    /// meshes.
    build_indexed_indirect: Option<BindGroup>,
    /// The bind group for the `build_indirect_params.wgsl` shader, for
    /// non-indexed meshes.
    build_non_indexed_indirect: Option<BindGroup>,
}

/// Stops the `GpuPreprocessNode` attempting to generate the buffer for this view
/// useful to avoid duplicating effort if the bind group is shared between views
#[derive(Component, Default)]
pub struct SkipGpuPreprocess;

impl Plugin for GpuMeshPreprocessPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "mesh_preprocess.wgsl");
        embedded_asset!(app, "reset_indirect_batch_sets.wgsl");
        embedded_asset!(app, "build_indirect_params.wgsl");
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // This plugin does nothing if GPU instance buffer building isn't in
        // use.
        let gpu_preprocessing_support = render_app.world().resource::<GpuPreprocessingSupport>();
        if !self.use_gpu_instance_buffer_builder || !gpu_preprocessing_support.is_available() {
            return;
        }

        render_app
            .init_resource::<PreprocessPipelines>()
            .init_resource::<SpecializedComputePipelines<PreprocessPipeline>>()
            .init_resource::<SpecializedComputePipelines<ResetIndirectBatchSetsPipeline>>()
            .init_resource::<SpecializedComputePipelines<BuildIndirectParametersPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_preprocess_pipelines.in_set(RenderSystems::Prepare),
                    prepare_preprocess_bind_groups
                        .run_if(resource_exists::<BatchedInstanceBuffers<
                            MeshUniform,
                            MeshInputUniform
                        >>)
                        .in_set(RenderSystems::PrepareBindGroups),
                    write_mesh_culling_data_buffer.in_set(RenderSystems::PrepareResourcesFlush),
                ),
            )
            .add_render_graph_node::<ClearIndirectParametersMetadataNode>(
                Core3d,
                NodePbr::ClearIndirectParametersMetadata
            )
            .add_render_graph_node::<EarlyGpuPreprocessNode>(Core3d, NodePbr::EarlyGpuPreprocess)
            .add_render_graph_node::<LateGpuPreprocessNode>(Core3d, NodePbr::LateGpuPreprocess)
            .add_render_graph_node::<EarlyPrepassBuildIndirectParametersNode>(
                Core3d,
                NodePbr::EarlyPrepassBuildIndirectParameters,
            )
            .add_render_graph_node::<LatePrepassBuildIndirectParametersNode>(
                Core3d,
                NodePbr::LatePrepassBuildIndirectParameters,
            )
            .add_render_graph_node::<MainBuildIndirectParametersNode>(
                Core3d,
                NodePbr::MainBuildIndirectParameters,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    NodePbr::ClearIndirectParametersMetadata,
                    NodePbr::EarlyGpuPreprocess,
                    NodePbr::EarlyPrepassBuildIndirectParameters,
                    Node3d::EarlyPrepass,
                    Node3d::EarlyDeferredPrepass,
                    Node3d::EarlyDownsampleDepth,
                    NodePbr::LateGpuPreprocess,
                    NodePbr::LatePrepassBuildIndirectParameters,
                    Node3d::LatePrepass,
                    Node3d::LateDeferredPrepass,
                    NodePbr::MainBuildIndirectParameters,
                    Node3d::StartMainPass,
                ),
            ).add_render_graph_edges(
                Core3d,
                (
                    NodePbr::EarlyPrepassBuildIndirectParameters,
                    NodePbr::EarlyShadowPass,
                    Node3d::EarlyDownsampleDepth,
                )
            ).add_render_graph_edges(
                Core3d,
                (
                    NodePbr::LatePrepassBuildIndirectParameters,
                    NodePbr::LateShadowPass,
                    NodePbr::MainBuildIndirectParameters,
                )
            );
    }
}

impl Node for ClearIndirectParametersMetadataNode {
    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(indirect_parameters_buffers) = world.get_resource::<IndirectParametersBuffers>()
        else {
            return Ok(());
        };

        // Clear out each indexed and non-indexed GPU-side buffer.
        for phase_indirect_parameters_buffers in indirect_parameters_buffers.values() {
            if let Some(indexed_gpu_metadata_buffer) = phase_indirect_parameters_buffers
                .indexed
                .gpu_metadata_buffer()
            {
                render_context.command_encoder().clear_buffer(
                    indexed_gpu_metadata_buffer,
                    0,
                    Some(
                        phase_indirect_parameters_buffers.indexed.batch_count() as u64
                            * size_of::<IndirectParametersGpuMetadata>() as u64,
                    ),
                );
            }

            if let Some(non_indexed_gpu_metadata_buffer) = phase_indirect_parameters_buffers
                .non_indexed
                .gpu_metadata_buffer()
            {
                render_context.command_encoder().clear_buffer(
                    non_indexed_gpu_metadata_buffer,
                    0,
                    Some(
                        phase_indirect_parameters_buffers.non_indexed.batch_count() as u64
                            * size_of::<IndirectParametersGpuMetadata>() as u64,
                    ),
                );
            }
        }

        Ok(())
    }
}

impl FromWorld for EarlyGpuPreprocessNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
            main_view_query: QueryState::new(world),
        }
    }
}

impl Node for EarlyGpuPreprocessNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
        self.main_view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let diagnostics = render_context.diagnostic_recorder();

        // Grab the [`BatchedInstanceBuffers`].
        let batched_instance_buffers =
            world.resource::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("early_mesh_preprocessing"),
                    timestamp_writes: None,
                });
        let pass_span = diagnostics.time_span(&mut compute_pass, "early_mesh_preprocessing");

        let mut all_views: SmallVec<[_; 8]> = SmallVec::new();
        all_views.push(graph.view_entity());
        if let Ok(shadow_cascade_views) =
            self.main_view_query.get_manual(world, graph.view_entity())
        {
            all_views.extend(shadow_cascade_views.lights.iter().copied());
        }

        // Run the compute passes.

        for view_entity in all_views {
            let Ok((
                view,
                bind_groups,
                view_uniform_offset,
                no_indirect_drawing,
                occlusion_culling,
            )) = self.view_query.get_manual(world, view_entity)
            else {
                continue;
            };

            let Some(bind_groups) = bind_groups else {
                continue;
            };
            let Some(view_uniform_offset) = view_uniform_offset else {
                continue;
            };

            // Select the right pipeline, depending on whether GPU culling is in
            // use.
            let maybe_pipeline_id = if no_indirect_drawing {
                preprocess_pipelines.direct_preprocess.pipeline_id
            } else if occlusion_culling {
                preprocess_pipelines
                    .early_gpu_occlusion_culling_preprocess
                    .pipeline_id
            } else {
                preprocess_pipelines
                    .gpu_frustum_culling_preprocess
                    .pipeline_id
            };

            // Fetch the pipeline.
            let Some(preprocess_pipeline_id) = maybe_pipeline_id else {
                warn!("The build mesh uniforms pipeline wasn't ready");
                continue;
            };

            let Some(preprocess_pipeline) =
                pipeline_cache.get_compute_pipeline(preprocess_pipeline_id)
            else {
                // This will happen while the pipeline is being compiled and is fine.
                continue;
            };

            compute_pass.set_pipeline(preprocess_pipeline);

            // Loop over each render phase.
            for (phase_type_id, batched_phase_instance_buffers) in
                &batched_instance_buffers.phase_instance_buffers
            {
                // Grab the work item buffers for this view.
                let Some(work_item_buffers) = batched_phase_instance_buffers
                    .work_item_buffers
                    .get(&view.retained_view_entity)
                else {
                    continue;
                };

                // Fetch the bind group for the render phase.
                let Some(phase_bind_groups) = bind_groups.get(phase_type_id) else {
                    continue;
                };

                // Make sure the mesh preprocessing shader has access to the
                // view info it needs to do culling and motion vector
                // computation.
                let dynamic_offsets = [view_uniform_offset.offset];

                // Are we drawing directly or indirectly?
                match *phase_bind_groups {
                    PhasePreprocessBindGroups::Direct(ref bind_group) => {
                        // Invoke the mesh preprocessing shader to transform
                        // meshes only, but not cull.
                        let PreprocessWorkItemBuffers::Direct(work_item_buffer) = work_item_buffers
                        else {
                            continue;
                        };
                        compute_pass.set_bind_group(0, bind_group, &dynamic_offsets);
                        let workgroup_count = work_item_buffer.len().div_ceil(WORKGROUP_SIZE);
                        if workgroup_count > 0 {
                            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
                        }
                    }

                    PhasePreprocessBindGroups::IndirectFrustumCulling {
                        indexed: ref maybe_indexed_bind_group,
                        non_indexed: ref maybe_non_indexed_bind_group,
                    }
                    | PhasePreprocessBindGroups::IndirectOcclusionCulling {
                        early_indexed: ref maybe_indexed_bind_group,
                        early_non_indexed: ref maybe_non_indexed_bind_group,
                        ..
                    } => {
                        // Invoke the mesh preprocessing shader to transform and
                        // cull the meshes.
                        let PreprocessWorkItemBuffers::Indirect {
                            indexed: indexed_buffer,
                            non_indexed: non_indexed_buffer,
                            ..
                        } = work_item_buffers
                        else {
                            continue;
                        };

                        // Transform and cull indexed meshes if there are any.
                        if let Some(indexed_bind_group) = maybe_indexed_bind_group {
                            if let PreprocessWorkItemBuffers::Indirect {
                                gpu_occlusion_culling:
                                    Some(GpuOcclusionCullingWorkItemBuffers {
                                        late_indirect_parameters_indexed_offset,
                                        ..
                                    }),
                                ..
                            } = *work_item_buffers
                            {
                                compute_pass.set_push_constants(
                                    0,
                                    bytemuck::bytes_of(&late_indirect_parameters_indexed_offset),
                                );
                            }

                            compute_pass.set_bind_group(0, indexed_bind_group, &dynamic_offsets);
                            let workgroup_count = indexed_buffer.len().div_ceil(WORKGROUP_SIZE);
                            if workgroup_count > 0 {
                                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
                            }
                        }

                        // Transform and cull non-indexed meshes if there are any.
                        if let Some(non_indexed_bind_group) = maybe_non_indexed_bind_group {
                            if let PreprocessWorkItemBuffers::Indirect {
                                gpu_occlusion_culling:
                                    Some(GpuOcclusionCullingWorkItemBuffers {
                                        late_indirect_parameters_non_indexed_offset,
                                        ..
                                    }),
                                ..
                            } = *work_item_buffers
                            {
                                compute_pass.set_push_constants(
                                    0,
                                    bytemuck::bytes_of(
                                        &late_indirect_parameters_non_indexed_offset,
                                    ),
                                );
                            }

                            compute_pass.set_bind_group(
                                0,
                                non_indexed_bind_group,
                                &dynamic_offsets,
                            );
                            let workgroup_count = non_indexed_buffer.len().div_ceil(WORKGROUP_SIZE);
                            if workgroup_count > 0 {
                                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
                            }
                        }
                    }
                }
            }
        }

        pass_span.end(&mut compute_pass);

        Ok(())
    }
}

impl FromWorld for EarlyPrepassBuildIndirectParametersNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl FromWorld for LatePrepassBuildIndirectParametersNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl FromWorld for MainBuildIndirectParametersNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl FromWorld for LateGpuPreprocessNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for LateGpuPreprocessNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let diagnostics = render_context.diagnostic_recorder();

        // Grab the [`BatchedInstanceBuffers`].
        let batched_instance_buffers =
            world.resource::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("late_mesh_preprocessing"),
                    timestamp_writes: None,
                });
        let pass_span = diagnostics.time_span(&mut compute_pass, "late_mesh_preprocessing");

        // Run the compute passes.
        for (view, bind_groups, view_uniform_offset) in self.view_query.iter_manual(world) {
            let maybe_pipeline_id = preprocess_pipelines
                .late_gpu_occlusion_culling_preprocess
                .pipeline_id;

            // Fetch the pipeline.
            let Some(preprocess_pipeline_id) = maybe_pipeline_id else {
                warn!("The build mesh uniforms pipeline wasn't ready");
                return Ok(());
            };

            let Some(preprocess_pipeline) =
                pipeline_cache.get_compute_pipeline(preprocess_pipeline_id)
            else {
                // This will happen while the pipeline is being compiled and is fine.
                return Ok(());
            };

            compute_pass.set_pipeline(preprocess_pipeline);

            // Loop over each phase. Because we built the phases in parallel,
            // each phase has a separate set of instance buffers.
            for (phase_type_id, batched_phase_instance_buffers) in
                &batched_instance_buffers.phase_instance_buffers
            {
                let UntypedPhaseBatchedInstanceBuffers {
                    ref work_item_buffers,
                    ref late_indexed_indirect_parameters_buffer,
                    ref late_non_indexed_indirect_parameters_buffer,
                    ..
                } = *batched_phase_instance_buffers;

                // Grab the work item buffers for this view.
                let Some(phase_work_item_buffers) =
                    work_item_buffers.get(&view.retained_view_entity)
                else {
                    continue;
                };

                let (
                    PreprocessWorkItemBuffers::Indirect {
                        gpu_occlusion_culling:
                            Some(GpuOcclusionCullingWorkItemBuffers {
                                late_indirect_parameters_indexed_offset,
                                late_indirect_parameters_non_indexed_offset,
                                ..
                            }),
                        ..
                    },
                    Some(PhasePreprocessBindGroups::IndirectOcclusionCulling {
                        late_indexed: maybe_late_indexed_bind_group,
                        late_non_indexed: maybe_late_non_indexed_bind_group,
                        ..
                    }),
                    Some(late_indexed_indirect_parameters_buffer),
                    Some(late_non_indexed_indirect_parameters_buffer),
                ) = (
                    phase_work_item_buffers,
                    bind_groups.get(phase_type_id),
                    late_indexed_indirect_parameters_buffer.buffer(),
                    late_non_indexed_indirect_parameters_buffer.buffer(),
                )
                else {
                    continue;
                };

                let mut dynamic_offsets: SmallVec<[u32; 1]> = smallvec![];
                dynamic_offsets.push(view_uniform_offset.offset);

                // If there's no space reserved for work items, then don't
                // bother doing the dispatch, as there can't possibly be any
                // meshes of the given class (indexed or non-indexed) in this
                // phase.

                // Transform and cull indexed meshes if there are any.
                if let Some(late_indexed_bind_group) = maybe_late_indexed_bind_group {
                    compute_pass.set_push_constants(
                        0,
                        bytemuck::bytes_of(late_indirect_parameters_indexed_offset),
                    );

                    compute_pass.set_bind_group(0, late_indexed_bind_group, &dynamic_offsets);
                    compute_pass.dispatch_workgroups_indirect(
                        late_indexed_indirect_parameters_buffer,
                        (*late_indirect_parameters_indexed_offset as u64)
                            * (size_of::<LatePreprocessWorkItemIndirectParameters>() as u64),
                    );
                }

                // Transform and cull non-indexed meshes if there are any.
                if let Some(late_non_indexed_bind_group) = maybe_late_non_indexed_bind_group {
                    compute_pass.set_push_constants(
                        0,
                        bytemuck::bytes_of(late_indirect_parameters_non_indexed_offset),
                    );

                    compute_pass.set_bind_group(0, late_non_indexed_bind_group, &dynamic_offsets);
                    compute_pass.dispatch_workgroups_indirect(
                        late_non_indexed_indirect_parameters_buffer,
                        (*late_indirect_parameters_non_indexed_offset as u64)
                            * (size_of::<LatePreprocessWorkItemIndirectParameters>() as u64),
                    );
                }
            }
        }

        pass_span.end(&mut compute_pass);

        Ok(())
    }
}

impl Node for EarlyPrepassBuildIndirectParametersNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();

        // If there are no views with a depth prepass enabled, we don't need to
        // run this.
        if self.view_query.iter_manual(world).next().is_none() {
            return Ok(());
        }

        run_build_indirect_parameters_node(
            render_context,
            world,
            &preprocess_pipelines.early_phase,
            "early_prepass_indirect_parameters_building",
        )
    }
}

impl Node for LatePrepassBuildIndirectParametersNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();

        // If there are no views with occlusion culling enabled, we don't need
        // to run this.
        if self.view_query.iter_manual(world).next().is_none() {
            return Ok(());
        }

        run_build_indirect_parameters_node(
            render_context,
            world,
            &preprocess_pipelines.late_phase,
            "late_prepass_indirect_parameters_building",
        )
    }
}

impl Node for MainBuildIndirectParametersNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();

        run_build_indirect_parameters_node(
            render_context,
            world,
            &preprocess_pipelines.main_phase,
            "main_indirect_parameters_building",
        )
    }
}

fn run_build_indirect_parameters_node(
    render_context: &mut RenderContext,
    world: &World,
    preprocess_phase_pipelines: &PreprocessPhasePipelines,
    label: &'static str,
) -> Result<(), NodeRunError> {
    let Some(build_indirect_params_bind_groups) =
        world.get_resource::<BuildIndirectParametersBindGroups>()
    else {
        return Ok(());
    };

    let diagnostics = render_context.diagnostic_recorder();

    let pipeline_cache = world.resource::<PipelineCache>();
    let indirect_parameters_buffers = world.resource::<IndirectParametersBuffers>();

    let mut compute_pass =
        render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });
    let pass_span = diagnostics.time_span(&mut compute_pass, label);

    // Fetch the pipeline.
    let (
        Some(reset_indirect_batch_sets_pipeline_id),
        Some(build_indexed_indirect_params_pipeline_id),
        Some(build_non_indexed_indirect_params_pipeline_id),
    ) = (
        preprocess_phase_pipelines
            .reset_indirect_batch_sets
            .pipeline_id,
        preprocess_phase_pipelines
            .gpu_occlusion_culling_build_indexed_indirect_params
            .pipeline_id,
        preprocess_phase_pipelines
            .gpu_occlusion_culling_build_non_indexed_indirect_params
            .pipeline_id,
    )
    else {
        warn!("The build indirect parameters pipelines weren't ready");
        pass_span.end(&mut compute_pass);
        return Ok(());
    };

    let (
        Some(reset_indirect_batch_sets_pipeline),
        Some(build_indexed_indirect_params_pipeline),
        Some(build_non_indexed_indirect_params_pipeline),
    ) = (
        pipeline_cache.get_compute_pipeline(reset_indirect_batch_sets_pipeline_id),
        pipeline_cache.get_compute_pipeline(build_indexed_indirect_params_pipeline_id),
        pipeline_cache.get_compute_pipeline(build_non_indexed_indirect_params_pipeline_id),
    )
    else {
        // This will happen while the pipeline is being compiled and is fine.
        pass_span.end(&mut compute_pass);
        return Ok(());
    };

    // Loop over each phase. As each has as separate set of buffers, we need to
    // build indirect parameters individually for each phase.
    for (phase_type_id, phase_build_indirect_params_bind_groups) in
        build_indirect_params_bind_groups.iter()
    {
        let Some(phase_indirect_parameters_buffers) =
            indirect_parameters_buffers.get(phase_type_id)
        else {
            continue;
        };

        // Build indexed indirect parameters.
        if let (
            Some(reset_indexed_indirect_batch_sets_bind_group),
            Some(build_indirect_indexed_params_bind_group),
        ) = (
            &phase_build_indirect_params_bind_groups.reset_indexed_indirect_batch_sets,
            &phase_build_indirect_params_bind_groups.build_indexed_indirect,
        ) {
            compute_pass.set_pipeline(reset_indirect_batch_sets_pipeline);
            compute_pass.set_bind_group(0, reset_indexed_indirect_batch_sets_bind_group, &[]);
            let workgroup_count = phase_indirect_parameters_buffers
                .batch_set_count(true)
                .div_ceil(WORKGROUP_SIZE);
            if workgroup_count > 0 {
                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }

            compute_pass.set_pipeline(build_indexed_indirect_params_pipeline);
            compute_pass.set_bind_group(0, build_indirect_indexed_params_bind_group, &[]);
            let workgroup_count = phase_indirect_parameters_buffers
                .indexed
                .batch_count()
                .div_ceil(WORKGROUP_SIZE);
            if workgroup_count > 0 {
                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }
        }

        // Build non-indexed indirect parameters.
        if let (
            Some(reset_non_indexed_indirect_batch_sets_bind_group),
            Some(build_indirect_non_indexed_params_bind_group),
        ) = (
            &phase_build_indirect_params_bind_groups.reset_non_indexed_indirect_batch_sets,
            &phase_build_indirect_params_bind_groups.build_non_indexed_indirect,
        ) {
            compute_pass.set_pipeline(reset_indirect_batch_sets_pipeline);
            compute_pass.set_bind_group(0, reset_non_indexed_indirect_batch_sets_bind_group, &[]);
            let workgroup_count = phase_indirect_parameters_buffers
                .batch_set_count(false)
                .div_ceil(WORKGROUP_SIZE);
            if workgroup_count > 0 {
                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }

            compute_pass.set_pipeline(build_non_indexed_indirect_params_pipeline);
            compute_pass.set_bind_group(0, build_indirect_non_indexed_params_bind_group, &[]);
            let workgroup_count = phase_indirect_parameters_buffers
                .non_indexed
                .batch_count()
                .div_ceil(WORKGROUP_SIZE);
            if workgroup_count > 0 {
                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }
        }
    }

    pass_span.end(&mut compute_pass);

    Ok(())
}

impl PreprocessPipelines {
    /// Returns true if the preprocessing and indirect parameters pipelines have
    /// been loaded or false otherwise.
    pub(crate) fn pipelines_are_loaded(
        &self,
        pipeline_cache: &PipelineCache,
        preprocessing_support: &GpuPreprocessingSupport,
    ) -> bool {
        match preprocessing_support.max_supported_mode {
            GpuPreprocessingMode::None => false,
            GpuPreprocessingMode::PreprocessingOnly => {
                self.direct_preprocess.is_loaded(pipeline_cache)
                    && self
                        .gpu_frustum_culling_preprocess
                        .is_loaded(pipeline_cache)
            }
            GpuPreprocessingMode::Culling => {
                self.direct_preprocess.is_loaded(pipeline_cache)
                    && self
                        .gpu_frustum_culling_preprocess
                        .is_loaded(pipeline_cache)
                    && self
                        .early_gpu_occlusion_culling_preprocess
                        .is_loaded(pipeline_cache)
                    && self
                        .late_gpu_occlusion_culling_preprocess
                        .is_loaded(pipeline_cache)
                    && self
                        .gpu_frustum_culling_build_indexed_indirect_params
                        .is_loaded(pipeline_cache)
                    && self
                        .gpu_frustum_culling_build_non_indexed_indirect_params
                        .is_loaded(pipeline_cache)
                    && self.early_phase.is_loaded(pipeline_cache)
                    && self.late_phase.is_loaded(pipeline_cache)
                    && self.main_phase.is_loaded(pipeline_cache)
            }
        }
    }
}

impl PreprocessPhasePipelines {
    fn is_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        self.reset_indirect_batch_sets.is_loaded(pipeline_cache)
            && self
                .gpu_occlusion_culling_build_indexed_indirect_params
                .is_loaded(pipeline_cache)
            && self
                .gpu_occlusion_culling_build_non_indexed_indirect_params
                .is_loaded(pipeline_cache)
    }
}

impl PreprocessPipeline {
    fn is_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        self.pipeline_id
            .is_some_and(|pipeline_id| pipeline_cache.get_compute_pipeline(pipeline_id).is_some())
    }
}

impl ResetIndirectBatchSetsPipeline {
    fn is_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        self.pipeline_id
            .is_some_and(|pipeline_id| pipeline_cache.get_compute_pipeline(pipeline_id).is_some())
    }
}

impl BuildIndirectParametersPipeline {
    /// Returns true if this pipeline has been loaded into the pipeline cache or
    /// false otherwise.
    fn is_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        self.pipeline_id
            .is_some_and(|pipeline_id| pipeline_cache.get_compute_pipeline(pipeline_id).is_some())
    }
}

impl SpecializedComputePipeline for PreprocessPipeline {
    type Key = PreprocessPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut shader_defs = vec!["WRITE_INDIRECT_PARAMETERS_METADATA".into()];
        if key.contains(PreprocessPipelineKey::FRUSTUM_CULLING) {
            shader_defs.push("INDIRECT".into());
            shader_defs.push("FRUSTUM_CULLING".into());
        }
        if key.contains(PreprocessPipelineKey::OCCLUSION_CULLING) {
            shader_defs.push("OCCLUSION_CULLING".into());
            if key.contains(PreprocessPipelineKey::EARLY_PHASE) {
                shader_defs.push("EARLY_PHASE".into());
            } else {
                shader_defs.push("LATE_PHASE".into());
            }
        }

        ComputePipelineDescriptor {
            label: Some(
                format!(
                    "mesh preprocessing ({})",
                    if key.contains(
                        PreprocessPipelineKey::OCCLUSION_CULLING
                            | PreprocessPipelineKey::EARLY_PHASE
                    ) {
                        "early GPU occlusion culling"
                    } else if key.contains(PreprocessPipelineKey::OCCLUSION_CULLING) {
                        "late GPU occlusion culling"
                    } else if key.contains(PreprocessPipelineKey::FRUSTUM_CULLING) {
                        "GPU frustum culling"
                    } else {
                        "direct"
                    }
                )
                .into(),
            ),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: if key.contains(PreprocessPipelineKey::OCCLUSION_CULLING) {
                vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }]
            } else {
                vec![]
            },
            shader: self.shader.clone(),
            shader_defs,
            ..default()
        }
    }
}

impl FromWorld for PreprocessPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // GPU culling bind group parameters are a superset of those in the CPU
        // culling (direct) shader.
        let direct_bind_group_layout_entries = preprocess_direct_bind_group_layout_entries();
        let gpu_frustum_culling_bind_group_layout_entries = gpu_culling_bind_group_layout_entries();
        let gpu_early_occlusion_culling_bind_group_layout_entries =
            gpu_occlusion_culling_bind_group_layout_entries().extend_with_indices(((
                11,
                storage_buffer::<PreprocessWorkItem>(/*has_dynamic_offset=*/ false),
            ),));
        let gpu_late_occlusion_culling_bind_group_layout_entries =
            gpu_occlusion_culling_bind_group_layout_entries();

        let reset_indirect_batch_sets_bind_group_layout_entries =
            DynamicBindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (storage_buffer::<IndirectBatchSet>(false),),
            );

        // Indexed and non-indexed bind group parameters share all the bind
        // group layout entries except the final one.
        let build_indexed_indirect_params_bind_group_layout_entries =
            build_indirect_params_bind_group_layout_entries()
                .extend_sequential((storage_buffer::<IndirectParametersIndexed>(false),));
        let build_non_indexed_indirect_params_bind_group_layout_entries =
            build_indirect_params_bind_group_layout_entries()
                .extend_sequential((storage_buffer::<IndirectParametersNonIndexed>(false),));

        // Create the bind group layouts.
        let direct_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms direct bind group layout",
            &direct_bind_group_layout_entries,
        );
        let gpu_frustum_culling_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms GPU frustum culling bind group layout",
            &gpu_frustum_culling_bind_group_layout_entries,
        );
        let gpu_early_occlusion_culling_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms GPU early occlusion culling bind group layout",
            &gpu_early_occlusion_culling_bind_group_layout_entries,
        );
        let gpu_late_occlusion_culling_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms GPU late occlusion culling bind group layout",
            &gpu_late_occlusion_culling_bind_group_layout_entries,
        );
        let reset_indirect_batch_sets_bind_group_layout = render_device.create_bind_group_layout(
            "reset indirect batch sets bind group layout",
            &reset_indirect_batch_sets_bind_group_layout_entries,
        );
        let build_indexed_indirect_params_bind_group_layout = render_device
            .create_bind_group_layout(
                "build indexed indirect parameters bind group layout",
                &build_indexed_indirect_params_bind_group_layout_entries,
            );
        let build_non_indexed_indirect_params_bind_group_layout = render_device
            .create_bind_group_layout(
                "build non-indexed indirect parameters bind group layout",
                &build_non_indexed_indirect_params_bind_group_layout_entries,
            );

        let preprocess_shader = load_embedded_asset!(world, "mesh_preprocess.wgsl");
        let reset_indirect_batch_sets_shader =
            load_embedded_asset!(world, "reset_indirect_batch_sets.wgsl");
        let build_indirect_params_shader =
            load_embedded_asset!(world, "build_indirect_params.wgsl");

        let preprocess_phase_pipelines = PreprocessPhasePipelines {
            reset_indirect_batch_sets: ResetIndirectBatchSetsPipeline {
                bind_group_layout: reset_indirect_batch_sets_bind_group_layout.clone(),
                shader: reset_indirect_batch_sets_shader,
                pipeline_id: None,
            },
            gpu_occlusion_culling_build_indexed_indirect_params: BuildIndirectParametersPipeline {
                bind_group_layout: build_indexed_indirect_params_bind_group_layout.clone(),
                shader: build_indirect_params_shader.clone(),
                pipeline_id: None,
            },
            gpu_occlusion_culling_build_non_indexed_indirect_params:
                BuildIndirectParametersPipeline {
                    bind_group_layout: build_non_indexed_indirect_params_bind_group_layout.clone(),
                    shader: build_indirect_params_shader.clone(),
                    pipeline_id: None,
                },
        };

        PreprocessPipelines {
            direct_preprocess: PreprocessPipeline {
                bind_group_layout: direct_bind_group_layout,
                shader: preprocess_shader.clone(),
                pipeline_id: None,
            },
            gpu_frustum_culling_preprocess: PreprocessPipeline {
                bind_group_layout: gpu_frustum_culling_bind_group_layout,
                shader: preprocess_shader.clone(),
                pipeline_id: None,
            },
            early_gpu_occlusion_culling_preprocess: PreprocessPipeline {
                bind_group_layout: gpu_early_occlusion_culling_bind_group_layout,
                shader: preprocess_shader.clone(),
                pipeline_id: None,
            },
            late_gpu_occlusion_culling_preprocess: PreprocessPipeline {
                bind_group_layout: gpu_late_occlusion_culling_bind_group_layout,
                shader: preprocess_shader,
                pipeline_id: None,
            },
            gpu_frustum_culling_build_indexed_indirect_params: BuildIndirectParametersPipeline {
                bind_group_layout: build_indexed_indirect_params_bind_group_layout.clone(),
                shader: build_indirect_params_shader.clone(),
                pipeline_id: None,
            },
            gpu_frustum_culling_build_non_indexed_indirect_params:
                BuildIndirectParametersPipeline {
                    bind_group_layout: build_non_indexed_indirect_params_bind_group_layout.clone(),
                    shader: build_indirect_params_shader,
                    pipeline_id: None,
                },
            early_phase: preprocess_phase_pipelines.clone(),
            late_phase: preprocess_phase_pipelines.clone(),
            main_phase: preprocess_phase_pipelines.clone(),
        }
    }
}

fn preprocess_direct_bind_group_layout_entries() -> DynamicBindGroupLayoutEntries {
    DynamicBindGroupLayoutEntries::new_with_indices(
        ShaderStages::COMPUTE,
        (
            // `view`
            (
                0,
                uniform_buffer::<ViewUniform>(/* has_dynamic_offset= */ true),
            ),
            // `current_input`
            (3, storage_buffer_read_only::<MeshInputUniform>(false)),
            // `previous_input`
            (4, storage_buffer_read_only::<MeshInputUniform>(false)),
            // `indices`
            (5, storage_buffer_read_only::<PreprocessWorkItem>(false)),
            // `output`
            (6, storage_buffer::<MeshUniform>(false)),
        ),
    )
}

// Returns the first 4 bind group layout entries shared between all invocations
// of the indirect parameters building shader.
fn build_indirect_params_bind_group_layout_entries() -> DynamicBindGroupLayoutEntries {
    DynamicBindGroupLayoutEntries::new_with_indices(
        ShaderStages::COMPUTE,
        (
            (0, storage_buffer_read_only::<MeshInputUniform>(false)),
            (
                1,
                storage_buffer_read_only::<IndirectParametersCpuMetadata>(false),
            ),
            (
                2,
                storage_buffer_read_only::<IndirectParametersGpuMetadata>(false),
            ),
            (3, storage_buffer::<IndirectBatchSet>(false)),
        ),
    )
}

/// A system that specializes the `mesh_preprocess.wgsl` and
/// `build_indirect_params.wgsl` pipelines if necessary.
fn gpu_culling_bind_group_layout_entries() -> DynamicBindGroupLayoutEntries {
    // GPU culling bind group parameters are a superset of those in the CPU
    // culling (direct) shader.
    preprocess_direct_bind_group_layout_entries().extend_with_indices((
        // `indirect_parameters_cpu_metadata`
        (
            7,
            storage_buffer_read_only::<IndirectParametersCpuMetadata>(
                /* has_dynamic_offset= */ false,
            ),
        ),
        // `indirect_parameters_gpu_metadata`
        (
            8,
            storage_buffer::<IndirectParametersGpuMetadata>(/* has_dynamic_offset= */ false),
        ),
        // `mesh_culling_data`
        (
            9,
            storage_buffer_read_only::<MeshCullingData>(/* has_dynamic_offset= */ false),
        ),
    ))
}

fn gpu_occlusion_culling_bind_group_layout_entries() -> DynamicBindGroupLayoutEntries {
    gpu_culling_bind_group_layout_entries().extend_with_indices((
        (
            2,
            uniform_buffer::<PreviousViewData>(/*has_dynamic_offset=*/ false),
        ),
        (
            10,
            texture_2d(TextureSampleType::Float { filterable: true }),
        ),
        (
            12,
            storage_buffer::<LatePreprocessWorkItemIndirectParameters>(
                /*has_dynamic_offset=*/ false,
            ),
        ),
    ))
}

/// A system that specializes the `mesh_preprocess.wgsl` pipelines if necessary.
pub fn prepare_preprocess_pipelines(
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    mut specialized_preprocess_pipelines: ResMut<SpecializedComputePipelines<PreprocessPipeline>>,
    mut specialized_reset_indirect_batch_sets_pipelines: ResMut<
        SpecializedComputePipelines<ResetIndirectBatchSetsPipeline>,
    >,
    mut specialized_build_indirect_parameters_pipelines: ResMut<
        SpecializedComputePipelines<BuildIndirectParametersPipeline>,
    >,
    preprocess_pipelines: ResMut<PreprocessPipelines>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
) {
    let preprocess_pipelines = preprocess_pipelines.into_inner();

    preprocess_pipelines.direct_preprocess.prepare(
        &pipeline_cache,
        &mut specialized_preprocess_pipelines,
        PreprocessPipelineKey::empty(),
    );
    preprocess_pipelines.gpu_frustum_culling_preprocess.prepare(
        &pipeline_cache,
        &mut specialized_preprocess_pipelines,
        PreprocessPipelineKey::FRUSTUM_CULLING,
    );

    if gpu_preprocessing_support.is_culling_supported() {
        preprocess_pipelines
            .early_gpu_occlusion_culling_preprocess
            .prepare(
                &pipeline_cache,
                &mut specialized_preprocess_pipelines,
                PreprocessPipelineKey::FRUSTUM_CULLING
                    | PreprocessPipelineKey::OCCLUSION_CULLING
                    | PreprocessPipelineKey::EARLY_PHASE,
            );
        preprocess_pipelines
            .late_gpu_occlusion_culling_preprocess
            .prepare(
                &pipeline_cache,
                &mut specialized_preprocess_pipelines,
                PreprocessPipelineKey::FRUSTUM_CULLING | PreprocessPipelineKey::OCCLUSION_CULLING,
            );
    }

    let mut build_indirect_parameters_pipeline_key = BuildIndirectParametersPipelineKey::empty();

    // If the GPU and driver support `multi_draw_indirect_count`, tell the
    // shader that.
    if render_device
        .wgpu_device()
        .features()
        .contains(WgpuFeatures::MULTI_DRAW_INDIRECT_COUNT)
    {
        build_indirect_parameters_pipeline_key
            .insert(BuildIndirectParametersPipelineKey::MULTI_DRAW_INDIRECT_COUNT_SUPPORTED);
    }

    preprocess_pipelines
        .gpu_frustum_culling_build_indexed_indirect_params
        .prepare(
            &pipeline_cache,
            &mut specialized_build_indirect_parameters_pipelines,
            build_indirect_parameters_pipeline_key | BuildIndirectParametersPipelineKey::INDEXED,
        );
    preprocess_pipelines
        .gpu_frustum_culling_build_non_indexed_indirect_params
        .prepare(
            &pipeline_cache,
            &mut specialized_build_indirect_parameters_pipelines,
            build_indirect_parameters_pipeline_key,
        );

    if !gpu_preprocessing_support.is_culling_supported() {
        return;
    }

    for (preprocess_phase_pipelines, build_indirect_parameters_phase_pipeline_key) in [
        (
            &mut preprocess_pipelines.early_phase,
            BuildIndirectParametersPipelineKey::EARLY_PHASE,
        ),
        (
            &mut preprocess_pipelines.late_phase,
            BuildIndirectParametersPipelineKey::LATE_PHASE,
        ),
        (
            &mut preprocess_pipelines.main_phase,
            BuildIndirectParametersPipelineKey::MAIN_PHASE,
        ),
    ] {
        preprocess_phase_pipelines
            .reset_indirect_batch_sets
            .prepare(
                &pipeline_cache,
                &mut specialized_reset_indirect_batch_sets_pipelines,
            );
        preprocess_phase_pipelines
            .gpu_occlusion_culling_build_indexed_indirect_params
            .prepare(
                &pipeline_cache,
                &mut specialized_build_indirect_parameters_pipelines,
                build_indirect_parameters_pipeline_key
                    | build_indirect_parameters_phase_pipeline_key
                    | BuildIndirectParametersPipelineKey::INDEXED
                    | BuildIndirectParametersPipelineKey::OCCLUSION_CULLING,
            );
        preprocess_phase_pipelines
            .gpu_occlusion_culling_build_non_indexed_indirect_params
            .prepare(
                &pipeline_cache,
                &mut specialized_build_indirect_parameters_pipelines,
                build_indirect_parameters_pipeline_key
                    | build_indirect_parameters_phase_pipeline_key
                    | BuildIndirectParametersPipelineKey::OCCLUSION_CULLING,
            );
    }
}

impl PreprocessPipeline {
    fn prepare(
        &mut self,
        pipeline_cache: &PipelineCache,
        pipelines: &mut SpecializedComputePipelines<PreprocessPipeline>,
        key: PreprocessPipelineKey,
    ) {
        if self.pipeline_id.is_some() {
            return;
        }

        let preprocess_pipeline_id = pipelines.specialize(pipeline_cache, self, key);
        self.pipeline_id = Some(preprocess_pipeline_id);
    }
}

impl SpecializedComputePipeline for ResetIndirectBatchSetsPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("reset indirect batch sets".into()),
            layout: vec![self.bind_group_layout.clone()],
            shader: self.shader.clone(),
            ..default()
        }
    }
}

impl SpecializedComputePipeline for BuildIndirectParametersPipeline {
    type Key = BuildIndirectParametersPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut shader_defs = vec![];
        if key.contains(BuildIndirectParametersPipelineKey::INDEXED) {
            shader_defs.push("INDEXED".into());
        }
        if key.contains(BuildIndirectParametersPipelineKey::MULTI_DRAW_INDIRECT_COUNT_SUPPORTED) {
            shader_defs.push("MULTI_DRAW_INDIRECT_COUNT_SUPPORTED".into());
        }
        if key.contains(BuildIndirectParametersPipelineKey::OCCLUSION_CULLING) {
            shader_defs.push("OCCLUSION_CULLING".into());
        }
        if key.contains(BuildIndirectParametersPipelineKey::EARLY_PHASE) {
            shader_defs.push("EARLY_PHASE".into());
        }
        if key.contains(BuildIndirectParametersPipelineKey::LATE_PHASE) {
            shader_defs.push("LATE_PHASE".into());
        }
        if key.contains(BuildIndirectParametersPipelineKey::MAIN_PHASE) {
            shader_defs.push("MAIN_PHASE".into());
        }

        let label = format!(
            "{} build {}indexed indirect parameters",
            if !key.contains(BuildIndirectParametersPipelineKey::OCCLUSION_CULLING) {
                "frustum culling"
            } else if key.contains(BuildIndirectParametersPipelineKey::EARLY_PHASE) {
                "early occlusion culling"
            } else if key.contains(BuildIndirectParametersPipelineKey::LATE_PHASE) {
                "late occlusion culling"
            } else {
                "main occlusion culling"
            },
            if key.contains(BuildIndirectParametersPipelineKey::INDEXED) {
                ""
            } else {
                "non-"
            }
        );

        ComputePipelineDescriptor {
            label: Some(label.into()),
            layout: vec![self.bind_group_layout.clone()],
            shader: self.shader.clone(),
            shader_defs,
            ..default()
        }
    }
}

impl ResetIndirectBatchSetsPipeline {
    fn prepare(
        &mut self,
        pipeline_cache: &PipelineCache,
        pipelines: &mut SpecializedComputePipelines<ResetIndirectBatchSetsPipeline>,
    ) {
        if self.pipeline_id.is_some() {
            return;
        }

        let reset_indirect_batch_sets_pipeline_id = pipelines.specialize(pipeline_cache, self, ());
        self.pipeline_id = Some(reset_indirect_batch_sets_pipeline_id);
    }
}

impl BuildIndirectParametersPipeline {
    fn prepare(
        &mut self,
        pipeline_cache: &PipelineCache,
        pipelines: &mut SpecializedComputePipelines<BuildIndirectParametersPipeline>,
        key: BuildIndirectParametersPipelineKey,
    ) {
        if self.pipeline_id.is_some() {
            return;
        }

        let build_indirect_parameters_pipeline_id = pipelines.specialize(pipeline_cache, self, key);
        self.pipeline_id = Some(build_indirect_parameters_pipeline_id);
    }
}

/// A system that attaches the mesh uniform buffers to the bind groups for the
/// variants of the mesh preprocessing compute shader.
#[expect(
    clippy::too_many_arguments,
    reason = "it's a system that needs a lot of arguments"
)]
pub fn prepare_preprocess_bind_groups(
    mut commands: Commands,
    views: Query<(Entity, &ExtractedView)>,
    view_depth_pyramids: Query<(&ViewDepthPyramid, &PreviousViewUniformOffset)>,
    render_device: Res<RenderDevice>,
    batched_instance_buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    indirect_parameters_buffers: Res<IndirectParametersBuffers>,
    mesh_culling_data_buffer: Res<MeshCullingDataBuffer>,
    view_uniforms: Res<ViewUniforms>,
    previous_view_uniforms: Res<PreviousViewUniforms>,
    pipelines: Res<PreprocessPipelines>,
) {
    // Grab the `BatchedInstanceBuffers`.
    let BatchedInstanceBuffers {
        current_input_buffer: current_input_buffer_vec,
        previous_input_buffer: previous_input_buffer_vec,
        phase_instance_buffers,
    } = batched_instance_buffers.into_inner();

    let (Some(current_input_buffer), Some(previous_input_buffer)) = (
        current_input_buffer_vec.buffer().buffer(),
        previous_input_buffer_vec.buffer().buffer(),
    ) else {
        return;
    };

    // Record whether we have any meshes that are to be drawn indirectly. If we
    // don't, then we can skip building indirect parameters.
    let mut any_indirect = false;

    // Loop over each view.
    for (view_entity, view) in &views {
        let mut bind_groups = TypeIdMap::default();

        // Loop over each phase.
        for (phase_type_id, phase_instance_buffers) in phase_instance_buffers {
            let UntypedPhaseBatchedInstanceBuffers {
                data_buffer: ref data_buffer_vec,
                ref work_item_buffers,
                ref late_indexed_indirect_parameters_buffer,
                ref late_non_indexed_indirect_parameters_buffer,
            } = *phase_instance_buffers;

            let Some(data_buffer) = data_buffer_vec.buffer() else {
                continue;
            };

            // Grab the indirect parameters buffers for this phase.
            let Some(phase_indirect_parameters_buffers) =
                indirect_parameters_buffers.get(phase_type_id)
            else {
                continue;
            };

            let Some(work_item_buffers) = work_item_buffers.get(&view.retained_view_entity) else {
                continue;
            };

            // Create the `PreprocessBindGroupBuilder`.
            let preprocess_bind_group_builder = PreprocessBindGroupBuilder {
                view: view_entity,
                late_indexed_indirect_parameters_buffer,
                late_non_indexed_indirect_parameters_buffer,
                render_device: &render_device,
                phase_indirect_parameters_buffers,
                mesh_culling_data_buffer: &mesh_culling_data_buffer,
                view_uniforms: &view_uniforms,
                previous_view_uniforms: &previous_view_uniforms,
                pipelines: &pipelines,
                current_input_buffer,
                previous_input_buffer,
                data_buffer,
            };

            // Depending on the type of work items we have, construct the
            // appropriate bind groups.
            let (was_indirect, bind_group) = match *work_item_buffers {
                PreprocessWorkItemBuffers::Direct(ref work_item_buffer) => (
                    false,
                    preprocess_bind_group_builder
                        .create_direct_preprocess_bind_groups(work_item_buffer),
                ),

                PreprocessWorkItemBuffers::Indirect {
                    indexed: ref indexed_work_item_buffer,
                    non_indexed: ref non_indexed_work_item_buffer,
                    gpu_occlusion_culling: Some(ref gpu_occlusion_culling_work_item_buffers),
                } => (
                    true,
                    preprocess_bind_group_builder
                        .create_indirect_occlusion_culling_preprocess_bind_groups(
                            &view_depth_pyramids,
                            indexed_work_item_buffer,
                            non_indexed_work_item_buffer,
                            gpu_occlusion_culling_work_item_buffers,
                        ),
                ),

                PreprocessWorkItemBuffers::Indirect {
                    indexed: ref indexed_work_item_buffer,
                    non_indexed: ref non_indexed_work_item_buffer,
                    gpu_occlusion_culling: None,
                } => (
                    true,
                    preprocess_bind_group_builder
                        .create_indirect_frustum_culling_preprocess_bind_groups(
                            indexed_work_item_buffer,
                            non_indexed_work_item_buffer,
                        ),
                ),
            };

            // Write that bind group in.
            if let Some(bind_group) = bind_group {
                any_indirect = any_indirect || was_indirect;
                bind_groups.insert(*phase_type_id, bind_group);
            }
        }

        // Save the bind groups.
        commands
            .entity(view_entity)
            .insert(PreprocessBindGroups(bind_groups));
    }

    // Now, if there were any indirect draw commands, create the bind groups for
    // the indirect parameters building shader.
    if any_indirect {
        create_build_indirect_parameters_bind_groups(
            &mut commands,
            &render_device,
            &pipelines,
            current_input_buffer,
            &indirect_parameters_buffers,
        );
    }
}

/// A temporary structure that stores all the information needed to construct
/// bind groups for the mesh preprocessing shader.
struct PreprocessBindGroupBuilder<'a> {
    /// The render-world entity corresponding to the current view.
    view: Entity,
    /// The indirect compute dispatch parameters buffer for indexed meshes in
    /// the late prepass.
    late_indexed_indirect_parameters_buffer:
        &'a RawBufferVec<LatePreprocessWorkItemIndirectParameters>,
    /// The indirect compute dispatch parameters buffer for non-indexed meshes
    /// in the late prepass.
    late_non_indexed_indirect_parameters_buffer:
        &'a RawBufferVec<LatePreprocessWorkItemIndirectParameters>,
    /// The device.
    render_device: &'a RenderDevice,
    /// The buffers that store indirect draw parameters.
    phase_indirect_parameters_buffers: &'a UntypedPhaseIndirectParametersBuffers,
    /// The GPU buffer that stores the information needed to cull each mesh.
    mesh_culling_data_buffer: &'a MeshCullingDataBuffer,
    /// The GPU buffer that stores information about the view.
    view_uniforms: &'a ViewUniforms,
    /// The GPU buffer that stores information about the view from last frame.
    previous_view_uniforms: &'a PreviousViewUniforms,
    /// The pipelines for the mesh preprocessing shader.
    pipelines: &'a PreprocessPipelines,
    /// The GPU buffer containing the list of [`MeshInputUniform`]s for the
    /// current frame.
    current_input_buffer: &'a Buffer,
    /// The GPU buffer containing the list of [`MeshInputUniform`]s for the
    /// previous frame.
    previous_input_buffer: &'a Buffer,
    /// The GPU buffer containing the list of [`MeshUniform`]s for the current
    /// frame.
    ///
    /// This is the buffer containing the mesh's final transforms that the
    /// shaders will write to.
    data_buffer: &'a Buffer,
}

impl<'a> PreprocessBindGroupBuilder<'a> {
    /// Creates the bind groups for mesh preprocessing when GPU frustum culling
    /// and GPU occlusion culling are both disabled.
    fn create_direct_preprocess_bind_groups(
        &self,
        work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
    ) -> Option<PhasePreprocessBindGroups> {
        // Don't use `as_entire_binding()` here; the shader reads the array
        // length and the underlying buffer may be longer than the actual size
        // of the vector.
        let work_item_buffer_size = NonZero::<u64>::try_from(
            work_item_buffer.len() as u64 * u64::from(PreprocessWorkItem::min_size()),
        )
        .ok();

        Some(PhasePreprocessBindGroups::Direct(
            self.render_device.create_bind_group(
                "preprocess_direct_bind_group",
                &self.pipelines.direct_preprocess.bind_group_layout,
                &BindGroupEntries::with_indices((
                    (0, self.view_uniforms.uniforms.binding()?),
                    (3, self.current_input_buffer.as_entire_binding()),
                    (4, self.previous_input_buffer.as_entire_binding()),
                    (
                        5,
                        BindingResource::Buffer(BufferBinding {
                            buffer: work_item_buffer.buffer()?,
                            offset: 0,
                            size: work_item_buffer_size,
                        }),
                    ),
                    (6, self.data_buffer.as_entire_binding()),
                )),
            ),
        ))
    }

    /// Creates the bind groups for mesh preprocessing when GPU occlusion
    /// culling is enabled.
    fn create_indirect_occlusion_culling_preprocess_bind_groups(
        &self,
        view_depth_pyramids: &Query<(&ViewDepthPyramid, &PreviousViewUniformOffset)>,
        indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
        non_indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
        gpu_occlusion_culling_work_item_buffers: &GpuOcclusionCullingWorkItemBuffers,
    ) -> Option<PhasePreprocessBindGroups> {
        let GpuOcclusionCullingWorkItemBuffers {
            late_indexed: ref late_indexed_work_item_buffer,
            late_non_indexed: ref late_non_indexed_work_item_buffer,
            ..
        } = *gpu_occlusion_culling_work_item_buffers;

        let (view_depth_pyramid, previous_view_uniform_offset) =
            view_depth_pyramids.get(self.view).ok()?;

        Some(PhasePreprocessBindGroups::IndirectOcclusionCulling {
            early_indexed: self.create_indirect_occlusion_culling_early_indexed_bind_group(
                view_depth_pyramid,
                previous_view_uniform_offset,
                indexed_work_item_buffer,
                late_indexed_work_item_buffer,
            ),

            early_non_indexed: self.create_indirect_occlusion_culling_early_non_indexed_bind_group(
                view_depth_pyramid,
                previous_view_uniform_offset,
                non_indexed_work_item_buffer,
                late_non_indexed_work_item_buffer,
            ),

            late_indexed: self.create_indirect_occlusion_culling_late_indexed_bind_group(
                view_depth_pyramid,
                previous_view_uniform_offset,
                late_indexed_work_item_buffer,
            ),

            late_non_indexed: self.create_indirect_occlusion_culling_late_non_indexed_bind_group(
                view_depth_pyramid,
                previous_view_uniform_offset,
                late_non_indexed_work_item_buffer,
            ),
        })
    }

    /// Creates the bind group for the first phase of mesh preprocessing of
    /// indexed meshes when GPU occlusion culling is enabled.
    fn create_indirect_occlusion_culling_early_indexed_bind_group(
        &self,
        view_depth_pyramid: &ViewDepthPyramid,
        previous_view_uniform_offset: &PreviousViewUniformOffset,
        indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
        late_indexed_work_item_buffer: &UninitBufferVec<PreprocessWorkItem>,
    ) -> Option<BindGroup> {
        let mesh_culling_data_buffer = self.mesh_culling_data_buffer.buffer()?;
        let view_uniforms_binding = self.view_uniforms.uniforms.binding()?;
        let previous_view_buffer = self.previous_view_uniforms.uniforms.buffer()?;

        match (
            self.phase_indirect_parameters_buffers
                .indexed
                .cpu_metadata_buffer(),
            self.phase_indirect_parameters_buffers
                .indexed
                .gpu_metadata_buffer(),
            indexed_work_item_buffer.buffer(),
            late_indexed_work_item_buffer.buffer(),
            self.late_indexed_indirect_parameters_buffer.buffer(),
        ) {
            (
                Some(indexed_cpu_metadata_buffer),
                Some(indexed_gpu_metadata_buffer),
                Some(indexed_work_item_gpu_buffer),
                Some(late_indexed_work_item_gpu_buffer),
                Some(late_indexed_indirect_parameters_buffer),
            ) => {
                // Don't use `as_entire_binding()` here; the shader reads the array
                // length and the underlying buffer may be longer than the actual size
                // of the vector.
                let indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                    indexed_work_item_buffer.len() as u64
                        * u64::from(PreprocessWorkItem::min_size()),
                )
                .ok();

                Some(
                    self.render_device.create_bind_group(
                        "preprocess_early_indexed_gpu_occlusion_culling_bind_group",
                        &self
                            .pipelines
                            .early_gpu_occlusion_culling_preprocess
                            .bind_group_layout,
                        &BindGroupEntries::with_indices((
                            (3, self.current_input_buffer.as_entire_binding()),
                            (4, self.previous_input_buffer.as_entire_binding()),
                            (
                                5,
                                BindingResource::Buffer(BufferBinding {
                                    buffer: indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: indexed_work_item_buffer_size,
                                }),
                            ),
                            (6, self.data_buffer.as_entire_binding()),
                            (7, indexed_cpu_metadata_buffer.as_entire_binding()),
                            (8, indexed_gpu_metadata_buffer.as_entire_binding()),
                            (9, mesh_culling_data_buffer.as_entire_binding()),
                            (0, view_uniforms_binding.clone()),
                            (10, &view_depth_pyramid.all_mips),
                            (
                                2,
                                BufferBinding {
                                    buffer: previous_view_buffer,
                                    offset: previous_view_uniform_offset.offset as u64,
                                    size: NonZeroU64::new(size_of::<PreviousViewData>() as u64),
                                },
                            ),
                            (
                                11,
                                BufferBinding {
                                    buffer: late_indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: indexed_work_item_buffer_size,
                                },
                            ),
                            (
                                12,
                                BufferBinding {
                                    buffer: late_indexed_indirect_parameters_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        late_indexed_indirect_parameters_buffer.size(),
                                    ),
                                },
                            ),
                        )),
                    ),
                )
            }
            _ => None,
        }
    }

    /// Creates the bind group for the first phase of mesh preprocessing of
    /// non-indexed meshes when GPU occlusion culling is enabled.
    fn create_indirect_occlusion_culling_early_non_indexed_bind_group(
        &self,
        view_depth_pyramid: &ViewDepthPyramid,
        previous_view_uniform_offset: &PreviousViewUniformOffset,
        non_indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
        late_non_indexed_work_item_buffer: &UninitBufferVec<PreprocessWorkItem>,
    ) -> Option<BindGroup> {
        let mesh_culling_data_buffer = self.mesh_culling_data_buffer.buffer()?;
        let view_uniforms_binding = self.view_uniforms.uniforms.binding()?;
        let previous_view_buffer = self.previous_view_uniforms.uniforms.buffer()?;

        match (
            self.phase_indirect_parameters_buffers
                .non_indexed
                .cpu_metadata_buffer(),
            self.phase_indirect_parameters_buffers
                .non_indexed
                .gpu_metadata_buffer(),
            non_indexed_work_item_buffer.buffer(),
            late_non_indexed_work_item_buffer.buffer(),
            self.late_non_indexed_indirect_parameters_buffer.buffer(),
        ) {
            (
                Some(non_indexed_cpu_metadata_buffer),
                Some(non_indexed_gpu_metadata_buffer),
                Some(non_indexed_work_item_gpu_buffer),
                Some(late_non_indexed_work_item_buffer),
                Some(late_non_indexed_indirect_parameters_buffer),
            ) => {
                // Don't use `as_entire_binding()` here; the shader reads the array
                // length and the underlying buffer may be longer than the actual size
                // of the vector.
                let non_indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                    non_indexed_work_item_buffer.len() as u64
                        * u64::from(PreprocessWorkItem::min_size()),
                )
                .ok();

                Some(
                    self.render_device.create_bind_group(
                        "preprocess_early_non_indexed_gpu_occlusion_culling_bind_group",
                        &self
                            .pipelines
                            .early_gpu_occlusion_culling_preprocess
                            .bind_group_layout,
                        &BindGroupEntries::with_indices((
                            (3, self.current_input_buffer.as_entire_binding()),
                            (4, self.previous_input_buffer.as_entire_binding()),
                            (
                                5,
                                BindingResource::Buffer(BufferBinding {
                                    buffer: non_indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: non_indexed_work_item_buffer_size,
                                }),
                            ),
                            (6, self.data_buffer.as_entire_binding()),
                            (7, non_indexed_cpu_metadata_buffer.as_entire_binding()),
                            (8, non_indexed_gpu_metadata_buffer.as_entire_binding()),
                            (9, mesh_culling_data_buffer.as_entire_binding()),
                            (0, view_uniforms_binding.clone()),
                            (10, &view_depth_pyramid.all_mips),
                            (
                                2,
                                BufferBinding {
                                    buffer: previous_view_buffer,
                                    offset: previous_view_uniform_offset.offset as u64,
                                    size: NonZeroU64::new(size_of::<PreviousViewData>() as u64),
                                },
                            ),
                            (
                                11,
                                BufferBinding {
                                    buffer: late_non_indexed_work_item_buffer,
                                    offset: 0,
                                    size: non_indexed_work_item_buffer_size,
                                },
                            ),
                            (
                                12,
                                BufferBinding {
                                    buffer: late_non_indexed_indirect_parameters_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        late_non_indexed_indirect_parameters_buffer.size(),
                                    ),
                                },
                            ),
                        )),
                    ),
                )
            }
            _ => None,
        }
    }

    /// Creates the bind group for the second phase of mesh preprocessing of
    /// indexed meshes when GPU occlusion culling is enabled.
    fn create_indirect_occlusion_culling_late_indexed_bind_group(
        &self,
        view_depth_pyramid: &ViewDepthPyramid,
        previous_view_uniform_offset: &PreviousViewUniformOffset,
        late_indexed_work_item_buffer: &UninitBufferVec<PreprocessWorkItem>,
    ) -> Option<BindGroup> {
        let mesh_culling_data_buffer = self.mesh_culling_data_buffer.buffer()?;
        let view_uniforms_binding = self.view_uniforms.uniforms.binding()?;
        let previous_view_buffer = self.previous_view_uniforms.uniforms.buffer()?;

        match (
            self.phase_indirect_parameters_buffers
                .indexed
                .cpu_metadata_buffer(),
            self.phase_indirect_parameters_buffers
                .indexed
                .gpu_metadata_buffer(),
            late_indexed_work_item_buffer.buffer(),
            self.late_indexed_indirect_parameters_buffer.buffer(),
        ) {
            (
                Some(indexed_cpu_metadata_buffer),
                Some(indexed_gpu_metadata_buffer),
                Some(late_indexed_work_item_gpu_buffer),
                Some(late_indexed_indirect_parameters_buffer),
            ) => {
                // Don't use `as_entire_binding()` here; the shader reads the array
                // length and the underlying buffer may be longer than the actual size
                // of the vector.
                let late_indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                    late_indexed_work_item_buffer.len() as u64
                        * u64::from(PreprocessWorkItem::min_size()),
                )
                .ok();

                Some(
                    self.render_device.create_bind_group(
                        "preprocess_late_indexed_gpu_occlusion_culling_bind_group",
                        &self
                            .pipelines
                            .late_gpu_occlusion_culling_preprocess
                            .bind_group_layout,
                        &BindGroupEntries::with_indices((
                            (3, self.current_input_buffer.as_entire_binding()),
                            (4, self.previous_input_buffer.as_entire_binding()),
                            (
                                5,
                                BindingResource::Buffer(BufferBinding {
                                    buffer: late_indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: late_indexed_work_item_buffer_size,
                                }),
                            ),
                            (6, self.data_buffer.as_entire_binding()),
                            (7, indexed_cpu_metadata_buffer.as_entire_binding()),
                            (8, indexed_gpu_metadata_buffer.as_entire_binding()),
                            (9, mesh_culling_data_buffer.as_entire_binding()),
                            (0, view_uniforms_binding.clone()),
                            (10, &view_depth_pyramid.all_mips),
                            (
                                2,
                                BufferBinding {
                                    buffer: previous_view_buffer,
                                    offset: previous_view_uniform_offset.offset as u64,
                                    size: NonZeroU64::new(size_of::<PreviousViewData>() as u64),
                                },
                            ),
                            (
                                12,
                                BufferBinding {
                                    buffer: late_indexed_indirect_parameters_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        late_indexed_indirect_parameters_buffer.size(),
                                    ),
                                },
                            ),
                        )),
                    ),
                )
            }
            _ => None,
        }
    }

    /// Creates the bind group for the second phase of mesh preprocessing of
    /// non-indexed meshes when GPU occlusion culling is enabled.
    fn create_indirect_occlusion_culling_late_non_indexed_bind_group(
        &self,
        view_depth_pyramid: &ViewDepthPyramid,
        previous_view_uniform_offset: &PreviousViewUniformOffset,
        late_non_indexed_work_item_buffer: &UninitBufferVec<PreprocessWorkItem>,
    ) -> Option<BindGroup> {
        let mesh_culling_data_buffer = self.mesh_culling_data_buffer.buffer()?;
        let view_uniforms_binding = self.view_uniforms.uniforms.binding()?;
        let previous_view_buffer = self.previous_view_uniforms.uniforms.buffer()?;

        match (
            self.phase_indirect_parameters_buffers
                .non_indexed
                .cpu_metadata_buffer(),
            self.phase_indirect_parameters_buffers
                .non_indexed
                .gpu_metadata_buffer(),
            late_non_indexed_work_item_buffer.buffer(),
            self.late_non_indexed_indirect_parameters_buffer.buffer(),
        ) {
            (
                Some(non_indexed_cpu_metadata_buffer),
                Some(non_indexed_gpu_metadata_buffer),
                Some(non_indexed_work_item_gpu_buffer),
                Some(late_non_indexed_indirect_parameters_buffer),
            ) => {
                // Don't use `as_entire_binding()` here; the shader reads the array
                // length and the underlying buffer may be longer than the actual size
                // of the vector.
                let non_indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                    late_non_indexed_work_item_buffer.len() as u64
                        * u64::from(PreprocessWorkItem::min_size()),
                )
                .ok();

                Some(
                    self.render_device.create_bind_group(
                        "preprocess_late_non_indexed_gpu_occlusion_culling_bind_group",
                        &self
                            .pipelines
                            .late_gpu_occlusion_culling_preprocess
                            .bind_group_layout,
                        &BindGroupEntries::with_indices((
                            (3, self.current_input_buffer.as_entire_binding()),
                            (4, self.previous_input_buffer.as_entire_binding()),
                            (
                                5,
                                BindingResource::Buffer(BufferBinding {
                                    buffer: non_indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: non_indexed_work_item_buffer_size,
                                }),
                            ),
                            (6, self.data_buffer.as_entire_binding()),
                            (7, non_indexed_cpu_metadata_buffer.as_entire_binding()),
                            (8, non_indexed_gpu_metadata_buffer.as_entire_binding()),
                            (9, mesh_culling_data_buffer.as_entire_binding()),
                            (0, view_uniforms_binding.clone()),
                            (10, &view_depth_pyramid.all_mips),
                            (
                                2,
                                BufferBinding {
                                    buffer: previous_view_buffer,
                                    offset: previous_view_uniform_offset.offset as u64,
                                    size: NonZeroU64::new(size_of::<PreviousViewData>() as u64),
                                },
                            ),
                            (
                                12,
                                BufferBinding {
                                    buffer: late_non_indexed_indirect_parameters_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        late_non_indexed_indirect_parameters_buffer.size(),
                                    ),
                                },
                            ),
                        )),
                    ),
                )
            }
            _ => None,
        }
    }

    /// Creates the bind groups for mesh preprocessing when GPU frustum culling
    /// is enabled, but GPU occlusion culling is disabled.
    fn create_indirect_frustum_culling_preprocess_bind_groups(
        &self,
        indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
        non_indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
    ) -> Option<PhasePreprocessBindGroups> {
        Some(PhasePreprocessBindGroups::IndirectFrustumCulling {
            indexed: self
                .create_indirect_frustum_culling_indexed_bind_group(indexed_work_item_buffer),
            non_indexed: self.create_indirect_frustum_culling_non_indexed_bind_group(
                non_indexed_work_item_buffer,
            ),
        })
    }

    /// Creates the bind group for mesh preprocessing of indexed meshes when GPU
    /// frustum culling is enabled, but GPU occlusion culling is disabled.
    fn create_indirect_frustum_culling_indexed_bind_group(
        &self,
        indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
    ) -> Option<BindGroup> {
        let mesh_culling_data_buffer = self.mesh_culling_data_buffer.buffer()?;
        let view_uniforms_binding = self.view_uniforms.uniforms.binding()?;

        match (
            self.phase_indirect_parameters_buffers
                .indexed
                .cpu_metadata_buffer(),
            self.phase_indirect_parameters_buffers
                .indexed
                .gpu_metadata_buffer(),
            indexed_work_item_buffer.buffer(),
        ) {
            (
                Some(indexed_cpu_metadata_buffer),
                Some(indexed_gpu_metadata_buffer),
                Some(indexed_work_item_gpu_buffer),
            ) => {
                // Don't use `as_entire_binding()` here; the shader reads the array
                // length and the underlying buffer may be longer than the actual size
                // of the vector.
                let indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                    indexed_work_item_buffer.len() as u64
                        * u64::from(PreprocessWorkItem::min_size()),
                )
                .ok();

                Some(
                    self.render_device.create_bind_group(
                        "preprocess_gpu_indexed_frustum_culling_bind_group",
                        &self
                            .pipelines
                            .gpu_frustum_culling_preprocess
                            .bind_group_layout,
                        &BindGroupEntries::with_indices((
                            (3, self.current_input_buffer.as_entire_binding()),
                            (4, self.previous_input_buffer.as_entire_binding()),
                            (
                                5,
                                BindingResource::Buffer(BufferBinding {
                                    buffer: indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: indexed_work_item_buffer_size,
                                }),
                            ),
                            (6, self.data_buffer.as_entire_binding()),
                            (7, indexed_cpu_metadata_buffer.as_entire_binding()),
                            (8, indexed_gpu_metadata_buffer.as_entire_binding()),
                            (9, mesh_culling_data_buffer.as_entire_binding()),
                            (0, view_uniforms_binding.clone()),
                        )),
                    ),
                )
            }
            _ => None,
        }
    }

    /// Creates the bind group for mesh preprocessing of non-indexed meshes when
    /// GPU frustum culling is enabled, but GPU occlusion culling is disabled.
    fn create_indirect_frustum_culling_non_indexed_bind_group(
        &self,
        non_indexed_work_item_buffer: &RawBufferVec<PreprocessWorkItem>,
    ) -> Option<BindGroup> {
        let mesh_culling_data_buffer = self.mesh_culling_data_buffer.buffer()?;
        let view_uniforms_binding = self.view_uniforms.uniforms.binding()?;

        match (
            self.phase_indirect_parameters_buffers
                .non_indexed
                .cpu_metadata_buffer(),
            self.phase_indirect_parameters_buffers
                .non_indexed
                .gpu_metadata_buffer(),
            non_indexed_work_item_buffer.buffer(),
        ) {
            (
                Some(non_indexed_cpu_metadata_buffer),
                Some(non_indexed_gpu_metadata_buffer),
                Some(non_indexed_work_item_gpu_buffer),
            ) => {
                // Don't use `as_entire_binding()` here; the shader reads the array
                // length and the underlying buffer may be longer than the actual size
                // of the vector.
                let non_indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                    non_indexed_work_item_buffer.len() as u64
                        * u64::from(PreprocessWorkItem::min_size()),
                )
                .ok();

                Some(
                    self.render_device.create_bind_group(
                        "preprocess_gpu_non_indexed_frustum_culling_bind_group",
                        &self
                            .pipelines
                            .gpu_frustum_culling_preprocess
                            .bind_group_layout,
                        &BindGroupEntries::with_indices((
                            (3, self.current_input_buffer.as_entire_binding()),
                            (4, self.previous_input_buffer.as_entire_binding()),
                            (
                                5,
                                BindingResource::Buffer(BufferBinding {
                                    buffer: non_indexed_work_item_gpu_buffer,
                                    offset: 0,
                                    size: non_indexed_work_item_buffer_size,
                                }),
                            ),
                            (6, self.data_buffer.as_entire_binding()),
                            (7, non_indexed_cpu_metadata_buffer.as_entire_binding()),
                            (8, non_indexed_gpu_metadata_buffer.as_entire_binding()),
                            (9, mesh_culling_data_buffer.as_entire_binding()),
                            (0, view_uniforms_binding.clone()),
                        )),
                    ),
                )
            }
            _ => None,
        }
    }
}

/// A system that creates bind groups from the indirect parameters metadata and
/// data buffers for the indirect batch set reset shader and the indirect
/// parameter building shader.
fn create_build_indirect_parameters_bind_groups(
    commands: &mut Commands,
    render_device: &RenderDevice,
    pipelines: &PreprocessPipelines,
    current_input_buffer: &Buffer,
    indirect_parameters_buffers: &IndirectParametersBuffers,
) {
    let mut build_indirect_parameters_bind_groups = BuildIndirectParametersBindGroups::new();

    for (phase_type_id, phase_indirect_parameters_buffer) in indirect_parameters_buffers.iter() {
        build_indirect_parameters_bind_groups.insert(
            *phase_type_id,
            PhaseBuildIndirectParametersBindGroups {
                reset_indexed_indirect_batch_sets: match (phase_indirect_parameters_buffer
                    .indexed
                    .batch_sets_buffer(),)
                {
                    (Some(indexed_batch_sets_buffer),) => Some(
                        render_device.create_bind_group(
                            "reset_indexed_indirect_batch_sets_bind_group",
                            // The early bind group is good for the main phase and late
                            // phase too. They bind the same buffers.
                            &pipelines
                                .early_phase
                                .reset_indirect_batch_sets
                                .bind_group_layout,
                            &BindGroupEntries::sequential((
                                indexed_batch_sets_buffer.as_entire_binding(),
                            )),
                        ),
                    ),
                    _ => None,
                },

                reset_non_indexed_indirect_batch_sets: match (phase_indirect_parameters_buffer
                    .non_indexed
                    .batch_sets_buffer(),)
                {
                    (Some(non_indexed_batch_sets_buffer),) => Some(
                        render_device.create_bind_group(
                            "reset_non_indexed_indirect_batch_sets_bind_group",
                            // The early bind group is good for the main phase and late
                            // phase too. They bind the same buffers.
                            &pipelines
                                .early_phase
                                .reset_indirect_batch_sets
                                .bind_group_layout,
                            &BindGroupEntries::sequential((
                                non_indexed_batch_sets_buffer.as_entire_binding(),
                            )),
                        ),
                    ),
                    _ => None,
                },

                build_indexed_indirect: match (
                    phase_indirect_parameters_buffer
                        .indexed
                        .cpu_metadata_buffer(),
                    phase_indirect_parameters_buffer
                        .indexed
                        .gpu_metadata_buffer(),
                    phase_indirect_parameters_buffer.indexed.data_buffer(),
                    phase_indirect_parameters_buffer.indexed.batch_sets_buffer(),
                ) {
                    (
                        Some(indexed_indirect_parameters_cpu_metadata_buffer),
                        Some(indexed_indirect_parameters_gpu_metadata_buffer),
                        Some(indexed_indirect_parameters_data_buffer),
                        Some(indexed_batch_sets_buffer),
                    ) => Some(
                        render_device.create_bind_group(
                            "build_indexed_indirect_parameters_bind_group",
                            // The frustum culling bind group is good for occlusion culling
                            // too. They bind the same buffers.
                            &pipelines
                                .gpu_frustum_culling_build_indexed_indirect_params
                                .bind_group_layout,
                            &BindGroupEntries::sequential((
                                current_input_buffer.as_entire_binding(),
                                // Don't use `as_entire_binding` here; the shader reads
                                // the length and `RawBufferVec` overallocates.
                                BufferBinding {
                                    buffer: indexed_indirect_parameters_cpu_metadata_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        phase_indirect_parameters_buffer.indexed.batch_count()
                                            as u64
                                            * size_of::<IndirectParametersCpuMetadata>() as u64,
                                    ),
                                },
                                BufferBinding {
                                    buffer: indexed_indirect_parameters_gpu_metadata_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        phase_indirect_parameters_buffer.indexed.batch_count()
                                            as u64
                                            * size_of::<IndirectParametersGpuMetadata>() as u64,
                                    ),
                                },
                                indexed_batch_sets_buffer.as_entire_binding(),
                                indexed_indirect_parameters_data_buffer.as_entire_binding(),
                            )),
                        ),
                    ),
                    _ => None,
                },

                build_non_indexed_indirect: match (
                    phase_indirect_parameters_buffer
                        .non_indexed
                        .cpu_metadata_buffer(),
                    phase_indirect_parameters_buffer
                        .non_indexed
                        .gpu_metadata_buffer(),
                    phase_indirect_parameters_buffer.non_indexed.data_buffer(),
                    phase_indirect_parameters_buffer
                        .non_indexed
                        .batch_sets_buffer(),
                ) {
                    (
                        Some(non_indexed_indirect_parameters_cpu_metadata_buffer),
                        Some(non_indexed_indirect_parameters_gpu_metadata_buffer),
                        Some(non_indexed_indirect_parameters_data_buffer),
                        Some(non_indexed_batch_sets_buffer),
                    ) => Some(
                        render_device.create_bind_group(
                            "build_non_indexed_indirect_parameters_bind_group",
                            // The frustum culling bind group is good for occlusion culling
                            // too. They bind the same buffers.
                            &pipelines
                                .gpu_frustum_culling_build_non_indexed_indirect_params
                                .bind_group_layout,
                            &BindGroupEntries::sequential((
                                current_input_buffer.as_entire_binding(),
                                // Don't use `as_entire_binding` here; the shader reads
                                // the length and `RawBufferVec` overallocates.
                                BufferBinding {
                                    buffer: non_indexed_indirect_parameters_cpu_metadata_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        phase_indirect_parameters_buffer.non_indexed.batch_count()
                                            as u64
                                            * size_of::<IndirectParametersCpuMetadata>() as u64,
                                    ),
                                },
                                BufferBinding {
                                    buffer: non_indexed_indirect_parameters_gpu_metadata_buffer,
                                    offset: 0,
                                    size: NonZeroU64::new(
                                        phase_indirect_parameters_buffer.non_indexed.batch_count()
                                            as u64
                                            * size_of::<IndirectParametersGpuMetadata>() as u64,
                                    ),
                                },
                                non_indexed_batch_sets_buffer.as_entire_binding(),
                                non_indexed_indirect_parameters_data_buffer.as_entire_binding(),
                            )),
                        ),
                    ),
                    _ => None,
                },
            },
        );
    }

    commands.insert_resource(build_indirect_parameters_bind_groups);
}

/// Writes the information needed to do GPU mesh culling to the GPU.
pub fn write_mesh_culling_data_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut mesh_culling_data_buffer: ResMut<MeshCullingDataBuffer>,
) {
    mesh_culling_data_buffer.write_buffer(&render_device, &render_queue);
}
