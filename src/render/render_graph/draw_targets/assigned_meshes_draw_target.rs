use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::render_graph::{
        resource_name, PipelineDescriptor, RenderPass, Renderable, ShaderPipelineAssignments,
    },
};

use zerocopy::AsBytes;

pub fn assigned_meshes_draw_target(
    world: &World,
    render_pass: &mut dyn RenderPass,
    pipeline_handle: Handle<PipelineDescriptor>,
) {
    let mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let shader_pipeline_assignments = world
        .resources
        .get_mut::<ShaderPipelineAssignments>()
        .unwrap();
    let mut current_mesh_id = None;
    let mut current_mesh_index_length = 0;

    let assigned_entities = shader_pipeline_assignments
        .assignments
        .get(&pipeline_handle);

    if let Some(assigned_entities) = assigned_entities {
        for entity in assigned_entities.iter() {
            // TODO: hopefully legion has better random access apis that are more like queries?
            let renderable = world.get_component::<Renderable>(*entity).unwrap();
            let mesh = world.get_component::<Handle<Mesh>>(*entity).unwrap();
            if !renderable.is_visible {
                continue;
            }

            let mut should_load_mesh = current_mesh_id == None;
            if let Some(current) = current_mesh_id {
                should_load_mesh = current != mesh.id;
            }

            if should_load_mesh {
                if let Some(mesh_asset) = mesh_storage.get_id(mesh.id) {
                    let renderer = render_pass.get_renderer();
                    renderer.create_buffer_with_data(
                        resource_name::buffer::TEMP_MESH_VERTEX_BUFFER_NAME,
                        mesh_asset.vertices.as_bytes(),
                        wgpu::BufferUsage::VERTEX,
                    );
                    renderer.create_buffer_with_data(
                        resource_name::buffer::TEMP_MESH_INDEX_BUFFER_NAME,
                        mesh_asset.indices.as_bytes(),
                        wgpu::BufferUsage::INDEX,
                    );

                    // TODO: Verify buffer format matches render pass
                    render_pass
                        .set_index_buffer(resource_name::buffer::TEMP_MESH_INDEX_BUFFER_NAME, 0);
                    render_pass.set_vertex_buffer(
                        0,
                        resource_name::buffer::TEMP_MESH_VERTEX_BUFFER_NAME,
                        0,
                    );
                    current_mesh_id = Some(mesh.id);
                    current_mesh_index_length = mesh_asset.indices.len() as u32;
                };
            }

            // TODO: validate bind group properties against shader uniform properties at least once
            render_pass.setup_bind_groups(Some(&entity));
            render_pass.draw_indexed(0..current_mesh_index_length, 0, 0..1);
        }

        // cleanup buffers
        let renderer = render_pass.get_renderer();
        renderer.remove_buffer(resource_name::buffer::TEMP_MESH_VERTEX_BUFFER_NAME);
        renderer.remove_buffer(resource_name::buffer::TEMP_MESH_INDEX_BUFFER_NAME);
    }
}
