use crate::{
    asset::{Handle, Mesh},
    legion::prelude::*,
    render::render_graph::{PipelineDescriptor, RenderPass, Renderable, ShaderPipelineAssignments, ResourceInfo},
};

pub fn assigned_meshes_draw_target(
    world: &World,
    render_pass: &mut dyn RenderPass,
    pipeline_handle: Handle<PipelineDescriptor>,
) {
    let shader_pipeline_assignments = world
        .resources
        .get_mut::<ShaderPipelineAssignments>()
        .unwrap();
    let mut current_mesh_handle = None;
    let mut current_mesh_index_len = 0;

    let assigned_entities = shader_pipeline_assignments
        .assignments
        .get(&pipeline_handle);

    if let Some(assigned_entities) = assigned_entities {
        for entity in assigned_entities.iter() {
            // TODO: hopefully legion has better random access apis that are more like queries?
            let renderable = world.get_component::<Renderable>(*entity).unwrap();
            let mesh = *world.get_component::<Handle<Mesh>>(*entity).unwrap();
            if !renderable.is_visible {
                continue;
            }

            let renderer = render_pass.get_renderer();
            let render_resources = renderer.get_render_resources();
            if current_mesh_handle != Some(mesh) {
                if let Some(vertex_buffer_resource) = render_resources.get_mesh_vertices_resource(mesh) {
                    let index_buffer_resource = render_resources.get_mesh_indices_resource(mesh).unwrap();
                    match renderer.get_resource_info(index_buffer_resource).unwrap() {
                        ResourceInfo::Buffer { size, ..} => current_mesh_index_len = (size / 2) as u32,
                        _ => panic!("expected a buffer type"),
                    }
                    render_pass.set_index_buffer(index_buffer_resource, 0);
                    render_pass.set_vertex_buffer(0, vertex_buffer_resource, 0);
                }
                // TODO: Verify buffer format matches render pass
                current_mesh_handle = Some(mesh);
            }

            // TODO: validate bind group properties against shader uniform properties at least once
            render_pass.setup_bind_groups(Some(&entity));
            render_pass.draw_indexed(0..current_mesh_index_len, 0, 0..1);
        }
    }
}
