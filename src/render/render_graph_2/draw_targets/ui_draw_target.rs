use crate::{
    asset::{AssetStorage, Mesh},
    legion::prelude::*,
    render::{
        render_graph_2::{RenderPass, resource_name, ResourceInfo},
    },
};

use zerocopy::AsBytes;

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
                    resource_name::buffer::TEMP_MESH_VERTEX_BUFFER_NAME,
                    mesh_asset.vertices.as_bytes(),
                    wgpu::BufferUsage::VERTEX,
                );
                renderer.create_buffer_with_data(
                    resource_name::buffer::TEMP_MESH_INDEX_BUFFER_NAME,
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
        render_pass.set_index_buffer(resource_name::buffer::TEMP_MESH_INDEX_BUFFER_NAME, 0);
        render_pass.set_vertex_buffer(0, resource_name::buffer::TEMP_MESH_VERTEX_BUFFER_NAME, 0);
        render_pass.set_vertex_buffer(1, resource_name::buffer::UI_INSTANCES, 0);
        render_pass.draw_indexed(0..indices_length as u32, 0, 0..(instance_count as u32));
    }
}