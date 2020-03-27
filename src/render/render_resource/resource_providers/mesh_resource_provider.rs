use crate::{
    asset::{AssetStorage, Handle},
    prelude::Renderable,
    render::{
        mesh::Mesh,
        render_graph::RenderGraph,
        render_resource::{AssetBatchers, BufferInfo, BufferUsage, ResourceProvider},
        renderer::Renderer,
        shader::AsUniforms,
        Vertex,
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
    fn initialize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph
            .set_vertex_buffer_descriptor(Vertex::get_vertex_buffer_descriptor().cloned().unwrap());
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources) {
        let mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
        for (entity, (mesh_handle, _renderable)) in self.mesh_query.iter_entities(world) {
            asset_batchers.set_entity_handle(entity, *mesh_handle);
            if let None = renderer
                .get_render_resources()
                .get_mesh_vertices_resource(*mesh_handle)
            {
                let mesh_asset = mesh_storage.get(&mesh_handle).unwrap();
                let vertex_buffer = renderer.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    mesh_asset.vertices.as_bytes(),
                );
                let index_buffer = renderer.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::INDEX,
                        ..Default::default()
                    },
                    mesh_asset.indices.as_bytes(),
                );

                let render_resources = renderer.get_render_resources_mut();
                render_resources.set_mesh_vertices_resource(*mesh_handle, vertex_buffer);
                render_resources.set_mesh_indices_resource(*mesh_handle, index_buffer);
            }
        }
    }

    fn finish_update(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        _resources: &Resources,
    ) {
        // TODO: assign vertex buffers
        // let mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        // let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
        // for batch in asset_batchers.get_handle_batches::<Mesh>() {
        // }
    }
}
