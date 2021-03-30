use crate::pipeline::{InputStepMode, VertexAttribute, VertexBufferLayout};
use crate::{
    pipeline::{IndexFormat, PrimitiveTopology, RenderPipelines, VertexFormat},
    renderer::{BufferInfo, BufferUsage, RenderResourceContext, RenderResourceId},
};
use bevy_app::prelude::EventReader;
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::AsBytes;
use bevy_ecs::prelude::ResMut;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, With},
    system::{Local, Query, QuerySet, Res},
    world::Mut,
};
use bevy_math::*;
use bevy_reflect::TypeUuid;
use bevy_utils::tracing::error;
use bevy_utils::{HashMap, HashSet};
use std::borrow::Cow;
pub const INDEX_BUFFER_ASSET_INDEX: u64 = 0;
pub const VERTEX_ATTRIBUTE_BUFFER_ID: u64 = 10;

/// An array where each entry describes a property of a single vertex.
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
    /// Returns the number of vertices in this VertexAttribute. For a single
    /// mesh, all of the VertexAttributeValues must have the same length.
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

    /// Returns `true` if there are no vertices in this VertexAttributeValue
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // TODO: add vertex format as parameter here and perform type conversions
    /// Flattens the VertexAttributeArray into a sequence of bytes. This is
    /// useful for serialization and sending to the GPU.
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

/// An array of indices into the VertexAttributeValues for a mesh.
///
/// It describes the order in which the vertex attributes should be joined into faces.
#[derive(Debug, Clone)]
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

/// `bevy_utils::HashMap` with all defined vertex attributes (Positions, Normals, ...) for this
/// mesh. Attribute name maps to attribute values.
pub type MeshAttributes = HashMap<Cow<'static, str>, VertexAttributeValues>;

#[derive(Debug, TypeUuid, Clone)]
#[uuid = "8ecbac0f-f545-4473-ad43-e1f4243af51e"]
pub struct Mesh {
    primitive_topology: PrimitiveTopology,
    storage: MeshStorageState,
}

#[derive(Debug, Clone, Default)]
pub struct MeshData {
    attributes: MeshAttributes,
    indices: Option<Indices>,
    meta: MeshMetaInfo,
}

#[derive(Debug, Clone, Default)]
pub struct MeshMetaInfo {
    vertex_layout: VertexBufferLayout,
    vertex_count: u32,
    index: Option<(u32, IndexFormat)>,
}

#[derive(Debug, Clone)]
enum MeshStorageState {
    /// Only a local copy exists, but it's not intended for realtime modification.
    ToBeOnGpu(MeshData),
    /// Data only exists on the Gpu.
    OnGpu(MeshMetaInfo),
    /// A local copy exists and is available for realtime modification.
    OnGpuAndLocal(MeshData),
}

impl MeshMetaInfo {
    pub fn get_vertex_buffer_layout(&self) -> VertexBufferLayout {
        self.vertex_layout.clone()
    }

    pub fn get_vertices_count(&self) -> usize {
        self.vertex_count as usize
    }

    pub fn get_index_count(&self) -> Option<u32> {
        self.index.map(|x| x.0)
    }

    pub fn get_index_format(&self) -> Option<IndexFormat> {
        self.index.map(|x| x.1)
    }
}

impl MeshData {
    /// Per vertex coloring. Use in conjunction with [`Mesh::set_attribute`]
    pub const ATTRIBUTE_COLOR: &'static str = "Vertex_Color";
    /// The direction the vertex normal is facing in.
    /// Use in conjunction with [`Mesh::set_attribute`]
    pub const ATTRIBUTE_NORMAL: &'static str = "Vertex_Normal";
    /// The direction of the vertex tangent. Used for normal mapping
    pub const ATTRIBUTE_TANGENT: &'static str = "Vertex_Tangent";

    /// Where the vertex is located in space. Use in conjunction with [`Mesh::set_attribute`]
    pub const ATTRIBUTE_POSITION: &'static str = "Vertex_Position";
    /// Texture coordinates for the vertex. Use in conjunction with [`Mesh::set_attribute`]
    pub const ATTRIBUTE_UV_0: &'static str = "Vertex_Uv";

    /// Sets the data for a vertex attribute (position, normal etc.). The name will
    /// often be one of the associated constants such as [`MeshData::ATTRIBUTE_POSITION`]
    pub fn set_attribute(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        values: impl Into<VertexAttributeValues>,
    ) {
        let values: VertexAttributeValues = values.into();
        let name = name.into();
        // update or verify the vertex count
        if self.meta.vertex_count == 0 {
            self.meta.vertex_count = values.len() as u32;
        } else {
            assert_eq!(values.len() as u32, self.meta.vertex_count,
                       "Attribute {} has a different vertex count ({}) than other attributes ({}) in this mesh.", &name, values.len(), self.meta.vertex_count);
        }

        self.attributes.insert(name, values);

        // update the vertex layout
        let mut attributes = Vec::new();
        let mut accumulated_offset = 0;
        for (attribute_name, attribute_values) in self.attributes.iter() {
            let vertex_format = VertexFormat::from(attribute_values);
            attributes.push(VertexAttribute {
                name: attribute_name.clone(),
                offset: accumulated_offset,
                format: vertex_format,
                shader_location: 0,
            });
            accumulated_offset += vertex_format.get_size();
        }

        self.meta.vertex_layout = VertexBufferLayout {
            name: Default::default(),
            stride: accumulated_offset,
            step_mode: InputStepMode::Vertex,
            attributes,
        }
    }

    /// Retrieve the data currently set behind a vertex attribute.
    pub fn attribute(&self, name: impl Into<Cow<'static, str>>) -> Option<&VertexAttributeValues> {
        self.attributes.get(&name.into())
    }

    pub fn attribute_mut(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&mut VertexAttributeValues> {
        self.attributes.get_mut(&name.into())
    }
    /// Indices describe how triangles are constructed out of the vertex attributes.
    /// They are only useful for the [`crate::pipeline::PrimitiveTopology`] variants that use
    /// triangles
    pub fn set_indices(&mut self, new_indices: Option<Indices>) {
        self.indices = new_indices;

        // count indices and set indices format, if existing
        self.meta.index = None;
        if let Some(indices) = &self.indices {
            match indices {
                Indices::U16(indices) => {
                    self.meta.index = Some((indices.len() as u32, IndexFormat::Uint16));
                }
                Indices::U32(indices) => {
                    self.meta.index = Some((indices.len() as u32, IndexFormat::Uint32));
                }
            }
        }
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn get_indices_buffer_bytes(&self) -> Option<Vec<u8>> {
        self.indices.as_ref().map(|indices| match indices {
            Indices::U16(indices) => indices.as_slice().as_bytes().to_vec(),
            Indices::U32(indices) => indices.as_slice().as_bytes().to_vec(),
        })
    }

    pub fn get_vertex_buffer_data(&self) -> Vec<u8> {
        let mut vertex_size = 0;
        for attribute_values in self.attributes.values() {
            let vertex_format = VertexFormat::from(attribute_values);
            vertex_size += vertex_format.get_size() as usize;
        }

        let vertex_count = self.meta.get_vertices_count();
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

    pub fn get_meta(&self) -> &MeshMetaInfo {
        &self.meta
    }
}

/// Contains geometry in the form of a mesh.
///
/// Often meshes are automatically generated by bevy's asset loaders or primitives, such as
/// [`crate::shape::Cube`] or [`crate::shape::Box`], but you can also construct
/// one yourself.
///
/// Example of constructing a mesh:
/// ```
/// # use bevy_render::mesh::{Mesh, Indices, MeshData};
/// # use bevy_render::pipeline::PrimitiveTopology;
/// fn create_triangle() -> Mesh {
///     
///     let mut mesh_data = MeshData::default();
///     mesh_data.set_attribute(MeshData::ATTRIBUTE_POSITION, vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]]);
///     mesh_data.set_indices(Some(Indices::U32(vec![0,1,2])));
///     Mesh::new_static(PrimitiveTopology::TriangleList, mesh_data)
/// }
/// ```
impl Mesh {
    /// Construct a new mesh. You need to provide a PrimitiveTopology so that the
    /// renderer knows how to treat the vertex data. Most of the time this will be
    /// `PrimitiveTopology::TriangleList`. The mesh will not be editable after creation.
    /// If you want to edit the mesh further, use [`Mesh::new_dynamic`]
    pub fn new_static(primitive_topology: PrimitiveTopology, mesh_data: MeshData) -> Self {
        Mesh {
            primitive_topology,
            storage: MeshStorageState::ToBeOnGpu(mesh_data),
        }
    }

    /// Like [`Mesh::new`], but the data will not be locally unloaded and can be modified at runtime. Consumes additional RAM.
    pub fn new_dynamic(primitive_topology: PrimitiveTopology, mesh_data: MeshData) -> Self {
        Mesh {
            primitive_topology,
            storage: MeshStorageState::OnGpuAndLocal(mesh_data),
        }
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        self.primitive_topology
    }

    pub fn get_mesh_data(&self) -> Option<&MeshData> {
        match &self.storage {
            MeshStorageState::OnGpuAndLocal(mesh_data) => Some(&mesh_data),
            _ => None,
        }
    }

    pub fn get_mesh_data_mut(&mut self) -> Option<&mut MeshData> {
        match &mut self.storage {
            MeshStorageState::OnGpuAndLocal(mesh_data) => Some(&mut *mesh_data),
            _ => None,
        }
    }

    pub fn meta(&self) -> &MeshMetaInfo {
        match &self.storage {
            MeshStorageState::ToBeOnGpu(mesh_data) => &mesh_data.meta,
            MeshStorageState::OnGpu(meta_data) => &meta_data,
            MeshStorageState::OnGpuAndLocal(mesh_data) => &mesh_data.meta,
        }
    }

    /// Whether the mesh data is intend to be edited an runtime
    pub fn is_dynamic(&self) -> bool {
        matches!(&self.storage, MeshStorageState::OnGpuAndLocal(_))
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
    mesh_entities: HashMap<Handle<Mesh>, MeshEntities>,
}

pub fn mesh_resource_provider_system(
    mut state: Local<MeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_events: EventReader<AssetEvent<Mesh>>,
    mut queries: QuerySet<(
        Query<&mut RenderPipelines, With<Handle<Mesh>>>,
        Query<(Entity, &Handle<Mesh>, &mut RenderPipelines), Changed<Handle<Mesh>>>,
    )>,
) {
    let mut meshes_to_upload_to_gpu = HashSet::default();
    let mut meshes_to_locally_unload = HashSet::default();
    let render_resource_context = &**render_resource_context;
    for event in mesh_events.iter() {
        match event {
            AssetEvent::Created { ref handle } => {
                meshes_to_upload_to_gpu.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                if let Some(mesh) = meshes.get(handle) {
                    // don't unload buffers from meshes without a local copy
                    // since they won't be re added anymore.
                    if !matches!(mesh.storage, MeshStorageState::OnGpu(_)) {
                        meshes_to_upload_to_gpu.insert(handle.clone_weak());
                        remove_current_mesh_resources(render_resource_context, handle);
                    }
                }
            }
            AssetEvent::Removed { ref handle } => {
                remove_current_mesh_resources(render_resource_context, handle);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                meshes_to_upload_to_gpu.remove(handle);
            }
        }
    }

    // update changed mesh data
    for changed_mesh_handle in meshes_to_upload_to_gpu.iter() {
        if let Some(mesh) = meshes.get(changed_mesh_handle) {
            if matches!(mesh.storage, MeshStorageState::ToBeOnGpu(_)) {
                meshes_to_locally_unload.insert(changed_mesh_handle.clone_weak());
            }

            match &mesh.storage {
                MeshStorageState::ToBeOnGpu(mesh_data)
                | MeshStorageState::OnGpuAndLocal(mesh_data) => {
                    // TODO: check for individual buffer changes in non-interleaved mode
                    if let Some(data) = mesh_data.get_indices_buffer_bytes() {
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

                    render_resource_context.set_asset_resource(
                        changed_mesh_handle,
                        RenderResourceId::Buffer(render_resource_context.create_buffer_with_data(
                            BufferInfo {
                                buffer_usage: BufferUsage::VERTEX,
                                ..Default::default()
                            },
                            &mesh_data.get_vertex_buffer_data(),
                        )),
                        VERTEX_ATTRIBUTE_BUFFER_ID,
                    );

                    if let Some(mesh_entities) = state.mesh_entities.get_mut(changed_mesh_handle) {
                        for entity in mesh_entities.entities.iter() {
                            if let Ok(render_pipelines) = queries.q0_mut().get_mut(*entity) {
                                update_entity_pipelines(
                                    render_resource_context,
                                    mesh,
                                    changed_mesh_handle,
                                    render_pipelines,
                                );
                            }
                        }
                    }
                }
                _ => {
                    error!("Attempted to upload a mesh without data. This is likely an internal bug. Please report an issue to the Bevy Team.");
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
            update_entity_pipelines(render_resource_context, mesh, handle, render_pipelines);
        }
    }

    // unload local copies of meshes
    for mesh in meshes_to_locally_unload.iter() {
        if let Some(mesh) = meshes.get_mut(mesh) {
            let meta_info = mesh.meta().clone();
            mesh.storage = MeshStorageState::OnGpu(meta_info);
        }
    }
}

fn update_entity_pipelines(
    render_resource_context: &dyn RenderResourceContext,
    mesh: &Mesh,
    handle: &Handle<Mesh>,
    mut render_pipelines: Mut<RenderPipelines>,
) {
    for render_pipeline in render_pipelines.pipelines.iter_mut() {
        render_pipeline.specialization.primitive_topology = mesh.primitive_topology;
        render_pipeline.specialization.vertex_buffer_layout =
            mesh.meta().get_vertex_buffer_layout();
        if let PrimitiveTopology::LineStrip | PrimitiveTopology::TriangleStrip =
            mesh.primitive_topology
        {
            render_pipeline.specialization.strip_index_format = mesh.meta().get_index_format();
        }
    }
    if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
        render_resource_context.get_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX)
    {
        let index_format: IndexFormat = mesh.meta().get_index_format().unwrap();
        // set index buffer into binding
        render_pipelines
            .bindings
            .set_index_buffer(index_buffer_resource, index_format);
    }

    if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_resource)) =
        render_resource_context.get_asset_resource(handle, VERTEX_ATTRIBUTE_BUFFER_ID)
    {
        // set index buffer into binding
        render_pipelines.bindings.vertex_attribute_buffer = Some(vertex_attribute_buffer_resource);
    }
}
