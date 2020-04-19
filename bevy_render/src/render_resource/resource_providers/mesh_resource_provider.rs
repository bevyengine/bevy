use crate::{
    mesh::{self, Mesh},
    pipeline::{state_descriptors::IndexFormat, VertexBufferDescriptors},
    render_resource::{AssetBatchers, BufferInfo, BufferUsage},
    renderer_2::GlobalRenderResourceContext,
    shader::AsUniforms,
    Vertex,
};
use bevy_asset::AssetStorage;
use legion::prelude::*;

pub fn mesh_resource_provider_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();
    // TODO: allow pipelines to specialize on vertex_buffer_descriptor and index_format
    let vertex_buffer_descriptor = Vertex::get_vertex_buffer_descriptor().unwrap();
    let index_format = IndexFormat::Uint16;
    vertex_buffer_descriptors.set(vertex_buffer_descriptor.clone());
    SystemBuilder::new("mesh_resource_provider")
        .read_resource::<GlobalRenderResourceContext>()
        .read_resource::<AssetStorage<Mesh>>()
        .write_resource::<AssetBatchers>()
        .build(
            move |_, _, (render_resource_context, meshes, asset_batchers), _| {
                let render_resources = &render_resource_context.context;
                if let Some(batches) = asset_batchers.get_handle_batches_mut::<Mesh>() {
                    for batch in batches {
                        let handle = batch.get_handle::<Mesh>().unwrap();
                        log::trace!("setup mesh for {:?}", batch.render_resource_assignments.id);
                        let (vertex_buffer, index_buffer) = if let Some(vertex_buffer) =
                            render_resources
                                .get_asset_resource(handle, mesh::VERTEX_BUFFER_ASSET_INDEX)
                        {
                            (
                                vertex_buffer,
                                render_resources
                                    .get_asset_resource(handle, mesh::INDEX_BUFFER_ASSET_INDEX),
                            )
                        } else {
                            let mesh_asset = meshes.get(&handle).unwrap();
                            let vertex_bytes = mesh_asset.get_vertex_buffer_bytes(&vertex_buffer_descriptor).unwrap();
                            // TODO: use a staging buffer here
                            let vertex_buffer = render_resources.create_buffer_with_data(
                                BufferInfo {
                                    buffer_usage: BufferUsage::VERTEX,
                                    ..Default::default()
                                },
                                &vertex_bytes,
                            );
                            let index_bytes = mesh_asset.get_index_buffer_bytes(index_format).unwrap();
                            let index_buffer = render_resources.create_buffer_with_data(
                                BufferInfo {
                                    buffer_usage: BufferUsage::INDEX,
                                    ..Default::default()
                                },
                                &index_bytes,
                            );

                            render_resources.set_asset_resource(
                                handle,
                                vertex_buffer,
                                mesh::VERTEX_BUFFER_ASSET_INDEX,
                            );
                            render_resources.set_asset_resource(
                                handle,
                                index_buffer,
                                mesh::INDEX_BUFFER_ASSET_INDEX,
                            );
                            (vertex_buffer, Some(index_buffer))
                        };

                        batch.render_resource_assignments.set_vertex_buffer(
                            "Vertex",
                            vertex_buffer,
                            index_buffer,
                        );
                    }
                }
            },
        )
}
