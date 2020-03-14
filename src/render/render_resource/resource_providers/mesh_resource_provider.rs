use crate::{
    asset::{AssetStorage, Handle},
    prelude::Renderable,
    render::{
        render_resource::{BufferUsage, ResourceProvider},
        renderer::Renderer,
        mesh::Mesh,
    },
};
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
}

impl ResourceProvider for MeshResourceProvider {
    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources) {
        let mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        for (mesh_handle, _renderable) in self.mesh_query.iter(world) {
            if let None = renderer
                .get_render_resources()
                .get_mesh_vertices_resource(*mesh_handle)
            {
                let mesh_asset = mesh_storage.get(&mesh_handle).unwrap();
                let vertex_buffer = renderer
                    .create_buffer_with_data(mesh_asset.vertices.as_bytes(), BufferUsage::VERTEX);
                let index_buffer = renderer
                    .create_buffer_with_data(mesh_asset.indices.as_bytes(), BufferUsage::INDEX);

                let render_resources = renderer.get_render_resources_mut();
                render_resources.set_mesh_vertices_resource(*mesh_handle, vertex_buffer);
                render_resources.set_mesh_indices_resource(*mesh_handle, index_buffer);
            }
        }
    }
}
