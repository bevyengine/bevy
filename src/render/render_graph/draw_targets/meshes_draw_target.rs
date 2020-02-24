use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::{
        render_graph::{PipelineDescriptor, RenderPass, Renderable},
        Instanced,
    },
};

use zerocopy::AsBytes;

pub fn meshes_draw_target(
    world: &World,
    render_pass: &mut dyn RenderPass,
    _pipeline_handle: Handle<PipelineDescriptor>,
) {
    let mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut current_mesh_id = None;
    let mut current_mesh_vertex_buffer = None;
    let mut current_mesh_index_buffer = None;
    let mut current_mesh_index_length = 0;
    let mesh_query =
        <(Read<Handle<Mesh>>, Read<Renderable>)>::query().filter(!component::<Instanced>());

    for (entity, (mesh, renderable)) in mesh_query.iter_entities(world) {
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
                if let Some(buffer) = current_mesh_vertex_buffer {
                    renderer.remove_buffer(buffer);
                }

                if let Some(buffer) = current_mesh_index_buffer {
                    renderer.remove_buffer(buffer);
                }
                current_mesh_vertex_buffer = Some(renderer.create_buffer_with_data(
                    mesh_asset.vertices.as_bytes(),
                    wgpu::BufferUsage::VERTEX,
                ));
                current_mesh_index_buffer = Some(renderer.create_buffer_with_data(
                    mesh_asset.indices.as_bytes(),
                    wgpu::BufferUsage::INDEX,
                ));

                // TODO: Verify buffer format matches render pass
                render_pass.set_index_buffer(current_mesh_index_buffer.unwrap(), 0);
                render_pass.set_vertex_buffer(0, current_mesh_vertex_buffer.unwrap(), 0);
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

    if let Some(buffer) = current_mesh_vertex_buffer {
        renderer.remove_buffer(buffer);
    }

    if let Some(buffer) = current_mesh_index_buffer {
        renderer.remove_buffer(buffer);
    }
}
