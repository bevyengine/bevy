use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::{
        render_graph_2::{RenderPass, ShaderUniforms},
        Instanced,
    },
};

use super::resource_name;
use super::ResourceInfo;
use zerocopy::AsBytes;

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc

// TODO: consider swapping out dyn RenderPass for explicit WgpuRenderPass type to avoid dynamic dispatch
pub type DrawTarget = fn(world: &World, render_pass: &mut dyn RenderPass);

const MESH_VERTEX_BUFFER_NAME: &str = "TempMeshVertexBuffer";
const MESH_INDEX_BUFFER_NAME: &str = "TempMeshIndexBuffer";
pub fn mesh_draw_target(world: &World, render_pass: &mut dyn RenderPass) {
    let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut current_mesh_id = None;
    let mut current_mesh_index_length = 0;
    let mesh_query =
        <(Read<ShaderUniforms>, Read<Handle<Mesh>>)>::query().filter(!component::<Instanced>());
    for (entity, (_shader_uniforms, mesh)) in mesh_query.iter_entities(world) {
        let mut should_load_mesh = current_mesh_id == None;
        if let Some(current) = current_mesh_id {
            should_load_mesh = current != mesh.id;
        }

        if should_load_mesh {
            if let Some(mesh_asset) = mesh_storage.get(mesh.id) {
                let renderer = render_pass.get_renderer();
                renderer.create_buffer_with_data(
                    MESH_VERTEX_BUFFER_NAME,
                    mesh_asset.vertices.as_bytes(),
                    wgpu::BufferUsage::VERTEX,
                );
                renderer.create_buffer_with_data(
                    MESH_INDEX_BUFFER_NAME,
                    mesh_asset.indices.as_bytes(),
                    wgpu::BufferUsage::INDEX,
                );

                // TODO: Verify buffer format matches render pass
                render_pass.set_index_buffer(MESH_INDEX_BUFFER_NAME, 0);
                render_pass.set_vertex_buffer(0, MESH_VERTEX_BUFFER_NAME, 0);
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
    renderer.remove_buffer(MESH_VERTEX_BUFFER_NAME);
    renderer.remove_buffer(MESH_INDEX_BUFFER_NAME);
}

pub fn ui_draw_target(world: &World, render_pass: &mut dyn RenderPass) {
    let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    // NOTE: this is ugly and borrowing is stupid
    let result = {
        let renderer = render_pass.get_renderer();
        let result = if let Some(ResourceInfo::InstanceBuffer { count, mesh_id, .. }) =
            renderer.get_resource_info(resource_name::buffer::UI_INSTANCES)
        {
            Some((*count, *mesh_id))
        } else {
            None
        };

        if let Some((instance_count, mesh_id)) = result {
            if let Some(mesh_asset) = mesh_storage.get(mesh_id) {
                renderer.create_buffer_with_data(
                    MESH_VERTEX_BUFFER_NAME,
                    mesh_asset.vertices.as_bytes(),
                    wgpu::BufferUsage::VERTEX,
                );
                renderer.create_buffer_with_data(
                    MESH_INDEX_BUFFER_NAME,
                    mesh_asset.indices.as_bytes(),
                    wgpu::BufferUsage::INDEX,
                );
                Some((instance_count, mesh_asset.indices.len()))
            } else {
                None
            }
        } else {
            None
        }

    };
    if let Some((instance_count, indices_length)) = result {
        render_pass.setup_bind_groups(None);
        render_pass.set_index_buffer(MESH_INDEX_BUFFER_NAME, 0);
        render_pass.set_vertex_buffer(0, MESH_VERTEX_BUFFER_NAME, 0);
        render_pass.set_vertex_buffer(1, resource_name::buffer::UI_INSTANCES, 0);
        render_pass.draw_indexed(0..indices_length as u32, 0, 0..(instance_count as u32));
    }
}
