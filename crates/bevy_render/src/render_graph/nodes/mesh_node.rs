use crate::{
    mesh::{self, Mesh},
    pipeline::{
        IndexFormat, PipelineCompiler, PipelineDescriptor, RenderPipelines, VertexBufferDescriptors,
    },
    render_graph::{Node, ResourceSlots, SystemNode},
    renderer::{BufferInfo, BufferUsage, RenderContext, RenderResourceContext, RenderResourceId},
};

use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{Commands, IntoQuerySystem, Local, Query, Res, ResMut, Resources, System, World};

use std::collections::{HashMap, HashSet};

pub struct MeshNode;

impl Node for MeshNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        _render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        // TODO: seems like we don't need to do anything here?
    }
}

impl SystemNode for MeshNode {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = mesh_node_system.system();
        commands.insert_local_resource(system.id(), MeshResourceProviderState::default());

        system
    }
}

#[derive(Default)]
pub struct MeshResourceProviderState {
    mesh_event_reader: EventReader<AssetEvent<Mesh>>,
}

pub fn mesh_node_system(
    mut state: Local<MeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    meshes: Res<Assets<Mesh>>,
    pipeline_compiler: Res<PipelineCompiler>,
    pipeline_descriptors: Res<Assets<PipelineDescriptor>>,
    // TODO: not needed when descriptors are reflected
    mut vertex_buffer_descriptors: ResMut<VertexBufferDescriptors>,
    mesh_events: Res<Events<AssetEvent<Mesh>>>,
    mut query: Query<(&Handle<Mesh>, &mut RenderPipelines)>,
) {
    // Find the vertex buffer descriptor that should be used for each mesh.
    // TODO: support multiple descriptors for a single mesh; this would require changes to the
    // RenderResourceContext to allow storing a resource per (asset, T) where T is some kind of
    // buffer descriptor ID.
    let mut mesh_buffer_descriptors = HashMap::new();
    for (mesh_handle, render_pipelines) in &mut query.iter() {
        if let Some(mesh) = meshes.get(mesh_handle) {
            'pipes: for pipeline in render_pipelines.pipelines.iter() {
                if let Some(specialized_descriptor_handle) = pipeline_compiler
                    .get_specialized_pipeline(pipeline.pipeline, &pipeline.specialization)
                {
                    if let Some(pipeline_descriptor) =
                        pipeline_descriptors.get(&specialized_descriptor_handle)
                    {
                        if let Some(layout) = pipeline_descriptor.layout.as_ref() {
                            // Find the first compatible vertex buffer descriptor.
                            for vb_descriptor in layout.vertex_buffer_descriptors.iter() {
                                if mesh.is_compatible_with_vertex_buffer_descriptor(vb_descriptor) {
                                    // Record the descriptor for when we load the buffer.
                                    mesh_buffer_descriptors.insert(*mesh_handle, vb_descriptor);
                                    break 'pipes;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let render_resource_context = &**render_resource_context;
    let mut changed_meshes = HashSet::new();
    for event in state.mesh_event_reader.iter(&mesh_events) {
        match event {
            AssetEvent::Created { handle } => {
                changed_meshes.insert(*handle);
            }
            AssetEvent::Modified { handle } => {
                changed_meshes.insert(*handle);
                remove_current_mesh_resources(render_resource_context, *handle);
            }
            AssetEvent::Removed { handle } => {
                remove_current_mesh_resources(render_resource_context, *handle);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_meshes.remove(handle);
            }
        }
    }

    for changed_mesh_handle in changed_meshes.into_iter() {
        if let (Some(mesh), Some(vertex_buffer_descriptor)) = (
            meshes.get(&changed_mesh_handle),
            mesh_buffer_descriptors.get(&changed_mesh_handle),
        ) {
            let vertex_bytes = mesh
                .get_vertex_buffer_bytes(vertex_buffer_descriptor)
                .unwrap();
            // TODO: use a staging buffer here
            let vertex_buffer = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::VERTEX,
                    ..Default::default()
                },
                &vertex_bytes,
            );

            // TODO: support optional index buffers
            let index_bytes = mesh.get_index_buffer_bytes(IndexFormat::Uint16).unwrap();
            let index_buffer = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::INDEX,
                    ..Default::default()
                },
                &index_bytes,
            );

            render_resource_context.set_asset_resource(
                changed_mesh_handle,
                RenderResourceId::Buffer(vertex_buffer),
                mesh::VERTEX_BUFFER_ASSET_INDEX,
            );
            render_resource_context.set_asset_resource(
                changed_mesh_handle,
                RenderResourceId::Buffer(index_buffer),
                mesh::INDEX_BUFFER_ASSET_INDEX,
            );
        }
    }

    // TODO: remove this once batches are pipeline specific and deprecate assigned_meshes draw target
    for (handle, mut render_pipelines) in &mut query.iter() {
        if let Some(mesh) = meshes.get(&handle) {
            for render_pipeline in render_pipelines.pipelines.iter_mut() {
                render_pipeline.specialization.primitive_topology = mesh.primitive_topology;
            }
        }

        if let Some(RenderResourceId::Buffer(vertex_buffer)) =
            render_resource_context.get_asset_resource(*handle, mesh::VERTEX_BUFFER_ASSET_INDEX)
        {
            render_pipelines.bindings.set_vertex_buffer(
                "Vertex",
                vertex_buffer,
                render_resource_context
                    .get_asset_resource(*handle, mesh::INDEX_BUFFER_ASSET_INDEX)
                    .and_then(|r| {
                        if let RenderResourceId::Buffer(buffer) = r {
                            Some(buffer)
                        } else {
                            None
                        }
                    }),
            );
        }
    }
}

fn remove_current_mesh_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: Handle<Mesh>,
) {
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(handle, mesh::VERTEX_BUFFER_ASSET_INDEX)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, mesh::VERTEX_BUFFER_ASSET_INDEX);
    }
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(handle, mesh::INDEX_BUFFER_ASSET_INDEX)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, mesh::INDEX_BUFFER_ASSET_INDEX);
    }
}
