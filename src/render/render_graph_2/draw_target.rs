use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::{
        render_graph_2::{ShaderMaterials, RenderPass},
        Instanced,
    },
};

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc
pub type DrawTarget = fn(world: &World, render_pass: &mut dyn RenderPass);

pub fn mesh_draw_target(world: &World, render_pass: &mut dyn RenderPass) {
    let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut last_mesh_id = None;
    let mesh_query =
        <(Read<ShaderMaterials>, Read<Handle<Mesh>>)>::query().filter(!component::<Instanced>());
    for (material, mesh) in mesh_query.iter(world) {
        let current_mesh_id = mesh.id;

        let mut should_load_mesh = last_mesh_id == None;
        if let Some(last) = last_mesh_id {
            should_load_mesh = last != current_mesh_id;
        }

        if should_load_mesh {
            if let Some(mesh_asset) = mesh_storage.get(mesh.id) {
                // render_pass.load_mesh(mesh.id, mesh_asset);
                // render_pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                // render_pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
            };
        }

        if let Some(ref mesh_asset) = mesh_storage.get(mesh.id) {
            // pass.set_bind_group(1, material.bind_group.as_ref().unwrap(), &[]);
            // pass.draw_indexed(0..mesh_asset.indices.len() as u32, 0, 0..1);
        };

        last_mesh_id = Some(current_mesh_id);
    }
}