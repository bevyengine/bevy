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
    query::QueryState,
    schedule::{common_conditions::resource_exists, IntoSystemConfigs as _},
    system::{lifetimeless::Read, Commands, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    batching::gpu_preprocessing::{self, BatchedInstanceBuffers, PreprocessWorkItem},
    render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only},
        BindGroup, BindGroupEntries, BindGroupLayout, BindingResource, BufferBinding,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor,
        DynamicBindGroupLayoutEntries, PipelineCache, Shader, ShaderStages, ShaderType,
        SpecializedComputePipeline, SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice},
    Render, RenderApp, RenderSet,
};
use bevy_utils::tracing::warn;

use crate::{graph::NodePbr, MeshInputUniform, MeshUniform};

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
    view_query: QueryState<(Entity, Read<PreprocessBindGroup>)>,
}

/// The compute shader pipeline for the mesh uniform building pass.
#[derive(Resource)]
pub struct PreprocessPipeline {
    /// The single bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in in `prepare_preprocess_pipeline`.
    pub pipeline_id: Option<CachedComputePipelineId>,
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
        let render_device = render_app.world().resource::<RenderDevice>();
        if !self.use_gpu_instance_buffer_builder
            || !gpu_preprocessing::can_preprocess_on_gpu(render_device)
        {
            return;
        }

        // Stitch the node in.
        render_app
            .add_render_graph_node::<GpuPreprocessNode>(Core3d, NodePbr::GpuPreprocess)
            .add_render_graph_edges(Core3d, (NodePbr::GpuPreprocess, Node3d::Prepass))
            .add_render_graph_edges(Core3d, (NodePbr::GpuPreprocess, NodePbr::ShadowPass))
            .init_resource::<PreprocessPipeline>()
            .init_resource::<SpecializedComputePipelines<PreprocessPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_preprocess_pipeline.in_set(RenderSet::Prepare),
                    prepare_preprocess_bind_groups
                        .run_if(
                            resource_exists::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
                        )
                        .in_set(RenderSet::PrepareBindGroups),
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
        let preprocess_pipeline = world.resource::<PreprocessPipeline>();

        let Some(preprocess_pipeline_id) = preprocess_pipeline.pipeline_id else {
            warn!("The build mesh uniforms pipeline wasn't created");
            return Ok(());
        };

        let Some(preprocess_pipeline) = pipeline_cache.get_compute_pipeline(preprocess_pipeline_id)
        else {
            // This will happen while the pipeline is being compiled and is fine.
            return Ok(());
        };

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("mesh preprocessing"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(preprocess_pipeline);

        // Run the compute passes.
        for (view, bind_group) in self.view_query.iter_manual(world) {
            let index_buffer = &index_buffers[&view];
            compute_pass.set_bind_group(0, &bind_group.0, &[]);
            let workgroup_count = index_buffer.len().div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }

        Ok(())
    }
}

impl SpecializedComputePipeline for PreprocessPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("mesh preprocessing".into()),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: MESH_PREPROCESS_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
        }
    }
}

impl FromWorld for PreprocessPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout_entries = DynamicBindGroupLayoutEntries::sequential(
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
        );

        let bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms bind group layout",
            &bind_group_layout_entries,
        );

        PreprocessPipeline {
            bind_group_layout,
            pipeline_id: None,
        }
    }
}

/// A system that specializes the `mesh_preprocess.wgsl` pipeline if necessary.
pub fn prepare_preprocess_pipeline(
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<PreprocessPipeline>>,
    mut preprocess_pipeline: ResMut<PreprocessPipeline>,
) {
    if preprocess_pipeline.pipeline_id.is_some() {
        return;
    }

    let preprocess_pipeline_id = pipelines.specialize(&pipeline_cache, &preprocess_pipeline, ());
    preprocess_pipeline.pipeline_id = Some(preprocess_pipeline_id);
}

/// A system that attaches the mesh uniform buffers to the bind group for the
/// compute shader.
pub fn prepare_preprocess_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    batched_instance_buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    pipeline: Res<PreprocessPipeline>,
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
        let Some(index_buffer) = index_buffer_vec.buffer() else {
            continue;
        };

        // Don't use `as_entire_binding()` here; the shader reads the array
        // length and the underlying buffer may be longer than the actual size
        // of the vector.
        let index_buffer_size = NonZeroU64::try_from(
            index_buffer_vec.len() as u64 * u64::from(PreprocessWorkItem::min_size()),
        )
        .ok();

        commands
            .entity(*view)
            .insert(PreprocessBindGroup(render_device.create_bind_group(
                "preprocess_bind_group",
                &pipeline.bind_group_layout,
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
            )));
    }
}
