//! GPU culling.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    schedule::IntoSystemConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    indirect::{
        GpuIndirectInstanceDescriptor, GpuIndirectParameters, IndirectBuffers, MeshIndirectUniform,
    },
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only, uniform_buffer},
        BindGroupEntries, BindGroupLayout, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, DynamicBindGroupLayoutEntries, GpuArrayBuffer, PipelineCache,
        Shader, ShaderStages, SpecializedComputePipeline, SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice},
    view::{GpuCulling, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use bevy_utils::tracing::warn;

use crate::{graph::NodePbr, MeshUniform};

const WORKGROUP_SIZE: usize = 64;

pub const CULL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10372890860177113771);

pub struct GpuCullPlugin;

#[derive(Default)]
pub struct GpuCullNode;

#[derive(Resource)]
pub struct CullingPipeline {
    culling_bind_group_layout: BindGroupLayout,
}

#[derive(Component)]
pub struct ViewCullingPipeline(CachedComputePipelineId);

impl Plugin for GpuCullPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, CULL_SHADER_HANDLE, "cull.wgsl", Shader::from_wgsl);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(Render, prepare_culling_pipelines.in_set(RenderSet::Prepare));
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<GpuCullNode>>(Core3d, NodePbr::GpuCull)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::StartMainPass,
                    NodePbr::GpuCull,
                    Node3d::MainOpaquePass,
                ),
            )
            .init_resource::<CullingPipeline>()
            .init_resource::<SpecializedComputePipelines<CullingPipeline>>();
    }
}

impl ViewNode for GpuCullNode {
    type ViewQuery = (Entity, Read<ViewUniformOffset>, Read<ViewCullingPipeline>);

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_entity, view_uniform_offset, view_culling_pipeline): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let culling_pipeline = world.resource::<CullingPipeline>();
        let indirect_buffers = world.resource::<IndirectBuffers>();
        let mesh_uniforms_buffer = world.resource::<GpuArrayBuffer<MeshUniform>>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let Some(view_culling_pipeline) =
            pipeline_cache.get_compute_pipeline(view_culling_pipeline.0)
        else {
            warn!("View culling pipeline wasn't present");
            return Ok(());
        };

        let Some(view_instance_buffers) = indirect_buffers.view_instances.get(&view_entity) else {
            warn!("Failed to find view instance buffers");
            return Ok(());
        };

        let Some(view_uniforms_buffer) = view_uniforms.uniforms.binding() else {
            warn!("View uniforms buffer wasn't uploaded");
            return Ok(());
        };

        let Some(mesh_uniforms_buffer) = mesh_uniforms_buffer.binding() else {
            warn!("Mesh uniforms buffer wasn't uploaded");
            return Ok(());
        };

        let Some(mesh_indirect_uniforms_buffer) = indirect_buffers.mesh_indirect_uniform.buffer()
        else {
            warn!("Failed to find mesh indirect uniforms buffer");
            return Ok(());
        };

        let Some(indirect_parameters_buffer) = indirect_buffers.params.buffer() else {
            warn!("Failed to find indirect parameters buffer");
            return Ok(());
        };

        let (Some(instances_buffer), Some(descriptors_buffer)) = (
            view_instance_buffers.instances.buffer(),
            view_instance_buffers.descriptors.buffer(),
        ) else {
            warn!("View instance buffers weren't uploaded");
            return Ok(());
        };

        // TODO: Do this in a separate pass and cache them.
        let cull_bind_group = render_context.render_device().create_bind_group(
            "cull_bind_group",
            &culling_pipeline.culling_bind_group_layout,
            &BindGroupEntries::sequential((
                view_uniforms_buffer,
                mesh_uniforms_buffer,
                instances_buffer.as_entire_binding(),
                descriptors_buffer.as_entire_binding(),
                mesh_indirect_uniforms_buffer.as_entire_binding(),
                indirect_parameters_buffer.as_entire_binding(),
            )),
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("cull"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(view_culling_pipeline);
        compute_pass.set_bind_group(0, &cull_bind_group, &[view_uniform_offset.offset]);
        let workgroup_count = div_round_up(view_instance_buffers.descriptors.len(), WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);

        Ok(())
    }
}

impl SpecializedComputePipeline for CullingPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("cull".into()),
            layout: vec![self.culling_bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: CULL_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
        }
    }
}

impl FromWorld for CullingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let culling_bind_group_layout_entries = DynamicBindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<ViewUniform>(/*has_dynamic_offset=*/ true),
                GpuArrayBuffer::<MeshUniform>::binding_layout(render_device),
                storage_buffer::<u32>(/*has_dynamic_offset=*/ false),
                storage_buffer_read_only::<GpuIndirectInstanceDescriptor>(
                    /*has_dynamic_offset=*/ false,
                ),
                storage_buffer_read_only::<MeshIndirectUniform>(/*has_dynamic_offset=*/ false),
                storage_buffer::<GpuIndirectParameters>(/*has_dynamic_offset=*/ false),
            ),
        );

        let culling_bind_group_layout = render_device
            .create_bind_group_layout("cull_bind_group_layout", &culling_bind_group_layout_entries);

        CullingPipeline {
            culling_bind_group_layout,
        }
    }
}

pub fn prepare_culling_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<CullingPipeline>>,
    culling_pipeline: Res<CullingPipeline>,
    view_query: Query<Entity, (With<ViewTarget>, With<GpuCulling>)>,
) {
    for entity in view_query.iter() {
        let culling_pipeline_id = pipelines.specialize(&pipeline_cache, &culling_pipeline, ());
        commands
            .entity(entity)
            .insert(ViewCullingPipeline(culling_pipeline_id));
    }
}

fn div_round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
