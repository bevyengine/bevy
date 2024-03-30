//! GPU mesh uniform building.
//!
//! This is an optional pass that uses a compute shader to reduce the amount of
//! data that has to be transferred from the CPU to the GPU. When enabled,
//! instead of transferring [`MeshUniform`]s to the GPU, we transfer the smaller
//! [`MeshInputUniform`]s instead and use the GPU to calculate the remaining
//! derived fields in [`MeshUniform`].

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{
    query::QueryItem,
    schedule::IntoSystemConfigs as _,
    system::{Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    batching::BatchedInstanceBuffers,
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only},
        BindGroup, BindGroupEntries, BindGroupLayout, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, DynamicBindGroupLayoutEntries,
        PipelineCache, Shader, ShaderStages, SpecializedComputePipeline,
        SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice},
    Render, RenderApp, RenderSet,
};
use bevy_utils::tracing::{error, warn};

use crate::{graph::NodePbr, MeshInputUniform, MeshUniform};

/// The handle to the `build_mesh_uniforms.wgsl` compute shader.
pub const BUILD_MESH_UNIFORMS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(16991728318640779533);

/// The GPU workgroup size.
const WORKGROUP_SIZE: usize = 64;

/// A plugin that builds mesh uniforms on GPU.
///
/// This will only be added if the platform supports compute shaders (e.g. not
/// on WebGL 2).
pub struct BuildMeshUniformsPlugin;

/// The render node for the mesh uniform building pass.
#[derive(Default)]
pub struct BuildMeshUniformsNode;

/// The compute shader pipeline for the mesh uniform building pass.
#[derive(Resource)]
pub struct BuildMeshUniformsPipeline {
    /// The single bind group layout for the compute shader.
    pub bind_group_layout: BindGroupLayout,
    /// The pipeline ID for the compute shader.
    ///
    /// This gets filled in in `prepare_build_mesh_uniforms_pipeline`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

/// The compute shader bind group for the mesh uniform building pass.
#[derive(Resource, Default)]
pub struct BuildMeshUniformsBindGroup(Option<BindGroup>);

impl Plugin for BuildMeshUniformsPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            BUILD_MESH_UNIFORMS_SHADER_HANDLE,
            "build_mesh_uniforms.wgsl",
            Shader::from_wgsl
        );

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            (
                prepare_build_mesh_uniforms_pipeline.in_set(RenderSet::Prepare),
                prepare_build_mesh_uniforms_bind_group.in_set(RenderSet::PrepareBindGroups),
            ),
        );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Stitch the node in.
        render_app
            .add_render_graph_node::<ViewNodeRunner<BuildMeshUniformsNode>>(
                Core3d,
                NodePbr::BuildMeshUniforms,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::StartMainPass,
                    NodePbr::BuildMeshUniforms,
                    Node3d::MainOpaquePass,
                ),
            )
            .init_resource::<BuildMeshUniformsPipeline>()
            .init_resource::<BuildMeshUniformsBindGroup>()
            .init_resource::<SpecializedComputePipelines<BuildMeshUniformsPipeline>>();
    }
}

impl ViewNode for BuildMeshUniformsNode {
    type ViewQuery = ();

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        _: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Grab the [`BatchedInstanceBuffers`]. If we aren't using GPU mesh
        // uniform building, bail out.
        let BatchedInstanceBuffers::GpuBuilt { index_count, .. } =
            world.resource::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>()
        else {
            error!(
                "Attempted to build mesh uniforms on GPU, but `GpuBuilt` batched instance buffers \
                weren't available"
            );
            return Ok(());
        };

        // Grab the bind group.
        let Some(ref bind_group) = world.resource::<BuildMeshUniformsBindGroup>().0 else {
            error!("Attempted to build mesh uniforms on GPU, but the bind group wasn't available");
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let build_mesh_uniforms_pipeline = world.resource::<BuildMeshUniformsPipeline>();

        let Some(build_mesh_uniforms_pipeline_id) = build_mesh_uniforms_pipeline.pipeline_id else {
            warn!("The build mesh uniforms pipeline wasn't uploaded");
            return Ok(());
        };

        let Some(view_build_mesh_uniforms_pipeline) =
            pipeline_cache.get_compute_pipeline(build_mesh_uniforms_pipeline_id)
        else {
            // This will happen while the pipeline is being compiled and is fine.
            return Ok(());
        };

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("build mesh uniforms"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(view_build_mesh_uniforms_pipeline);
        compute_pass.set_bind_group(0, bind_group, &[]);
        let workgroup_count = div_round_up(*index_count, WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);

        Ok(())
    }
}

impl SpecializedComputePipeline for BuildMeshUniformsPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("build mesh uniforms".into()),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: BUILD_MESH_UNIFORMS_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
        }
    }
}

impl FromWorld for BuildMeshUniformsPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout_entries = DynamicBindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // `current_input`
                storage_buffer_read_only::<MeshInputUniform>(/*has_dynamic_offset=*/ false),
                // `previous_input`
                storage_buffer_read_only::<MeshInputUniform>(/*has_dynamic_offset=*/ false),
                // `indices`
                storage_buffer_read_only::<u32>(/*has_dynamic_offset=*/ false),
                // `output`
                storage_buffer::<MeshUniform>(/*has_dynamic_offset=*/ false),
            ),
        );

        let bind_group_layout = render_device.create_bind_group_layout(
            "build mesh uniforms bind group layout",
            &bind_group_layout_entries,
        );

        BuildMeshUniformsPipeline {
            bind_group_layout,
            pipeline_id: None,
        }
    }
}

/// A system that specializes the `build_mesh_uniforms.wgsl` pipeline if
/// necessary.
pub fn prepare_build_mesh_uniforms_pipeline(
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<BuildMeshUniformsPipeline>>,
    mut build_mesh_uniforms_pipeline: ResMut<BuildMeshUniformsPipeline>,
) {
    if build_mesh_uniforms_pipeline.pipeline_id.is_some() {
        return;
    }

    let build_mesh_uniforms_pipeline_id =
        pipelines.specialize(&pipeline_cache, &build_mesh_uniforms_pipeline, ());
    build_mesh_uniforms_pipeline.pipeline_id = Some(build_mesh_uniforms_pipeline_id);
}

/// A system that attaches the mesh uniform buffers to the bind group for the
/// compute shader.
pub fn prepare_build_mesh_uniforms_bind_group(
    render_device: Res<RenderDevice>,
    batched_instance_buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    pipeline: Res<BuildMeshUniformsPipeline>,
    mut bind_group: ResMut<BuildMeshUniformsBindGroup>,
) {
    // Grab the [`BatchedInstanceBuffers`]. If we aren't using GPU mesh
    // uniform building, bail out.
    let BatchedInstanceBuffers::GpuBuilt {
        data_buffer: ref data_buffer_vec,
        index_buffer: ref index_buffer_vec,
        current_input_buffer: ref current_input_buffer_vec,
        previous_input_buffer: ref previous_input_buffer_vec,
        index_count: _,
    } = *batched_instance_buffers
    else {
        return;
    };

    let (
        Some(current_input_buffer),
        Some(previous_input_buffer),
        Some(index_buffer),
        Some(data_buffer),
    ) = (
        current_input_buffer_vec.buffer(),
        previous_input_buffer_vec.buffer(),
        index_buffer_vec.buffer(),
        data_buffer_vec.buffer(),
    )
    else {
        return;
    };

    bind_group.0 = Some(render_device.create_bind_group(
        "build_mesh_uniforms_bind_group",
        &pipeline.bind_group_layout,
        &BindGroupEntries::sequential((
            current_input_buffer.as_entire_binding(),
            previous_input_buffer.as_entire_binding(),
            index_buffer.as_entire_binding(),
            data_buffer.as_entire_binding(),
        )),
    ));
}

/// Returns `a / b`, rounded toward positive infinity.
fn div_round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
