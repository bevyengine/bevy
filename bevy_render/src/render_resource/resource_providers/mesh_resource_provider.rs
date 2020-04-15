use crate::{
    mesh::{self, Mesh},
    pipeline::VertexBufferDescriptors,
    render_resource::{
        AssetBatchers, BufferInfo, BufferUsage, RenderResourceAssignments, ResourceProvider,
    },
    renderer_2::RenderContext,
    shader::AsUniforms,
    Vertex,
};
use bevy_asset::{AssetStorage, Handle};
use legion::prelude::*;
use zerocopy::AsBytes;

#[derive(Default)]
pub struct MeshResourceProvider;

impl MeshResourceProvider {
    fn setup_mesh_resources(
        render_context: &mut dyn RenderContext,
        mesh_storage: &AssetStorage<Mesh>,
        handle: Handle<Mesh>,
        render_resource_assignments: &mut RenderResourceAssignments,
    ) {
        let render_resources = render_context.resources_mut();
        let (vertex_buffer, index_buffer) =
            if let Some(vertex_buffer) = render_resources.get_asset_resource(handle, mesh::VERTEX_BUFFER_ASSET_INDEX) {
                (
                    vertex_buffer,
                    render_resources.get_asset_resource(handle, mesh::INDEX_BUFFER_ASSET_INDEX),
                )
            } else {
                let mesh_asset = mesh_storage.get(&handle).unwrap();
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

                render_resources.set_asset_resource(handle, vertex_buffer, mesh::VERTEX_BUFFER_ASSET_INDEX);
                render_resources.set_asset_resource(handle, index_buffer, mesh::INDEX_BUFFER_ASSET_INDEX);
                (vertex_buffer, Some(index_buffer))
            };

        render_resource_assignments.set_vertex_buffer("Vertex", vertex_buffer, index_buffer);
    }
}

impl ResourceProvider for MeshResourceProvider {
    fn initialize(
        &mut self,
        _render_context: &mut dyn RenderContext,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();
        vertex_buffer_descriptors.set(Vertex::get_vertex_buffer_descriptor().cloned().unwrap());
    }

    fn update(
        &mut self,
        _render_context: &mut dyn RenderContext,
        _world: &World,
        _resources: &Resources,
    ) {
    }

    fn finish_update(
        &mut self,
        render_context: &mut dyn RenderContext,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mesh_storage = resources.get::<AssetStorage<Mesh>>().unwrap();
        let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();

        // this scope is necessary because the Fetch<AssetBatchers> pointer behaves weirdly
        {
            if let Some(batches) = asset_batchers.get_handle_batches_mut::<Mesh>() {
                for batch in batches {
                    let handle = batch.get_handle::<Mesh>().unwrap();
                    log::trace!("setup mesh for {:?}", batch.render_resource_assignments.id);
                    Self::setup_mesh_resources(
                        render_context,
                        &mesh_storage,
                        handle,
                        &mut batch.render_resource_assignments,
                    );
                }
            }
        };
    }
}
