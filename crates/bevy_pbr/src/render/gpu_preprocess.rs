//! GPU mesh preprocessing.
//!
//! This is an optional pass that uses a compute shader to reduce the amount of
//! data that has to be transferred from the CPU to the GPU. When enabled,
//! instead of transferring [`MeshUniform`]s to the GPU, we transfer the smaller
//! [`MeshInputUniform`]s instead and use the GPU to calculate the remaining
//! derived fields in [`MeshUniform`].

use std::num::NonZeroU64;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryState},
    schedule::{common_conditions::resource_exists, IntoSystemConfigs as _},
    system::{lifetimeless::Read, Commands, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    batching::gpu_preprocessing::{
        BatchedInstanceBuffers, GpuPreprocessingSupport, IndirectParameters,
        IndirectParametersBuffer, PreprocessWorkItem,
    },
    render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindingResource, BufferBinding,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor,
        DynamicBindGroupLayoutEntries, PipelineCache, Shader, ShaderStages, ShaderType,
        SpecializedComputePipeline, SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{GpuCulling, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use bevy_utils::tracing::warn;
use bitflags::bitflags;
use smallvec::{smallvec, SmallVec};

use crate::{
    graph::NodePbr, MeshCullingData, MeshCullingDataBuffer, MeshInputUniform, MeshUniform,
};

/// The handle to the `mesh_preprocess.wgsl` compute shader.
pub const MESH_PREPROCESS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(16991728318640779533);

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

/// The render node for the mesh uniform building pass.
pub struct GpuPreprocessNode {
    view_query: QueryState<(
        Entity,
        Read<PreprocessBindGroup>,
        Read<ViewUniformOffset>,
        Has<GpuCulling>,
    )>,
}

/// The compute shader pipelines for the mesh uniform building pass.
#[derive(Resource)]
pub struct PreprocessPipelines {
    /// The pipeline used for CPU culling. This pipeline doesn't populate
    /// indirect parameters.
    pub direct: PreprocessPipeline,
    /// The pipeline used for GPU culling. This pipeline populates indirect
    /// parameters.
    pub gpu_culling: PreprocessPipeline,
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

bitflags! {
    /// Specifies variants of the mesh preprocessing shader.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PreprocessPipelineKey: u8 {
        /// Whether GPU culling is in use.
        ///
        /// This `#define`'s `GPU_CULLING` in the shader.
        const GPU_CULLING = 1;
    }
}

/// The compute shader bind group for the mesh uniform building pass.
///
/// This goes on the view.
#[derive(Component)]
pub struct PreprocessBindGroup(BindGroup);

impl Plugin for GpuMeshPreprocessPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MESH_PREPROCESS_SHADER_HANDLE,
            "mesh_preprocess.wgsl",
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
        if !self.use_gpu_instance_buffer_builder
            || *gpu_preprocessing_support == GpuPreprocessingSupport::None
        {
            return;
        }

        // Stitch the node in.
        render_app
            .add_render_graph_node::<GpuPreprocessNode>(Core3d, NodePbr::GpuPreprocess)
            .add_render_graph_edges(Core3d, (NodePbr::GpuPreprocess, Node3d::Prepass))
            .add_render_graph_edges(Core3d, (NodePbr::GpuPreprocess, NodePbr::ShadowPass))
            .init_resource::<PreprocessPipelines>()
            .init_resource::<SpecializedComputePipelines<PreprocessPipeline>>()
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
            );
    }
}

impl FromWorld for GpuPreprocessNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for GpuPreprocessNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
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

        // Run the compute passes.
        for (view, bind_group, view_uniform_offset, gpu_culling) in
            self.view_query.iter_manual(world)
        {
            // Grab the index buffer for this view.
            let Some(index_buffer) = index_buffers.get(&view) else {
                warn!("The preprocessing index buffer wasn't present");
                return Ok(());
            };

            // Select the right pipeline, depending on whether GPU culling is in
            // use.
            let maybe_pipeline_id = if gpu_culling {
                preprocess_pipelines.gpu_culling.pipeline_id
            } else {
                preprocess_pipelines.direct.pipeline_id
            };

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

            let mut dynamic_offsets: SmallVec<[u32; 1]> = smallvec![];
            if gpu_culling {
                dynamic_offsets.push(view_uniform_offset.offset);
            }
            compute_pass.set_bind_group(0, &bind_group.0, &dynamic_offsets);

            let workgroup_count = index_buffer.buffer.len().div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }

        Ok(())
    }
}

impl PreprocessPipelines {
    pub(crate) fn pipelines_are_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        self.direct.is_loaded(pipeline_cache) && self.gpu_culling.is_loaded(pipeline_cache)
    }
}

impl PreprocessPipeline {
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
                // `indirect_parameters`
                storage_buffer::<IndirectParameters>(/*has_dynamic_offset=*/ false),
                // `mesh_culling_data`
                storage_buffer_read_only::<MeshCullingData>(/*has_dynamic_offset=*/ false),
                // `view`
                uniform_buffer::<ViewUniform>(/*has_dynamic_offset=*/ true),
            ));

        let direct_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms direct bind group layout",
            &direct_bind_group_layout_entries,
        );
        let gpu_culling_bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms GPU culling bind group layout",
            &gpu_culling_bind_group_layout_entries,
        );

        PreprocessPipelines {
            direct: PreprocessPipeline {
                bind_group_layout: direct_bind_group_layout,
                pipeline_id: None,
            },
            gpu_culling: PreprocessPipeline {
                bind_group_layout: gpu_culling_bind_group_layout,
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

/// A system that specializes the `mesh_preprocess.wgsl` pipelines if necessary.
pub fn prepare_preprocess_pipelines(
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<PreprocessPipeline>>,
    mut preprocess_pipelines: ResMut<PreprocessPipelines>,
) {
    preprocess_pipelines.direct.prepare(
        &pipeline_cache,
        &mut pipelines,
        PreprocessPipelineKey::empty(),
    );
    preprocess_pipelines.gpu_culling.prepare(
        &pipeline_cache,
        &mut pipelines,
        PreprocessPipelineKey::GPU_CULLING,
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

/// A system that attaches the mesh uniform buffers to the bind groups for the
/// variants of the mesh preprocessing compute shader.
pub fn prepare_preprocess_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    batched_instance_buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    indirect_parameters_buffer: Res<IndirectParametersBuffer>,
    mesh_culling_data_buffer: Res<MeshCullingDataBuffer>,
    view_uniforms: Res<ViewUniforms>,
    pipelines: Res<PreprocessPipelines>,
) {
    // Grab the `BatchedInstanceBuffers`.
    let BatchedInstanceBuffers {
        data_buffer: ref data_buffer_vec,
        work_item_buffers: ref index_buffers,
        current_input_buffer: ref current_input_buffer_vec,
        previous_input_buffer: ref previous_input_buffer_vec,
    } = batched_instance_buffers.into_inner();

    let (Some(current_input_buffer), Some(previous_input_buffer), Some(data_buffer)) = (
        current_input_buffer_vec.buffer(),
        previous_input_buffer_vec.buffer(),
        data_buffer_vec.buffer(),
    ) else {
        return;
    };

    for (view, index_buffer_vec) in index_buffers {
        let Some(index_buffer) = index_buffer_vec.buffer.buffer() else {
            continue;
        };

        // Don't use `as_entire_binding()` here; the shader reads the array
        // length and the underlying buffer may be longer than the actual size
        // of the vector.
        let index_buffer_size = NonZeroU64::try_from(
            index_buffer_vec.buffer.len() as u64 * u64::from(PreprocessWorkItem::min_size()),
        )
        .ok();

        let bind_group = if index_buffer_vec.gpu_culling {
            let (
                Some(indirect_parameters_buffer),
                Some(mesh_culling_data_buffer),
                Some(view_uniforms_binding),
            ) = (
                indirect_parameters_buffer.buffer(),
                mesh_culling_data_buffer.buffer(),
                view_uniforms.uniforms.binding(),
            )
            else {
                continue;
            };

            PreprocessBindGroup(render_device.create_bind_group(
                "preprocess_gpu_culling_bind_group",
                &pipelines.gpu_culling.bind_group_layout,
                &BindGroupEntries::sequential((
                    current_input_buffer.as_entire_binding(),
                    previous_input_buffer.as_entire_binding(),
                    BindingResource::Buffer(BufferBinding {
                        buffer: index_buffer,
                        offset: 0,
                        size: index_buffer_size,
                    }),
                    data_buffer.as_entire_binding(),
                    indirect_parameters_buffer.as_entire_binding(),
                    mesh_culling_data_buffer.as_entire_binding(),
                    view_uniforms_binding,
                )),
            ))
        } else {
            PreprocessBindGroup(render_device.create_bind_group(
                "preprocess_direct_bind_group",
                &pipelines.direct.bind_group_layout,
                &BindGroupEntries::sequential((
                    current_input_buffer.as_entire_binding(),
                    previous_input_buffer.as_entire_binding(),
                    BindingResource::Buffer(BufferBinding {
                        buffer: index_buffer,
                        offset: 0,
                        size: index_buffer_size,
                    }),
                    data_buffer.as_entire_binding(),
                )),
            ))
        };

        commands.entity(*view).insert(bind_group);
    }
}

/// Writes the information needed to do GPU mesh culling to the GPU.
pub fn write_mesh_culling_data_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut mesh_culling_data_buffer: ResMut<MeshCullingDataBuffer>,
) {
    mesh_culling_data_buffer.write_buffer(&render_device, &render_queue);
    mesh_culling_data_buffer.clear();
}
