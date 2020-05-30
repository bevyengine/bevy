use bevy_asset::Handle;
use legion::prelude::*;

use crate::{
    draw_target::DrawTarget,
    mesh::{self, Mesh},
    pass::RenderPass,
    pipeline::{PipelineAssignments, PipelineDescriptor},
    render_resource::{
        EntitiesWaitingForAssets, EntityRenderResourceAssignments,
        RenderResourceAssignments, ResourceInfo,
    },
    renderer::RenderContext,
    Renderable,
};

#[derive(Default)]
pub struct AssignedMeshesDrawTarget;

impl AssignedMeshesDrawTarget {
    pub const NAME: &'static str = "AssignedMeshes";
}

impl DrawTarget for AssignedMeshesDrawTarget {
    fn draw(
        &self,
        world: &World,
        resources: &Resources,
        render_pass: &mut dyn RenderPass,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let shader_pipeline_assignments = resources.get::<PipelineAssignments>().unwrap();
        let entity_render_resource_assignments =
            resources.get::<EntityRenderResourceAssignments>().unwrap();
        let entities_waiting_for_assets = resources.get::<EntitiesWaitingForAssets>().unwrap();
        let mut current_mesh_handle = None;
        let mut current_mesh_index_len = 0;
        let global_render_resource_assignments =
            resources.get::<RenderResourceAssignments>().unwrap();
        render_pass.set_render_resources(pipeline_descriptor, &global_render_resource_assignments);

        let assigned_render_resource_assignments = shader_pipeline_assignments
            .assignments
            .get(&pipeline_handle);

        if let Some(assigned_render_resource_assignments) = assigned_render_resource_assignments {
            for assignment_id in assigned_render_resource_assignments.iter() {
                // TODO: hopefully legion has better random access apis that are more like queries?
                let entity = entity_render_resource_assignments
                    .get(*assignment_id)
                    .unwrap();
                let renderable = world.get_component::<Renderable>(*entity).unwrap();
                if !renderable.is_visible
                    || renderable.is_instanced
                    || entities_waiting_for_assets.contains(entity)
                {
                    continue;
                }

                let mesh_handle = *world.get_component::<Handle<Mesh>>(*entity).unwrap();
                let render_context = render_pass.get_render_context();
                let render_resources = render_context.resources();
                // TODO: this can be removed
                if current_mesh_handle != Some(mesh_handle) {
                    if let Some(vertex_buffer_resource) = render_resources
                        .get_asset_resource(mesh_handle, mesh::VERTEX_BUFFER_ASSET_INDEX)
                    {
                        let index_buffer_resource = render_resources
                            .get_asset_resource(mesh_handle, mesh::INDEX_BUFFER_ASSET_INDEX)
                            .unwrap();
                        render_resources.get_resource_info(
                            index_buffer_resource,
                            &mut |resource_info| match resource_info {
                                Some(ResourceInfo::Buffer(buffer_info)) => {
                                    current_mesh_index_len = (buffer_info.size / 2) as u32
                                }
                                _ => panic!("expected a buffer type"),
                            },
                        );
                        render_pass.set_index_buffer(index_buffer_resource, 0);
                        render_pass.set_vertex_buffer(0, vertex_buffer_resource, 0);
                    }
                    // TODO: Verify buffer format matches render pass
                    current_mesh_handle = Some(mesh_handle);
                }

                // TODO: validate bind group properties against shader uniform properties at least once
                render_pass.set_render_resources(
                    pipeline_descriptor,
                    &renderable.render_resource_assignments,
                );

                render_pass.draw_indexed(0..current_mesh_index_len, 0, 0..1);
            }
        }
    }

    fn setup(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let pipeline_assignments = resources.get::<PipelineAssignments>().unwrap();
        let entity_render_resource_assignments =
            resources.get::<EntityRenderResourceAssignments>().unwrap();
        let assigned_render_resource_assignments =
            pipeline_assignments.assignments.get(&pipeline_handle);
        let global_render_resource_assignments =
            resources.get::<RenderResourceAssignments>().unwrap();
        render_context
            .resources()
            .setup_bind_groups(pipeline_descriptor, &global_render_resource_assignments);
        if let Some(assigned_render_resource_assignments) = assigned_render_resource_assignments {
            for assignment_id in assigned_render_resource_assignments.iter() {
                let entity = entity_render_resource_assignments
                    .get(*assignment_id)
                    .unwrap();
                let renderable = world.get_component::<Renderable>(*entity).unwrap();
                if !renderable.is_visible || renderable.is_instanced {
                    continue;
                }

                render_context.resources().setup_bind_groups(
                    pipeline_descriptor,
                    &renderable.render_resource_assignments,
                );
            }
        }
    }

    fn get_name(&self) -> String {
        AssignedMeshesDrawTarget::NAME.to_string()
    }
}
