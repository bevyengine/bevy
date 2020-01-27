use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::{
        render_graph_2::{ShaderUniforms, RenderPass},
        Instanced,
    },
};

use zerocopy::AsBytes;

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc
pub type DrawTarget = fn(world: &World, render_pass: &mut dyn RenderPass);

const MESH_VERTEX_BUFFER_NAME: &str = "TempMeshVertexBuffer";
const MESH_INDEX_BUFFER_NAME: &str = "TempMeshIndexBuffer";
pub fn mesh_draw_target(world: &World, render_pass: &mut dyn RenderPass) {
    let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut last_mesh_id = None;
    let mesh_query =
        <(Read<ShaderUniforms>, Read<Handle<Mesh>>)>::query().filter(!component::<Instanced>());
    for (shader_uniforms, mesh) in mesh_query.iter(world) {
        let current_mesh_id = mesh.id;

        let mut should_load_mesh = last_mesh_id == None;
        if let Some(last) = last_mesh_id {
            should_load_mesh = last != current_mesh_id;
        }

        if should_load_mesh {
            if let Some(mesh_asset) = mesh_storage.get(mesh.id) {
                let renderer = render_pass.get_renderer();
                renderer.create_buffer_with_data(MESH_VERTEX_BUFFER_NAME, mesh_asset.vertices.as_bytes(), wgpu::BufferUsage::VERTEX);
                renderer.create_buffer_with_data(MESH_INDEX_BUFFER_NAME, mesh_asset.indices.as_bytes(), wgpu::BufferUsage::INDEX);
                
                // TODO: Verify buffer format matches render pass
                render_pass.set_index_buffer(MESH_INDEX_BUFFER_NAME, 0);
                render_pass.set_vertex_buffer(0, MESH_VERTEX_BUFFER_NAME, 0);
            };
        }

        // TODO: re-getting the mesh isn't necessary. just store the index count
        if let Some(mesh_asset) = mesh_storage.get(mesh.id) {
            // TODO: validate bind group properties against shader uniform properties at least once
            render_pass.setup_bind_groups(&&*shader_uniforms);
            render_pass.draw_indexed(0..mesh_asset.indices.len() as u32, 0, 0..1);
        };

        last_mesh_id = Some(current_mesh_id);
    }
    
    // cleanup buffers
    let renderer = render_pass.get_renderer();
    renderer.remove_buffer(MESH_VERTEX_BUFFER_NAME);
    renderer.remove_buffer(MESH_INDEX_BUFFER_NAME);
}