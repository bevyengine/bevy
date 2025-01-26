//! GPU mesh preprocessing.
//!
//! This is an optional pass that uses a compute shader to reduce the amount of
//! data that has to be transferred from the CPU to the GPU. When enabled,
//! instead of transferring [`MeshUniform`]s to the GPU, we transfer the smaller
//! [`MeshInputUniform`]s instead and use the GPU to calculate the remaining
//! derived fields in [`MeshUniform`].

use core::num::{NonZero, NonZeroU64};

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryState, Without},
    resource::Resource,
    schedule::{common_conditions::resource_exists, IntoSystemConfigs as _},
    system::{lifetimeless::Read, Commands, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_render::{
    batching::gpu_preprocessing::{
        BatchedInstanceBuffers, GpuPreprocessingSupport, IndirectBatchSet,
        IndirectParametersBuffers, IndirectParametersIndexed, IndirectParametersMetadata,
        IndirectParametersNonIndexed, PreprocessWorkItem, PreprocessWorkItemBuffers,
    },
    render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindingResource, Buffer, BufferBinding,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor,
        DynamicBindGroupLayoutEntries, PipelineCache, Shader, ShaderStages, ShaderType,
        SpecializedComputePipeline, SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    view::{NoIndirectDrawing, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use bevy_utils::TypeIdMap;
use bitflags::bitflags;
use smallvec::{smallvec, SmallVec};
use tracing::warn;

use crate::{
    graph::NodePbr, MeshCullingData, MeshCullingDataBuffer, MeshInputUniform, MeshUniform,
};

use super::ViewLightEntities;

/// The handle to the `mesh_preprocess.wgsl` compute shader.
pub const MESH_PREPROCESS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(16991728318640779533);
/// The handle to the `mesh_preprocess_types.wgsl` compute shader.
pub const MESH_PREPROCESS_TYPES_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2720440370122465935);
/// The handle to the `build_indirect_params.wgsl` compute shader.
pub const BUILD_INDIRECT_PARAMS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(3711077208359699672);

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

/// The render node for the mesh preprocessing pass.
///
/// This pass runs a compute shader to cull invisible meshes (if that wasn't
/// done by the CPU), transforms them, and, if indirect drawing is on, populates
/// indirect draw parameter metadata for the subsequent
/// [`BuildIndirectParametersNode`].
pub struct GpuPreprocessNode {
    view_query: QueryState<
        (
            Entity,
            Read<PreprocessBindGroups>,
            Read<ViewUniformOffset>,
            Has<NoIndirectDrawing>,
        ),
        Without<SkipGpuPreprocess>,
    >,
    main_view_query: QueryState<Read<ViewLightEntities>>,
}

/// The render node for the indirect parameter building pass.
///
/// This node runs a compute shader on the output of the [`GpuPreprocessNode`]
/// in order to transform the [`IndirectParametersMetadata`] into
/// properly-formatted [`IndirectParametersIndexed`] and
/// [`IndirectParametersNonIndexed`].
pub struct BuildIndirectParametersNode {
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
    /// The pipeline used for GPU culling. This pipeline populates indirect
    /// parameter metadata.
    pub gpu_culling_preprocess: PreprocessPipeline,
    /// The pipeline used for indexed indirect parameter building.
    ///
    /// This pipeline converts indirect parameter metadata into indexed indirect
    /// parameters.
    pub build_indexed_indirect_params: BuildIndirectParametersPipeline,
    /// The pipeline used for non-indexed indirect parameter building.
    ///
    /// This pipeline converts indirect parameter metadata into non-indexed
    /// indirect parameters.
    pub build_non_indexed_indirect_params: BuildIndirectParametersPipeline,
}

/// The pipeline for the GPU mesh preprocessing shader.
pub struct PreprocessPipeline {
    /// The bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in `prepare_preprocess_pipelines`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

/// The pipeline for the indirect parameter building shader.
pub struct BuildIndirectParametersPipeline {
    /// The bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in `prepare_preprocess_pipelines`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

bitflags! {
    /// Specifies variants of the mesh preprocessing shader.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PreprocessPipelineKey: u8 {
        /// Whether GPU culling is in use.
        ///
        /// This `#define`'s `GPU_CULLING` in the shader.
        const GPU_CULLING = 1;
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
    /// being used.
    ///
    /// Because indirect drawing requires splitting the meshes into indexed and
    /// non-indexed meshes, there are two bind groups here.
    Indirect {
        /// The bind group used for indexed meshes.
        ///
        /// This will be `None` if there are no indexed meshes.
        indexed: Option<BindGroup>,
        /// The bind group used for non-indexed meshes.
        ///
        /// This will be `None` if there are no non-indexed meshes.
        non_indexed: Option<BindGroup>,
    },
}

/// The bind groups for the indirect parameters building compute shader.
///
/// This is shared among all views and phases.
#[derive(Resource)]
pub struct BuildIndirectParametersBindGroups {
    /// The bind group used for indexed meshes.
    ///
    /// This will be `None` if there are no indexed meshes.
    indexed: Option<BindGroup>,
    /// The bind group used for non-indexed meshes.
    ///
    /// This will be `None` if there are no non-indexed meshes.
    non_indexed: Option<BindGroup>,
}

/// Stops the `GpuPreprocessNode` attempting to generate the buffer for this view
/// useful to avoid duplicating effort if the bind group is shared between views
#[derive(Component, Default)]
pub struct SkipGpuPreprocess;

impl Plugin for GpuMeshPreprocessPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MESH_PREPROCESS_SHADER_HANDLE,
            "mesh_preprocess.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH_PREPROCESS_TYPES_SHADER_HANDLE,
            "mesh_preprocess_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            BUILD_INDIRECT_PARAMS_SHADER_HANDLE,
            "build_indirect_params.wgsl",
            Shader::from_wgsl
        );
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
            .init_resource::<SpecializedComputePipelines<BuildIndirectParametersPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_preprocess_pipelines.in_set(RenderSet::Prepare),
                    prepare_preprocess_bind_groups
                        .run_if(
                            resource_exists::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
                        )
                        .in_set(RenderSet::PrepareBindGroups),
                    write_mesh_culling_data_buffer.in_set(RenderSet::PrepareResourcesFlush),
                )
            )
            .add_render_graph_node::<GpuPreprocessNode>(Core3d, NodePbr::GpuPreprocess)
            .add_render_graph_node::<BuildIndirectParametersNode>(
                Core3d,
                NodePbr::BuildIndirectParameters
            )
            .add_render_graph_edges(
                Core3d,
                (NodePbr::GpuPreprocess, NodePbr::BuildIndirectParameters, Node3d::Prepass)
            )
            .add_render_graph_edges(
                Core3d,
                (NodePbr::GpuPreprocess, NodePbr::BuildIndirectParameters, NodePbr::ShadowPass)
            );
    }
}

impl FromWorld for GpuPreprocessNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
            main_view_query: QueryState::new(world),
        }
    }
}

impl Node for GpuPreprocessNode {
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
        // Grab the [`BatchedInstanceBuffers`].
        let BatchedInstanceBuffers {
            work_item_buffers: ref index_buffers,
            ..
        } = world.resource::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("mesh preprocessing"),
                    timestamp_writes: None,
                });

        let mut all_views: SmallVec<[_; 8]> = SmallVec::new();
        all_views.push(graph.view_entity());
        if let Ok(shadow_cascade_views) =
            self.main_view_query.get_manual(world, graph.view_entity())
        {
            all_views.extend(shadow_cascade_views.lights.iter().copied());
        }

        // Run the compute passes.

        for view_entity in all_views {
            let Ok((view, bind_groups, view_uniform_offset, no_indirect_drawing)) =
                self.view_query.get_manual(world, view_entity)
            else {
                continue;
            };

            // Grab the work item buffers for this view.
            let Some(view_work_item_buffers) = index_buffers.get(&view) else {
                warn!("The preprocessing index buffer wasn't present");
                continue;
            };

            // Select the right pipeline, depending on whether GPU culling is in
            // use.
            let maybe_pipeline_id = if !no_indirect_drawing {
                preprocess_pipelines.gpu_culling_preprocess.pipeline_id
            } else {
                preprocess_pipelines.direct_preprocess.pipeline_id
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
            for (phase_type_id, phase_work_item_buffers) in view_work_item_buffers {
                // Fetch the bind group for the render phase.
                let Some(phase_bind_groups) = bind_groups.get(phase_type_id) else {
                    continue;
                };

                // If we're drawing indirectly, make sure the mesh preprocessing
                // shader has access to the view info it needs to do culling.
                let mut dynamic_offsets: SmallVec<[u32; 1]> = smallvec![];
                if !no_indirect_drawing {
                    dynamic_offsets.push(view_uniform_offset.offset);
                }

                // Are we drawing directly or indirectly?
                match *phase_bind_groups {
                    PhasePreprocessBindGroups::Direct(ref bind_group) => {
                        // Invoke the mesh preprocessing shader to transform
                        // meshes only, but not cull.
                        let PreprocessWorkItemBuffers::Direct(phase_work_item_buffer) =
                            phase_work_item_buffers
                        else {
                            continue;
                        };
                        compute_pass.set_bind_group(0, bind_group, &dynamic_offsets);
                        let workgroup_count = phase_work_item_buffer.len().div_ceil(WORKGROUP_SIZE);
                        if workgroup_count > 0 {
                            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
                        }
                    }

                    PhasePreprocessBindGroups::Indirect {
                        indexed: ref maybe_indexed_bind_group,
                        non_indexed: ref maybe_non_indexed_bind_group,
                    } => {
                        // Invoke the mesh preprocessing shader to transform and
                        // cull the meshes.
                        let PreprocessWorkItemBuffers::Indirect {
                            indexed: indexed_buffer,
                            non_indexed: non_indexed_buffer,
                            ..
                        } = phase_work_item_buffers
                        else {
                            continue;
                        };

                        // Transform and cull indexed meshes if there are any.
                        if let Some(indexed_bind_group) = maybe_indexed_bind_group {
                            compute_pass.set_bind_group(0, indexed_bind_group, &dynamic_offsets);
                            let workgroup_count = indexed_buffer.len().div_ceil(WORKGROUP_SIZE);
                            if workgroup_count > 0 {
                                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
                            }
                        }

                        // Transform and cull non-indexed meshes if there are any.
                        if let Some(non_indexed_bind_group) = maybe_non_indexed_bind_group {
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

        Ok(())
    }
}

impl FromWorld for BuildIndirectParametersNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for BuildIndirectParametersNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Fetch the bind group.
        let Some(build_indirect_params_bind_groups) =
            world.get_resource::<BuildIndirectParametersBindGroups>()
        else {
            return Ok(());
        };

        // Fetch the pipelines and the buffers we need.
        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_pipelines = world.resource::<PreprocessPipelines>();
        let indirect_parameters_buffers = world.resource::<IndirectParametersBuffers>();

        // Create the compute pass.
        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("build indirect parameters"),
                    timestamp_writes: None,
                });

        // Fetch the pipelines.

        let (maybe_indexed_pipeline_id, maybe_non_indexed_pipeline_id) = (
            preprocess_pipelines
                .build_indexed_indirect_params
                .pipeline_id,
            preprocess_pipelines
                .build_non_indexed_indirect_params
                .pipeline_id,
        );

        let (
            Some(build_indexed_indirect_params_pipeline_id),
            Some(build_non_indexed_indirect_params_pipeline_id),
        ) = (maybe_indexed_pipeline_id, maybe_non_indexed_pipeline_id)
        else {
            warn!("The build indirect parameters pipelines weren't ready");
            return Ok(());
        };

        let (
            Some(build_indexed_indirect_params_pipeline),
            Some(build_non_indexed_indirect_params_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(build_indexed_indirect_params_pipeline_id),
            pipeline_cache.get_compute_pipeline(build_non_indexed_indirect_params_pipeline_id),
        )
        else {
            // This will happen while the pipeline is being compiled and is fine.
            return Ok(());
        };

        // Transform the [`IndirectParametersMetadata`] that the GPU mesh
        // preprocessing phase wrote to [`IndirectParametersIndexed`] for
        // indexed meshes, if we have any.
        if let Some(ref build_indirect_indexed_params_bind_group) =
            build_indirect_params_bind_groups.indexed
        {
            compute_pass.set_pipeline(build_indexed_indirect_params_pipeline);
            compute_pass.set_bind_group(0, build_indirect_indexed_params_bind_group, &[]);
            let workgroup_count = indirect_parameters_buffers
                .indexed_batch_count()
                .div_ceil(WORKGROUP_SIZE);
            if workgroup_count > 0 {
                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }
        }

        // Transform the [`IndirectParametersMetadata`] that the GPU mesh
        // preprocessing phase wrote to [`IndirectParametersNonIndexed`] for
        // non-indexed meshes, if we have any.
        if let Some(ref build_indirect_non_indexed_params_bind_group) =
            build_indirect_params_bind_groups.non_indexed
        {
            compute_pass.set_pipeline(build_non_indexed_indirect_params_pipeline);
            compute_pass.set_bind_group(0, build_indirect_non_indexed_params_bind_group, &[]);
            let workgroup_count = indirect_parameters_buffers
                .non_indexed_batch_count()
                .div_ceil(WORKGROUP_SIZE);
            if workgroup_count > 0 {
                compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }
        }

        Ok(())
    }
}

impl PreprocessPipelines {
    /// Returns true if the preprocessing and indirect parameters pipelines have
    /// been loaded or false otherwise.
    pub(crate) fn pipelines_are_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        self.direct_preprocess.is_loaded(pipeline_cache)
            && self.gpu_culling_preprocess.is_loaded(pipeline_cache)
            && self.build_indexed_indirect_params.is_loaded(pipeline_cache)
            && self
                .build_non_indexed_indirect_params
                .is_loaded(pipeline_cache)
    }
}

impl PreprocessPipeline {
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
        let mut shader_defs = vec![];
        if key.contains(PreprocessPipelineKey::GPU_CULLING) {
            shader_defs.push("INDIRECT".into());
            shader_defs.push("FRUSTUM_CULLING".into());
        }

        ComputePipelineDescriptor {
            label: Some(
                format!(
                    "mesh preprocessing ({})",
                    if key.contains(PreprocessPipelineKey::GPU_CULLING) {
                        "GPU culling"
                    } else {
                        "direct"
                    }
                )
                .into(),
            ),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: MESH_PREPROCESS_SHADER_HANDLE,
            shader_defs,
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

impl FromWorld for PreprocessPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // GPU culling bind group parameters are a superset of those in the CPU
        // culling (direct) shader.
        let direct_bind_group_layout_entries = preprocess_direct_bind_group_layout_entries();
        let gpu_culling_bind_group_layout_entries = preprocess_direct_bind_group_layout_entries()
            .extend_sequential((
                // `indirect_parameters_metadata`
                storage_buffer::<IndirectParametersMetadata>(/* has_dynamic_offset= */ false),
                // `mesh_culling_data`
                storage_buffer_read_only::<MeshCullingData>(/* has_dynamic_offset= */ false),
                // `view`
                uniform_buffer::<ViewUniform>(/* has_dynamic_offset= */ true),
            ));

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
        let gpu_culling_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms GPU culling bind group layout",
            &gpu_culling_bind_group_layout_entries,
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

        PreprocessPipelines {
            direct_preprocess: PreprocessPipeline {
                bind_group_layout: direct_bind_group_layout,
                pipeline_id: None,
            },
            gpu_culling_preprocess: PreprocessPipeline {
                bind_group_layout: gpu_culling_bind_group_layout,
                pipeline_id: None,
            },
            build_indexed_indirect_params: BuildIndirectParametersPipeline {
                bind_group_layout: build_indexed_indirect_params_bind_group_layout,
                pipeline_id: None,
            },
            build_non_indexed_indirect_params: BuildIndirectParametersPipeline {
                bind_group_layout: build_non_indexed_indirect_params_bind_group_layout,
                pipeline_id: None,
            },
        }
    }
}

fn preprocess_direct_bind_group_layout_entries() -> DynamicBindGroupLayoutEntries {
    DynamicBindGroupLayoutEntries::sequential(
        ShaderStages::COMPUTE,
        (
            // `current_input`
            storage_buffer_read_only::<MeshInputUniform>(false),
            // `previous_input`
            storage_buffer_read_only::<MeshInputUniform>(false),
            // `indices`
            storage_buffer_read_only::<PreprocessWorkItem>(false),
            // `output`
            storage_buffer::<MeshUniform>(false),
        ),
    )
}

// Returns the first 3 bind group layout entries shared between all invocations
// of the indirect parameters building shader.
fn build_indirect_params_bind_group_layout_entries() -> DynamicBindGroupLayoutEntries {
    DynamicBindGroupLayoutEntries::sequential(
        ShaderStages::COMPUTE,
        (
            storage_buffer_read_only::<MeshInputUniform>(false),
            storage_buffer_read_only::<IndirectParametersMetadata>(false),
            storage_buffer::<IndirectBatchSet>(false),
        ),
    )
}

/// A system that specializes the `mesh_preprocess.wgsl` and
/// `build_indirect_params.wgsl` pipelines if necessary.
pub fn prepare_preprocess_pipelines(
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    mut specialized_preprocess_pipelines: ResMut<SpecializedComputePipelines<PreprocessPipeline>>,
    mut specialized_build_indirect_parameters_pipelines: ResMut<
        SpecializedComputePipelines<BuildIndirectParametersPipeline>,
    >,
    mut preprocess_pipelines: ResMut<PreprocessPipelines>,
) {
    preprocess_pipelines.direct_preprocess.prepare(
        &pipeline_cache,
        &mut specialized_preprocess_pipelines,
        PreprocessPipelineKey::empty(),
    );
    preprocess_pipelines.gpu_culling_preprocess.prepare(
        &pipeline_cache,
        &mut specialized_preprocess_pipelines,
        PreprocessPipelineKey::GPU_CULLING,
    );

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

    preprocess_pipelines.build_indexed_indirect_params.prepare(
        &pipeline_cache,
        &mut specialized_build_indirect_parameters_pipelines,
        build_indirect_parameters_pipeline_key | BuildIndirectParametersPipelineKey::INDEXED,
    );
    preprocess_pipelines
        .build_non_indexed_indirect_params
        .prepare(
            &pipeline_cache,
            &mut specialized_build_indirect_parameters_pipelines,
            build_indirect_parameters_pipeline_key,
        );
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

        ComputePipelineDescriptor {
            label: if key.contains(BuildIndirectParametersPipelineKey::INDEXED) {
                Some("build indexed indirect parameters".into())
            } else {
                Some("build non-indexed indirect parameters".into())
            },
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: BUILD_INDIRECT_PARAMS_SHADER_HANDLE,
            shader_defs,
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        }
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
pub fn prepare_preprocess_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    batched_instance_buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    indirect_parameters_buffers: Res<IndirectParametersBuffers>,
    mesh_culling_data_buffer: Res<MeshCullingDataBuffer>,
    view_uniforms: Res<ViewUniforms>,
    pipelines: Res<PreprocessPipelines>,
) {
    // Grab the `BatchedInstanceBuffers`.
    let batched_instance_buffers = batched_instance_buffers.into_inner();

    let Some(current_input_buffer) = batched_instance_buffers
        .current_input_buffer
        .buffer()
        .buffer()
    else {
        return;
    };

    // Keep track of whether any of the phases will be drawn indirectly. If
    // they are, then we'll need bind groups for the indirect parameters
    // building shader too.
    let mut any_indirect = false;

    for (view, phase_work_item_buffers) in &batched_instance_buffers.work_item_buffers {
        let mut bind_groups = TypeIdMap::default();

        for (&phase_id, work_item_buffers) in phase_work_item_buffers {
            if let Some(bind_group) = prepare_preprocess_bind_group_for_phase(
                &render_device,
                &pipelines,
                &view_uniforms,
                &indirect_parameters_buffers,
                &mesh_culling_data_buffer,
                batched_instance_buffers,
                work_item_buffers,
                &mut any_indirect,
            ) {
                bind_groups.insert(phase_id, bind_group);
            }
        }

        commands
            .entity(*view)
            .insert(PreprocessBindGroups(bind_groups));
    }

    // If any of the phases will be drawn indirectly, create the bind groups for
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

// Creates the bind group for the GPU preprocessing shader for a single phase
// for a single view.
#[expect(
    clippy::too_many_arguments,
    reason = "it's a system that needs a bunch of parameters"
)]
fn prepare_preprocess_bind_group_for_phase(
    render_device: &RenderDevice,
    pipelines: &PreprocessPipelines,
    view_uniforms: &ViewUniforms,
    indirect_parameters_buffers: &IndirectParametersBuffers,
    mesh_culling_data_buffer: &MeshCullingDataBuffer,
    batched_instance_buffers: &BatchedInstanceBuffers<MeshUniform, MeshInputUniform>,
    work_item_buffers: &PreprocessWorkItemBuffers,
    any_indirect: &mut bool,
) -> Option<PhasePreprocessBindGroups> {
    // Get the current input buffers.

    let BatchedInstanceBuffers {
        data_buffer: ref data_buffer_vec,
        current_input_buffer: ref current_input_buffer_vec,
        previous_input_buffer: ref previous_input_buffer_vec,
        ..
    } = batched_instance_buffers;

    let current_input_buffer = current_input_buffer_vec.buffer().buffer()?;
    let previous_input_buffer = previous_input_buffer_vec.buffer().buffer()?;
    let data_buffer = data_buffer_vec.buffer()?;

    // Build the appropriate bind group, depending on whether we're drawing
    // directly or indirectly.

    match *work_item_buffers {
        PreprocessWorkItemBuffers::Direct(ref work_item_buffer_vec) => {
            let work_item_buffer = work_item_buffer_vec.buffer()?;

            // Don't use `as_entire_binding()` here; the shader reads the array
            // length and the underlying buffer may be longer than the actual size
            // of the vector.
            let work_item_buffer_size = NonZero::<u64>::try_from(
                work_item_buffer_vec.len() as u64 * u64::from(PreprocessWorkItem::min_size()),
            )
            .ok();

            Some(PhasePreprocessBindGroups::Direct(
                render_device.create_bind_group(
                    "preprocess_direct_bind_group",
                    &pipelines.direct_preprocess.bind_group_layout,
                    &BindGroupEntries::sequential((
                        current_input_buffer.as_entire_binding(),
                        previous_input_buffer.as_entire_binding(),
                        BindingResource::Buffer(BufferBinding {
                            buffer: work_item_buffer,
                            offset: 0,
                            size: work_item_buffer_size,
                        }),
                        data_buffer.as_entire_binding(),
                    )),
                ),
            ))
        }

        PreprocessWorkItemBuffers::Indirect {
            indexed: ref indexed_buffer,
            non_indexed: ref non_indexed_buffer,
        } => {
            // For indirect drawing, we need two separate bind groups, one for indexed meshes and one for non-indexed meshes.

            let mesh_culling_data_buffer = mesh_culling_data_buffer.buffer()?;
            let view_uniforms_binding = view_uniforms.uniforms.binding()?;

            let indexed_bind_group = match (
                indexed_buffer.buffer(),
                indirect_parameters_buffers.indexed_metadata_buffer(),
            ) {
                (
                    Some(indexed_work_item_buffer),
                    Some(indexed_indirect_parameters_metadata_buffer),
                ) => {
                    // Don't use `as_entire_binding()` here; the shader reads the array
                    // length and the underlying buffer may be longer than the actual size
                    // of the vector.
                    let indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                        indexed_buffer.len() as u64 * u64::from(PreprocessWorkItem::min_size()),
                    )
                    .ok();
                    Some(render_device.create_bind_group(
                        "preprocess_indexed_indirect_gpu_culling_bind_group",
                        &pipelines.gpu_culling_preprocess.bind_group_layout,
                        &BindGroupEntries::sequential((
                            current_input_buffer.as_entire_binding(),
                            previous_input_buffer.as_entire_binding(),
                            BindingResource::Buffer(BufferBinding {
                                buffer: indexed_work_item_buffer,
                                offset: 0,
                                size: indexed_work_item_buffer_size,
                            }),
                            data_buffer.as_entire_binding(),
                            indexed_indirect_parameters_metadata_buffer.as_entire_binding(),
                            mesh_culling_data_buffer.as_entire_binding(),
                            view_uniforms_binding.clone(),
                        )),
                    ))
                }
                _ => None,
            };

            let non_indexed_bind_group = match (
                non_indexed_buffer.buffer(),
                indirect_parameters_buffers.non_indexed_metadata_buffer(),
            ) {
                (
                    Some(non_indexed_work_item_buffer),
                    Some(non_indexed_indirect_parameters_metadata_buffer),
                ) => {
                    // Don't use `as_entire_binding()` here; the shader reads the array
                    // length and the underlying buffer may be longer than the actual size
                    // of the vector.
                    let non_indexed_work_item_buffer_size = NonZero::<u64>::try_from(
                        non_indexed_buffer.len() as u64 * u64::from(PreprocessWorkItem::min_size()),
                    )
                    .ok();
                    Some(render_device.create_bind_group(
                        "preprocess_non_indexed_indirect_gpu_culling_bind_group",
                        &pipelines.gpu_culling_preprocess.bind_group_layout,
                        &BindGroupEntries::sequential((
                            current_input_buffer.as_entire_binding(),
                            previous_input_buffer.as_entire_binding(),
                            BindingResource::Buffer(BufferBinding {
                                buffer: non_indexed_work_item_buffer,
                                offset: 0,
                                size: non_indexed_work_item_buffer_size,
                            }),
                            data_buffer.as_entire_binding(),
                            non_indexed_indirect_parameters_metadata_buffer.as_entire_binding(),
                            mesh_culling_data_buffer.as_entire_binding(),
                            view_uniforms_binding,
                        )),
                    ))
                }
                _ => None,
            };

            // Note that we found phases that will be drawn indirectly so that
            // we remember to build the bind groups for the indirect parameter
            // building shader.
            *any_indirect = true;

            Some(PhasePreprocessBindGroups::Indirect {
                indexed: indexed_bind_group,
                non_indexed: non_indexed_bind_group,
            })
        }
    }
}

/// A system that creates bind groups from the indirect parameters metadata and
/// data buffers for the indirect parameter building shader.
fn create_build_indirect_parameters_bind_groups(
    commands: &mut Commands,
    render_device: &RenderDevice,
    pipelines: &PreprocessPipelines,
    current_input_buffer: &Buffer,
    indirect_parameters_buffer: &IndirectParametersBuffers,
) {
    commands.insert_resource(BuildIndirectParametersBindGroups {
        indexed: match (
            indirect_parameters_buffer.indexed_metadata_buffer(),
            indirect_parameters_buffer.indexed_data_buffer(),
            indirect_parameters_buffer.indexed_batch_sets_buffer(),
        ) {
            (
                Some(indexed_indirect_parameters_metadata_buffer),
                Some(indexed_indirect_parameters_data_buffer),
                Some(indexed_batch_sets_buffer),
            ) => Some(render_device.create_bind_group(
                "build_indexed_indirect_parameters_bind_group",
                &pipelines.build_indexed_indirect_params.bind_group_layout,
                &BindGroupEntries::sequential((
                    current_input_buffer.as_entire_binding(),
                    // Don't use `as_entire_binding` here; the shader reads
                    // the length and `RawBufferVec` overallocates.
                    BufferBinding {
                        buffer: indexed_indirect_parameters_metadata_buffer,
                        offset: 0,
                        size: NonZeroU64::new(
                            indirect_parameters_buffer.indexed_batch_count() as u64
                                * size_of::<IndirectParametersMetadata>() as u64,
                        ),
                    },
                    indexed_batch_sets_buffer.as_entire_binding(),
                    indexed_indirect_parameters_data_buffer.as_entire_binding(),
                )),
            )),
            _ => None,
        },
        non_indexed: match (
            indirect_parameters_buffer.non_indexed_metadata_buffer(),
            indirect_parameters_buffer.non_indexed_data_buffer(),
            indirect_parameters_buffer.non_indexed_batch_sets_buffer(),
        ) {
            (
                Some(non_indexed_indirect_parameters_metadata_buffer),
                Some(non_indexed_indirect_parameters_data_buffer),
                Some(non_indexed_batch_sets_buffer),
            ) => Some(
                render_device.create_bind_group(
                    "build_non_indexed_indirect_parameters_bind_group",
                    &pipelines
                        .build_non_indexed_indirect_params
                        .bind_group_layout,
                    &BindGroupEntries::sequential((
                        current_input_buffer.as_entire_binding(),
                        // Don't use `as_entire_binding` here; the shader reads
                        // the length and `RawBufferVec` overallocates.
                        BufferBinding {
                            buffer: non_indexed_indirect_parameters_metadata_buffer,
                            offset: 0,
                            size: NonZeroU64::new(
                                indirect_parameters_buffer.non_indexed_batch_count() as u64
                                    * size_of::<IndirectParametersMetadata>() as u64,
                            ),
                        },
                        non_indexed_batch_sets_buffer.as_entire_binding(),
                        non_indexed_indirect_parameters_data_buffer.as_entire_binding(),
                    )),
                ),
            ),
            _ => None,
        },
    });
}

/// Writes the information needed to do GPU mesh culling to the GPU.
pub fn write_mesh_culling_data_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut mesh_culling_data_buffer: ResMut<MeshCullingDataBuffer>,
) {
    mesh_culling_data_buffer.write_buffer(&render_device, &render_queue);
}
