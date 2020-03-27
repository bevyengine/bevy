use crate::{
    asset::{AssetStorage, Handle},
    legion::prelude::*,
    render::{
        draw_target::DrawTarget,
        mesh::Mesh,
        pipeline::{PipelineDescriptor, ShaderPipelineAssignments},
        render_resource::{
            resource_name, EntityRenderResourceAssignments, RenderResourceAssignments, ResourceInfo,
        },
        renderer::{RenderPass, Renderer},
        Renderable,
    },
};

#[derive(Default)]
pub struct AssignedMeshesDrawTarget;

impl DrawTarget for AssignedMeshesDrawTarget {
    fn draw(
        &self,
        world: &World,
        resources: &Resources,
        render_pass: &mut dyn RenderPass,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) {
        let shader_pipeline_assignments = resources.get::<ShaderPipelineAssignments>().unwrap();
        let entity_render_resource_assignments =
            resources.get::<EntityRenderResourceAssignments>().unwrap();
        let mut current_mesh_handle = None;
        let mut current_mesh_index_len = 0;
        let global_render_resource_assignments =
            resources.get::<RenderResourceAssignments>().unwrap();
        render_pass.set_render_resources(&global_render_resource_assignments);

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
                if !renderable.is_visible || renderable.is_instanced {
                    continue;
                }

                let mesh = *world.get_component::<Handle<Mesh>>(*entity).unwrap();
                let renderer = render_pass.get_renderer();
                let render_resources = renderer.get_render_resources();
                if current_mesh_handle != Some(mesh) {
                    if let Some(vertex_buffer_resource) =
                        render_resources.get_mesh_vertices_resource(mesh)
                    {
                        let index_buffer_resource =
                            render_resources.get_mesh_indices_resource(mesh).unwrap();
                        match renderer.get_resource_info(index_buffer_resource).unwrap() {
                            ResourceInfo::Buffer(buffer_info) => {
                                current_mesh_index_len = (buffer_info.size / 2) as u32
                            }
                            _ => panic!("expected a buffer type"),
                        }
                        render_pass.set_index_buffer(index_buffer_resource, 0);
                        render_pass.set_vertex_buffer(0, vertex_buffer_resource, 0);
                    }
                    // TODO: Verify buffer format matches render pass
                    current_mesh_handle = Some(mesh);
                }

                // TODO: validate bind group properties against shader uniform properties at least once
                render_pass.set_render_resources(&renderable.render_resource_assignments);
                render_pass.draw_indexed(0..current_mesh_index_len, 0, 0..1);
            }
        }
    }

    fn setup(
        &mut self,
        world: &mut World,
        resources: &Resources,
        renderer: &mut dyn Renderer,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) {
        let shader_pipeline_assignments = resources.get::<ShaderPipelineAssignments>().unwrap();
        let entity_render_resource_assignments =
            resources.get::<EntityRenderResourceAssignments>().unwrap();
        let assigned_render_resource_assignments = shader_pipeline_assignments
            .assignments
            .get(&pipeline_handle);
        let pipeline_storage = resources.get::<AssetStorage<PipelineDescriptor>>().unwrap();
        let pipeline_descriptor = pipeline_storage.get(&pipeline_handle).unwrap();
        let mut global_render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();
        renderer.setup_bind_groups(&mut global_render_resource_assignments, pipeline_descriptor);
        if let Some(assigned_render_resource_assignments) = assigned_render_resource_assignments {
            for assignment_id in assigned_render_resource_assignments.iter() {
                // TODO: hopefully legion has better random access apis that are more like queries?
                let entity = entity_render_resource_assignments
                    .get(*assignment_id)
                    .unwrap();
                let mut renderable = world.get_component_mut::<Renderable>(*entity).unwrap();
                if !renderable.is_visible || renderable.is_instanced {
                    continue;
                }

                renderer.setup_bind_groups(
                    &mut renderable.render_resource_assignments,
                    pipeline_descriptor,
                );
            }
        }
    }

    fn get_name(&self) -> String {
        resource_name::draw_target::ASSIGNED_MESHES.to_string()
    }
}
