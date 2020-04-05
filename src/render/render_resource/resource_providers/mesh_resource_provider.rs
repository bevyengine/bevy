use crate::{
    asset::{AssetStorage, Handle},
    prelude::Renderable,
    render::{
        mesh::Mesh,
        pipeline::VertexBufferDescriptors,
        render_resource::{
            AssetBatchers, BufferInfo, BufferUsage, RenderResourceAssignments, ResourceProvider,
        },
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

    fn setup_mesh_resources(
        renderer: &mut dyn Renderer,
        mesh_storage: &mut AssetStorage<Mesh>,
        handle: Handle<Mesh>,
        render_resource_assignments: &mut RenderResourceAssignments,
    ) {
        let (vertex_buffer, index_buffer) = if let Some(vertex_buffer) = renderer
            .get_render_resources()
            .get_mesh_vertices_resource(handle)
        {
            (
                vertex_buffer,
                renderer
                    .get_render_resources()
                    .get_mesh_indices_resource(handle),
            )
        } else {
            let mesh_asset = mesh_storage.get(&handle).unwrap();
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
            render_resources.set_mesh_vertices_resource(handle, vertex_buffer);
            render_resources.set_mesh_indices_resource(handle, index_buffer);
            (vertex_buffer, Some(index_buffer))
        };

        render_resource_assignments.set_vertex_buffer("Vertex", vertex_buffer, index_buffer);
    }
}

impl ResourceProvider for MeshResourceProvider {
    fn initialize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();
        vertex_buffer_descriptors.set(Vertex::get_vertex_buffer_descriptor().cloned().unwrap());
    }

    fn update(&mut self, _renderer: &mut dyn Renderer, world: &mut World, resources: &Resources) {
        let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
        for (entity, (mesh_handle, _renderable)) in self.mesh_query.iter_entities_mut(world) {
            asset_batchers.set_entity_handle(entity, *mesh_handle);
        }
    }

    fn finish_update(
        &mut self,
        renderer: &mut dyn Renderer,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();

        // this scope is necessary because the Fetch<AssetBatchers> pointer behaves weirdly
        {
            if let Some(batches) = asset_batchers.get_handle_batches_mut::<Mesh>() {
                for batch in batches {
                    let handle = batch.get_handle::<Mesh>().unwrap();
                    log::trace!("setup mesh for {:?}", batch.render_resource_assignments.id);
                    Self::setup_mesh_resources(
                        renderer,
                        &mut mesh_storage,
                        handle,
                        &mut batch.render_resource_assignments,
                    );
                }
            }
        };
    }
}
