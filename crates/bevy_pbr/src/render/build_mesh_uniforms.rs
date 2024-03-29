//! Build mesh uniforms.

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
        BindGroupEntries, BindGroupLayout, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, DynamicBindGroupLayoutEntries, PipelineCache, Shader,
        ShaderStages, SpecializedComputePipeline, SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice},
    Render, RenderApp, RenderSet,
};
use bevy_utils::tracing::warn;

use crate::{graph::NodePbr, MeshInputUniform, MeshUniform};

pub const BUILD_MESH_UNIFORMS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(16991728318640779533);

const WORKGROUP_SIZE: usize = 64;

pub struct BuildMeshUniformsPlugin;

#[derive(Default)]
pub struct BuildMeshUniformsNode;

#[derive(Resource)]
pub struct BuildMeshUniformsPipeline {
    pub bind_group_layout: BindGroupLayout,
    /// This gets filled in in `prepare_build_mesh_uniforms_pipeline`.
    pub pipeline_id: Option<CachedComputePipelineId>,
}

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
            prepare_build_mesh_uniforms_pipeline.in_set(RenderSet::Prepare),
        );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

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
        let BatchedInstanceBuffers::GpuBuilt {
            data_buffer: ref data_buffer_vec,
            index_buffer: ref index_buffer_vec,
            current_input_buffer: ref current_input_buffer_vec,
            previous_input_buffer: ref previous_input_buffer_vec,
            index_count,
        } = world.resource::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>()
        else {
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
            warn!("The view build mesh uniforms pipeline wasn't present in the pipeline cache");
            return Ok(());
        };

        let Some(current_input_buffer) = current_input_buffer_vec.buffer() else {
            warn!("The current input buffer wasn't uploaded");
            return Ok(());
        };
        let Some(previous_input_buffer) = previous_input_buffer_vec.buffer() else {
            warn!("The previous input buffer wasn't uploaded");
            return Ok(());
        };
        let Some(index_buffer) = index_buffer_vec.buffer() else {
            warn!("The index buffer wasn't uploaded");
            return Ok(());
        };
        let Some(data_buffer) = data_buffer_vec.buffer() else {
            warn!("The data buffer wasn't uploaded");
            return Ok(());
        };

        // TODO: Do this in a separate system and cache it.
        let bind_group = render_context.render_device().create_bind_group(
            "build_mesh_uniforms_bind_group",
            &build_mesh_uniforms_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                current_input_buffer.as_entire_binding(),
                previous_input_buffer.as_entire_binding(),
                index_buffer.as_entire_binding(),
                data_buffer.as_entire_binding(),
            )),
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("build mesh uniforms"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(view_build_mesh_uniforms_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
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
                storage_buffer_read_only::<MeshInputUniform>(/*has_dynamic_offset=*/ false),
                storage_buffer_read_only::<MeshInputUniform>(/*has_dynamic_offset=*/ false),
                storage_buffer_read_only::<u32>(/*has_dynamic_offset=*/ false),
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

fn div_round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
