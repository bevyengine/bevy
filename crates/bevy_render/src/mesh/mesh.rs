use crate::{
    pipeline::{IndexFormat, PrimitiveTopology, RenderPipelines, VertexFormat},
    renderer::{BufferInfo, BufferUsage, RenderResourceContext, RenderResourceId},
};
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::AsBytes;
use bevy_ecs::{Changed, Entity, Local, Mut, Query, QuerySet, Res, With};
use bevy_math::*;
use bevy_reflect::TypeUuid;
use std::borrow::Cow;

use crate::pipeline::{InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor};
use bevy_utils::{HashMap, HashSet};

pub const INDEX_BUFFER_ASSET_INDEX: u64 = 0;
pub const VERTEX_ATTRIBUTE_BUFFER_ID: u64 = 10;

#[derive(Clone, Debug)]
pub enum VertexAttributeValues {
    Float(Vec<f32>),
    Int(Vec<i32>),
    Uint(Vec<u32>),
    Float2(Vec<[f32; 2]>),
    Int2(Vec<[i32; 2]>),
    Uint2(Vec<[u32; 2]>),
    Float3(Vec<[f32; 3]>),
    Int3(Vec<[i32; 3]>),
    Uint3(Vec<[u32; 3]>),
    Float4(Vec<[f32; 4]>),
    Int4(Vec<[i32; 4]>),
    Uint4(Vec<[u32; 4]>),
    Uchar4Norm(Vec<[u8; 4]>),
}

impl VertexAttributeValues {
    pub fn len(&self) -> usize {
        match *self {
            VertexAttributeValues::Float(ref values) => values.len(),
            VertexAttributeValues::Int(ref values) => values.len(),
            VertexAttributeValues::Uint(ref values) => values.len(),
            VertexAttributeValues::Float2(ref values) => values.len(),
            VertexAttributeValues::Int2(ref values) => values.len(),
            VertexAttributeValues::Uint2(ref values) => values.len(),
            VertexAttributeValues::Float3(ref values) => values.len(),
            VertexAttributeValues::Int3(ref values) => values.len(),
            VertexAttributeValues::Uint3(ref values) => values.len(),
            VertexAttributeValues::Float4(ref values) => values.len(),
            VertexAttributeValues::Int4(ref values) => values.len(),
            VertexAttributeValues::Uint4(ref values) => values.len(),
            VertexAttributeValues::Uchar4Norm(ref values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // TODO: add vertex format as parameter here and perform type conversions
    pub fn get_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Float(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Int(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Uint(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Float2(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Int2(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Uint2(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Float3(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Int3(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Uint3(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Float4(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Int4(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Uint4(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Uchar4Norm(values) => values.as_slice().as_bytes(),
        }
    }
}

impl From<&VertexAttributeValues> for VertexFormat {
    fn from(values: &VertexAttributeValues) -> Self {
        match values {
            VertexAttributeValues::Float(_) => VertexFormat::Float,
            VertexAttributeValues::Int(_) => VertexFormat::Int,
            VertexAttributeValues::Uint(_) => VertexFormat::Uint,
            VertexAttributeValues::Float2(_) => VertexFormat::Float2,
            VertexAttributeValues::Int2(_) => VertexFormat::Int2,
            VertexAttributeValues::Uint2(_) => VertexFormat::Uint2,
            VertexAttributeValues::Float3(_) => VertexFormat::Float3,
            VertexAttributeValues::Int3(_) => VertexFormat::Int3,
            VertexAttributeValues::Uint3(_) => VertexFormat::Uint3,
            VertexAttributeValues::Float4(_) => VertexFormat::Float4,
            VertexAttributeValues::Int4(_) => VertexFormat::Int4,
            VertexAttributeValues::Uint4(_) => VertexFormat::Uint4,
            VertexAttributeValues::Uchar4Norm(_) => VertexFormat::Uchar4Norm,
        }
    }
}

impl From<Vec<f32>> for VertexAttributeValues {
    fn from(vec: Vec<f32>) -> Self {
        VertexAttributeValues::Float(vec)
    }
}

impl From<Vec<i32>> for VertexAttributeValues {
    fn from(vec: Vec<i32>) -> Self {
        VertexAttributeValues::Int(vec)
    }
}

impl From<Vec<u32>> for VertexAttributeValues {
    fn from(vec: Vec<u32>) -> Self {
        VertexAttributeValues::Uint(vec)
    }
}

impl From<Vec<[f32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 2]>) -> Self {
        VertexAttributeValues::Float2(vec)
    }
}

impl From<Vec<[i32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 2]>) -> Self {
        VertexAttributeValues::Int2(vec)
    }
}

impl From<Vec<[u32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 2]>) -> Self {
        VertexAttributeValues::Uint2(vec)
    }
}

impl From<Vec<[f32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 3]>) -> Self {
        VertexAttributeValues::Float3(vec)
    }
}

impl From<Vec<[i32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 3]>) -> Self {
        VertexAttributeValues::Int3(vec)
    }
}

impl From<Vec<[u32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 3]>) -> Self {
        VertexAttributeValues::Uint3(vec)
    }
}

impl From<Vec<[f32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 4]>) -> Self {
        VertexAttributeValues::Float4(vec)
    }
}

impl From<Vec<[i32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 4]>) -> Self {
        VertexAttributeValues::Int4(vec)
    }
}

impl From<Vec<[u32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 4]>) -> Self {
        VertexAttributeValues::Uint4(vec)
    }
}

impl From<Vec<[u8; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u8; 4]>) -> Self {
        VertexAttributeValues::Uchar4Norm(vec)
    }
}

#[derive(Debug)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl From<&Indices> for IndexFormat {
    fn from(indices: &Indices) -> Self {
        match indices {
            Indices::U16(_) => IndexFormat::Uint16,
            Indices::U32(_) => IndexFormat::Uint32,
        }
    }
}

// TODO: allow values to be unloaded after been submitting to the GPU to conserve memory
#[derive(Debug, TypeUuid)]
#[uuid = "8ecbac0f-f545-4473-ad43-e1f4243af51e"]
pub struct Mesh {
    primitive_topology: PrimitiveTopology,
    /// `bevy_utils::HashMap` with all defined vertex attributes (Positions, Normals, ...) for this mesh. Attribute name maps to attribute values.
    attributes: HashMap<Cow<'static, str>, VertexAttributeValues>,
    indices: Option<Indices>,
}

impl Mesh {
    pub const ATTRIBUTE_COLOR: &'static str = "Vertex_Color";
    pub const ATTRIBUTE_NORMAL: &'static str = "Vertex_Normal";
    pub const ATTRIBUTE_POSITION: &'static str = "Vertex_Position";
    pub const ATTRIBUTE_UV_0: &'static str = "Vertex_Uv";

    pub fn new(primitive_topology: PrimitiveTopology) -> Self {
        Mesh {
            primitive_topology,
            attributes: Default::default(),
            indices: None,
        }
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        self.primitive_topology
    }

    pub fn set_attribute(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        values: impl Into<VertexAttributeValues>,
    ) {
        let values: VertexAttributeValues = values.into();
        self.attributes.insert(name.into(), values);
    }

    pub fn attribute(&self, name: impl Into<Cow<'static, str>>) -> Option<&VertexAttributeValues> {
        self.attributes.get(&name.into())
    }

    pub fn attribute_mut(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&mut VertexAttributeValues> {
        self.attributes.get_mut(&name.into())
    }

    pub fn set_indices(&mut self, indices: Option<Indices>) {
        self.indices = indices;
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        self.indices.as_mut()
    }

    pub fn get_index_buffer_bytes(&self) -> Option<Vec<u8>> {
        self.indices.as_ref().map(|indices| match &indices {
            Indices::U16(indices) => indices.as_slice().as_bytes().to_vec(),
            Indices::U32(indices) => indices.as_slice().as_bytes().to_vec(),
        })
    }

    pub fn get_vertex_buffer_descriptor(&self) -> VertexBufferDescriptor {
        let mut attributes = Vec::new();
        let mut accumulated_offset = 0;
        for (attribute_name, attribute_values) in self.attributes.iter() {
            let vertex_format = VertexFormat::from(attribute_values);
            attributes.push(VertexAttributeDescriptor {
                name: attribute_name.clone(),
                offset: accumulated_offset,
                format: vertex_format,
                shader_location: 0,
            });
            accumulated_offset += vertex_format.get_size();
        }

        VertexBufferDescriptor {
            name: Default::default(),
            stride: accumulated_offset,
            step_mode: InputStepMode::Vertex,
            attributes,
        }
    }

    pub fn count_vertices(&self) -> usize {
        let mut vertex_count: Option<usize> = None;
        for (attribute_name, attribute_data) in self.attributes.iter() {
            let attribute_len = attribute_data.len();
            if let Some(previous_vertex_count) = vertex_count {
                assert_eq!(previous_vertex_count, attribute_len,
                        "Attribute {} has a different vertex count ({}) than other attributes ({}) in this mesh.", attribute_name, attribute_len, previous_vertex_count);
            }
            vertex_count = Some(attribute_len);
        }

        vertex_count.unwrap_or(0)
    }

    pub fn get_vertex_buffer_data(&self) -> Vec<u8> {
        let mut vertex_size = 0;
        for attribute_values in self.attributes.values() {
            let vertex_format = VertexFormat::from(attribute_values);
            vertex_size += vertex_format.get_size() as usize;
        }

        let vertex_count = self.count_vertices();
        let mut attributes_interleaved_buffer = vec![0; vertex_count * vertex_size];
        // bundle into interleaved buffers
        let mut attribute_offset = 0;
        for attribute_values in self.attributes.values() {
            let vertex_format = VertexFormat::from(attribute_values);
            let attribute_size = vertex_format.get_size() as usize;
            let attributes_bytes = attribute_values.get_bytes();
            for (vertex_index, attribute_bytes) in
                attributes_bytes.chunks_exact(attribute_size).enumerate()
            {
                let offset = vertex_index * vertex_size + attribute_offset;
                attributes_interleaved_buffer[offset..offset + attribute_size]
                    .copy_from_slice(attribute_bytes);
            }

            attribute_offset += attribute_size;
        }

        attributes_interleaved_buffer
    }
}

fn remove_resource_save(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
    index: u64,
) {
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(&handle, index)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, index);
    }
}
fn remove_current_mesh_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
) {
    remove_resource_save(render_resource_context, handle, VERTEX_ATTRIBUTE_BUFFER_ID);
    remove_resource_save(render_resource_context, handle, INDEX_BUFFER_ASSET_INDEX);
}

#[derive(Default)]
pub struct MeshEntities {
    entities: HashSet<Entity>,
}

#[derive(Default)]
pub struct MeshResourceProviderState {
    mesh_event_reader: EventReader<AssetEvent<Mesh>>,
    mesh_entities: HashMap<Handle<Mesh>, MeshEntities>,
}

pub fn mesh_resource_provider_system(
    mut state: Local<MeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    meshes: Res<Assets<Mesh>>,
    mesh_events: Res<Events<AssetEvent<Mesh>>>,
    mut queries: QuerySet<(
        Query<&mut RenderPipelines, With<Handle<Mesh>>>,
        Query<(Entity, &Handle<Mesh>, &mut RenderPipelines), Changed<Handle<Mesh>>>,
    )>,
) {
    let mut changed_meshes = HashSet::default();
    let render_resource_context = &**render_resource_context;
    for event in state.mesh_event_reader.iter(&mesh_events) {
        match event {
            AssetEvent::Created { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
                remove_current_mesh_resources(render_resource_context, handle);
            }
            AssetEvent::Removed { ref handle } => {
                remove_current_mesh_resources(render_resource_context, handle);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_meshes.remove(handle);
            }
        }
    }

    // update changed mesh data
    for changed_mesh_handle in changed_meshes.iter() {
        if let Some(mesh) = meshes.get(changed_mesh_handle) {
            // TODO: check for individual buffer changes in non-interleaved mode
            if let Some(data) = mesh.get_index_buffer_bytes() {
                let index_buffer = render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::INDEX,
                        ..Default::default()
                    },
                    &data,
                );

                render_resource_context.set_asset_resource(
                    changed_mesh_handle,
                    RenderResourceId::Buffer(index_buffer),
                    INDEX_BUFFER_ASSET_INDEX,
                );
            }

            let interleaved_buffer = mesh.get_vertex_buffer_data();

            render_resource_context.set_asset_resource(
                changed_mesh_handle,
                RenderResourceId::Buffer(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    &interleaved_buffer,
                )),
                VERTEX_ATTRIBUTE_BUFFER_ID,
            );

            if let Some(mesh_entities) = state.mesh_entities.get_mut(changed_mesh_handle) {
                for entity in mesh_entities.entities.iter() {
                    if let Ok(render_pipelines) = queries.q0_mut().get_mut(*entity) {
                        update_entity_mesh(
                            render_resource_context,
                            mesh,
                            changed_mesh_handle,
                            render_pipelines,
                        );
                    }
                }
            }
        }
    }

    // handover buffers to pipeline
    for (entity, handle, render_pipelines) in queries.q1_mut().iter_mut() {
        let mesh_entities = state
            .mesh_entities
            .entry(handle.clone_weak())
            .or_insert_with(MeshEntities::default);
        mesh_entities.entities.insert(entity);
        if let Some(mesh) = meshes.get(handle) {
            update_entity_mesh(render_resource_context, mesh, handle, render_pipelines);
        }
    }
}

fn update_entity_mesh(
    render_resource_context: &dyn RenderResourceContext,
    mesh: &Mesh,
    handle: &Handle<Mesh>,
    mut render_pipelines: Mut<RenderPipelines>,
) {
    for render_pipeline in render_pipelines.pipelines.iter_mut() {
        render_pipeline.specialization.primitive_topology = mesh.primitive_topology;
        // TODO: don't allocate a new vertex buffer descriptor for every entity
        render_pipeline.specialization.vertex_buffer_descriptor =
            mesh.get_vertex_buffer_descriptor();
        render_pipeline.specialization.index_format = mesh
            .indices()
            .map(|i| i.into())
            .unwrap_or(IndexFormat::Uint32);
    }

    if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
        render_resource_context.get_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX)
    {
        // set index buffer into binding
        render_pipelines
            .bindings
            .set_index_buffer(index_buffer_resource);
    }

    if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_resource)) =
        render_resource_context.get_asset_resource(handle, VERTEX_ATTRIBUTE_BUFFER_ID)
    {
        // set index buffer into binding
        render_pipelines.bindings.vertex_attribute_buffer = Some(vertex_attribute_buffer_resource);
    }
}
