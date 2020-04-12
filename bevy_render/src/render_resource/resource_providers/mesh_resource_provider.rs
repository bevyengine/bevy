use crate::{
    mesh::Mesh,
    pipeline::VertexBufferDescriptors,
    render_resource::{
        AssetBatchers, BufferInfo, BufferUsage, RenderResourceAssignments, ResourceProvider,
    },
    shader::AsUniforms,
    Renderable, Vertex, renderer_2::RenderContext,
};
use bevy_asset::{AssetStorage, Handle};
use legion::{filter::*, prelude::*};
use zerocopy::AsBytes;

pub struct MeshResourceProvider {
    pub mesh_query: Query<
        (Read<Handle<Mesh>>, Read<Renderable>),
        EntityFilterTuple<
            And<(
                ComponentFilter<Handle<Mesh>>,
                ComponentFilter<Renderable>,
                ComponentFilter<Handle<Mesh>>,
            )>,
            And<(Passthrough, Passthrough)>,
            And<(
                Passthrough,
                Passthrough,
                ComponentChangedFilter<Handle<Mesh>>,
            )>,
        >,
    >,
}

impl MeshResourceProvider {
    pub fn new() -> Self {
        MeshResourceProvider {
            mesh_query: <(Read<Handle<Mesh>>, Read<Renderable>)>::query()
                .filter(changed::<Handle<Mesh>>()),
        }
    }

    fn setup_mesh_resources(
        render_context: &mut dyn RenderContext,
        mesh_storage: &AssetStorage<Mesh>,
        handle: Handle<Mesh>,
        render_resource_assignments: &mut RenderResourceAssignments,
    ) {
        let render_resources = render_context.resources_mut();
        let (vertex_buffer, index_buffer) = if let Some(vertex_buffer) = render_resources
            .get_mesh_vertices_resource(handle)
        {
            (
                vertex_buffer,
                render_resources
                    .get_mesh_indices_resource(handle),
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

            let asset_resources = render_resources.asset_resources_mut();
            asset_resources.set_mesh_vertices_resource(handle, vertex_buffer);
            asset_resources.set_mesh_indices_resource(handle, index_buffer);
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

    fn update(&mut self, _render_context: &mut dyn RenderContext, _world: &mut World, _resources: &Resources) {
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
