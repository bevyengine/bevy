use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::render_graph::{resource_name, PipelineDescriptor, RenderPass, ResourceInfo},
};

use zerocopy::AsBytes;

pub fn ui_draw_target(
    world: &World,
    render_pass: &mut dyn RenderPass,
    _pipeline_handle: Handle<PipelineDescriptor>,
) {
    // TODO: re-add support for this
    // let mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    // let mut current_mesh_vertex_buffer = None;
    // let mut current_mesh_index_buffer = None;
    // let ui_instances_buffer = {
    //     let renderer = render_pass.get_renderer();
    //     match renderer.get_render_resources().get_named_resource(resource_name::buffer::UI_INSTANCES) {
    //         Some(buffer) => buffer,
    //         None => return,
    //     }
    // };
    // // NOTE: this is ugly and borrowing is stupid
    // let result = {
    //     let renderer = render_pass.get_renderer();
    //     let result = if let Some(ResourceInfo::InstanceBuffer { count, mesh_id, .. }) =
    //         renderer.get_resource_info(ui_instances_buffer)
    //     {
    //         Some((*count, *mesh_id))
    //     } else {
    //         None
    //     };

    //     if let Some((instance_count, mesh_id)) = result {
    //         if let Some(mesh_asset) = mesh_storage.get_id(mesh_id) {
    //             if let Some(buffer) = current_mesh_vertex_buffer {
    //                 renderer.remove_buffer(buffer);
    //             }

    //             if let Some(buffer) = current_mesh_index_buffer {
    //                 renderer.remove_buffer(buffer);
    //             }
    //             current_mesh_vertex_buffer = Some(renderer.create_buffer_with_data(
    //                 mesh_asset.vertices.as_bytes(),
    //                 wgpu::BufferUsage::VERTEX,
    //             ));
    //             current_mesh_index_buffer = Some(renderer.create_buffer_with_data(
    //                 mesh_asset.indices.as_bytes(),
    //                 wgpu::BufferUsage::INDEX,
    //             ));
    //             Some((instance_count, mesh_asset.indices.len()))
    //         } else {
    //             None
    //         }
    //     } else {
    //         None
    //     }
    // };
    // if let Some((instance_count, indices_length)) = result {
    //     render_pass.setup_bind_groups(None);
    //     render_pass.set_index_buffer(current_mesh_index_buffer.unwrap(), 0);
    //     render_pass.set_vertex_buffer(0, current_mesh_vertex_buffer.unwrap(), 0);
    //     render_pass.set_vertex_buffer(1, ui_instances_buffer, 0);
    //     render_pass.draw_indexed(0..indices_length as u32, 0, 0..(instance_count as u32));
    // }

    // let renderer = render_pass.get_renderer();
    // if let Some(buffer) = current_mesh_vertex_buffer {
    //     renderer.remove_buffer(buffer);
    // }

    // if let Some(buffer) = current_mesh_index_buffer {
    //     renderer.remove_buffer(buffer);
    // }
}
