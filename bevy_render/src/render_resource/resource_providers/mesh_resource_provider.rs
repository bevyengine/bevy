use crate::{
    mesh::{self, Mesh},
    pipeline::VertexBufferDescriptors,
    render_resource::{AssetBatchers, BufferInfo, BufferUsage},
    renderer_2::GlobalRenderResourceContext,
    shader::AsUniforms,
    Vertex,
};
use bevy_asset::AssetStorage;
use legion::prelude::*;
use zerocopy::AsBytes;

pub fn mesh_resource_provider_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();
    vertex_buffer_descriptors.set(Vertex::get_vertex_buffer_descriptor().cloned().unwrap());
    SystemBuilder::new("mesh_resource_provider")
        .read_resource::<GlobalRenderResourceContext>()
        .read_resource::<AssetStorage<Mesh>>()
        .write_resource::<AssetBatchers>()
        .build(
            |_, _, (render_resource_context, meshes, asset_batchers), _| {
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
                            let vertex_buffer = render_resources.create_buffer_with_data(
                                BufferInfo {
                                    buffer_usage: BufferUsage::VERTEX,
                                    ..Default::default()
                                },
                                mesh_asset.vertices.as_bytes(),
                            );
                            let index_buffer = render_resources.create_buffer_with_data(
                                BufferInfo {
                                    buffer_usage: BufferUsage::INDEX,
                                    ..Default::default()
                                },
                                mesh_asset.indices.as_bytes(),
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
