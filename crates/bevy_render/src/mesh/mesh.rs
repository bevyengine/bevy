mod conversions;

use crate::{
    pipeline::{IndexFormat, PrimitiveTopology, RenderPipelines, VertexFormat},
    renderer::{BufferInfo, BufferUsage, RenderResourceContext, RenderResourceId},
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::cast_slice;
use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    prelude::QueryState,
    query::{Changed, With},
    system::{Local, QuerySet, Res},
    world::Mut,
};
use bevy_math::*;
use bevy_reflect::TypeUuid;
use bevy_utils::EnumVariantMeta;
use std::{borrow::Cow, collections::BTreeMap};

use crate::pipeline::{InputStepMode, VertexAttribute, VertexBufferLayout};
use bevy_utils::{HashMap, HashSet};

pub const INDEX_BUFFER_ASSET_INDEX: u64 = 0;
pub const VERTEX_ATTRIBUTE_BUFFER_ID: u64 = 10;

/// An array where each entry describes a property of a single vertex.
#[derive(Clone, Debug, EnumVariantMeta)]
pub enum VertexAttributeValues {
    Float32(Vec<f32>),
    Sint32(Vec<i32>),
    Uint32(Vec<u32>),
    Float32x2(Vec<[f32; 2]>),
    Sint32x2(Vec<[i32; 2]>),
    Uint32x2(Vec<[u32; 2]>),
    Float32x3(Vec<[f32; 3]>),
    Sint32x3(Vec<[i32; 3]>),
    Uint32x3(Vec<[u32; 3]>),
    Float32x4(Vec<[f32; 4]>),
    Sint32x4(Vec<[i32; 4]>),
    Uint32x4(Vec<[u32; 4]>),
    Sint16x2(Vec<[i16; 2]>),
    Snorm16x2(Vec<[i16; 2]>),
    Uint16x2(Vec<[u16; 2]>),
    Unorm16x2(Vec<[u16; 2]>),
    Sint16x4(Vec<[i16; 4]>),
    Snorm16x4(Vec<[i16; 4]>),
    Uint16x4(Vec<[u16; 4]>),
    Unorm16x4(Vec<[u16; 4]>),
    Sint8x2(Vec<[i8; 2]>),
    Snorm8x2(Vec<[i8; 2]>),
    Uint8x2(Vec<[u8; 2]>),
    Unorm8x2(Vec<[u8; 2]>),
    Sint8x4(Vec<[i8; 4]>),
    Snorm8x4(Vec<[i8; 4]>),
    Uint8x4(Vec<[u8; 4]>),
    Unorm8x4(Vec<[u8; 4]>),
}

impl VertexAttributeValues {
    /// Returns the number of vertices in this VertexAttribute. For a single
    /// mesh, all of the VertexAttributeValues must have the same length.
    pub fn len(&self) -> usize {
        match *self {
            VertexAttributeValues::Float32(ref values) => values.len(),
            VertexAttributeValues::Sint32(ref values) => values.len(),
            VertexAttributeValues::Uint32(ref values) => values.len(),
            VertexAttributeValues::Float32x2(ref values) => values.len(),
            VertexAttributeValues::Sint32x2(ref values) => values.len(),
            VertexAttributeValues::Uint32x2(ref values) => values.len(),
            VertexAttributeValues::Float32x3(ref values) => values.len(),
            VertexAttributeValues::Sint32x3(ref values) => values.len(),
            VertexAttributeValues::Uint32x3(ref values) => values.len(),
            VertexAttributeValues::Float32x4(ref values) => values.len(),
            VertexAttributeValues::Sint32x4(ref values) => values.len(),
            VertexAttributeValues::Uint32x4(ref values) => values.len(),
            VertexAttributeValues::Sint16x2(ref values) => values.len(),
            VertexAttributeValues::Snorm16x2(ref values) => values.len(),
            VertexAttributeValues::Uint16x2(ref values) => values.len(),
            VertexAttributeValues::Unorm16x2(ref values) => values.len(),
            VertexAttributeValues::Sint16x4(ref values) => values.len(),
            VertexAttributeValues::Snorm16x4(ref values) => values.len(),
            VertexAttributeValues::Uint16x4(ref values) => values.len(),
            VertexAttributeValues::Unorm16x4(ref values) => values.len(),
            VertexAttributeValues::Sint8x2(ref values) => values.len(),
            VertexAttributeValues::Snorm8x2(ref values) => values.len(),
            VertexAttributeValues::Uint8x2(ref values) => values.len(),
            VertexAttributeValues::Unorm8x2(ref values) => values.len(),
            VertexAttributeValues::Sint8x4(ref values) => values.len(),
            VertexAttributeValues::Snorm8x4(ref values) => values.len(),
            VertexAttributeValues::Uint8x4(ref values) => values.len(),
            VertexAttributeValues::Unorm8x4(ref values) => values.len(),
        }
    }

    /// Returns `true` if there are no vertices in this VertexAttributeValue
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn as_float3(&self) -> Option<&[[f32; 3]]> {
        match self {
            VertexAttributeValues::Float32x3(values) => Some(values),
            _ => None,
        }
    }

    // TODO: add vertex format as parameter here and perform type conversions
    /// Flattens the VertexAttributeArray into a sequence of bytes. This is
    /// useful for serialization and sending to the GPU.
    pub fn get_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Float32(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint32(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint32(values) => cast_slice(&values[..]),
            VertexAttributeValues::Float32x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint32x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint32x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Float32x3(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint32x3(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint32x3(values) => cast_slice(&values[..]),
            VertexAttributeValues::Float32x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint32x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint32x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint16x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Snorm16x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint16x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Unorm16x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint16x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Snorm16x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint16x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Unorm16x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint8x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Snorm8x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint8x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Unorm8x2(values) => cast_slice(&values[..]),
            VertexAttributeValues::Sint8x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Snorm8x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Uint8x4(values) => cast_slice(&values[..]),
            VertexAttributeValues::Unorm8x4(values) => cast_slice(&values[..]),
        }
    }
}

impl From<&VertexAttributeValues> for VertexFormat {
    fn from(values: &VertexAttributeValues) -> Self {
        match values {
            VertexAttributeValues::Float32(_) => VertexFormat::Float32,
            VertexAttributeValues::Sint32(_) => VertexFormat::Sint32,
            VertexAttributeValues::Uint32(_) => VertexFormat::Uint32,
            VertexAttributeValues::Float32x2(_) => VertexFormat::Float32x2,
            VertexAttributeValues::Sint32x2(_) => VertexFormat::Sint32x2,
            VertexAttributeValues::Uint32x2(_) => VertexFormat::Uint32x2,
            VertexAttributeValues::Float32x3(_) => VertexFormat::Float32x3,
            VertexAttributeValues::Sint32x3(_) => VertexFormat::Sint32x3,
            VertexAttributeValues::Uint32x3(_) => VertexFormat::Uint32x3,
            VertexAttributeValues::Float32x4(_) => VertexFormat::Float32x4,
            VertexAttributeValues::Sint32x4(_) => VertexFormat::Sint32x4,
            VertexAttributeValues::Uint32x4(_) => VertexFormat::Uint32x4,
            VertexAttributeValues::Sint16x2(_) => VertexFormat::Sint16x2,
            VertexAttributeValues::Snorm16x2(_) => VertexFormat::Snorm16x2,
            VertexAttributeValues::Uint16x2(_) => VertexFormat::Uint16x2,
            VertexAttributeValues::Unorm16x2(_) => VertexFormat::Unorm16x2,
            VertexAttributeValues::Sint16x4(_) => VertexFormat::Sint16x4,
            VertexAttributeValues::Snorm16x4(_) => VertexFormat::Snorm16x4,
            VertexAttributeValues::Uint16x4(_) => VertexFormat::Uint16x4,
            VertexAttributeValues::Unorm16x4(_) => VertexFormat::Unorm16x4,
            VertexAttributeValues::Sint8x2(_) => VertexFormat::Sint8x2,
            VertexAttributeValues::Snorm8x2(_) => VertexFormat::Snorm8x2,
            VertexAttributeValues::Uint8x2(_) => VertexFormat::Uint8x2,
            VertexAttributeValues::Unorm8x2(_) => VertexFormat::Unorm8x2,
            VertexAttributeValues::Sint8x4(_) => VertexFormat::Sint8x4,
            VertexAttributeValues::Snorm8x4(_) => VertexFormat::Snorm8x4,
            VertexAttributeValues::Uint8x4(_) => VertexFormat::Uint8x4,
            VertexAttributeValues::Unorm8x4(_) => VertexFormat::Unorm8x4,
        }
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

impl Indices {
    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        match self {
            Indices::U16(vec) => IndicesIter::U16(vec.iter()),
            Indices::U32(vec) => IndicesIter::U32(vec.iter()),
        }
    }
}
enum IndicesIter<'a> {
    U16(std::slice::Iter<'a, u16>),
    U32(std::slice::Iter<'a, u32>),
}
impl Iterator for IndicesIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IndicesIter::U16(iter) => iter.next().map(|val| *val as usize),
            IndicesIter::U32(iter) => iter.next().map(|val| *val as usize),
        }
    }
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
#[derive(Debug, TypeUuid, Clone)]
#[uuid = "8ecbac0f-f545-4473-ad43-e1f4243af51e"]
pub struct Mesh {
    primitive_topology: PrimitiveTopology,
    /// `std::collections::BTreeMap` with all defined vertex attributes (Positions, Normals, ...)
    /// for this mesh. Attribute name maps to attribute values.
    /// Uses a BTreeMap because, unlike HashMap, it has a defined iteration order,
    /// which allows easy stable VertexBuffers (i.e. same buffer order)
    attributes: BTreeMap<Cow<'static, str>, VertexAttributeValues>,
    indices: Option<Indices>,
}

/// Contains geometry in the form of a mesh.
///
/// Often meshes are automatically generated by bevy's asset loaders or primitives, such as
/// [`crate::shape::Cube`] or [`crate::shape::Box`], but you can also construct
/// one yourself.
///
/// Example of constructing a mesh:
/// ```
/// # use bevy_render::mesh::{Mesh, Indices};
/// # use bevy_render::pipeline::PrimitiveTopology;
/// fn create_triangle() -> Mesh {
///     let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
///     mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]]);
///     mesh.set_indices(Some(Indices::U32(vec![0,1,2])));
///     mesh
/// }
/// ```
impl Mesh {
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

    /// Per vertex joint transform matrix weight. Use in conjunction with [`Mesh::set_attribute`]
    pub const ATTRIBUTE_JOINT_WEIGHT: &'static str = "Vertex_JointWeight";
    /// Per vertex joint transform matrix index. Use in conjunction with [`Mesh::set_attribute`]
    pub const ATTRIBUTE_JOINT_INDEX: &'static str = "Vertex_JointIndex";

    /// Construct a new mesh. You need to provide a PrimitiveTopology so that the
    /// renderer knows how to treat the vertex data. Most of the time this will be
    /// `PrimitiveTopology::TriangleList`.
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

    /// Sets the data for a vertex attribute (position, normal etc.). The name will
    /// often be one of the associated constants such as [`Mesh::ATTRIBUTE_POSITION`]
    pub fn set_attribute(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        values: impl Into<VertexAttributeValues>,
    ) {
        let values: VertexAttributeValues = values.into();
        self.attributes.insert(name.into(), values);
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
    pub fn set_indices(&mut self, indices: Option<Indices>) {
        self.indices = indices;
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        self.indices.as_mut()
    }

    pub fn get_index_buffer_bytes(&self) -> Option<&[u8]> {
        self.indices.as_ref().map(|indices| match &indices {
            Indices::U16(indices) => cast_slice(&indices[..]),
            Indices::U32(indices) => cast_slice(&indices[..]),
        })
    }

    pub fn get_vertex_buffer_layout(&self) -> VertexBufferLayout {
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

        VertexBufferLayout {
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

    /// Duplicates the vertex attributes so that no vertices are shared.
    ///
    /// This can dramatically increase the vertex count, so make sure this is what you want.
    /// Does nothing if no [Indices] are set.
    pub fn duplicate_vertices(&mut self) {
        fn duplicate<T: Copy>(values: &[T], indices: impl Iterator<Item = usize>) -> Vec<T> {
            indices.map(|i| values[i]).collect()
        }

        assert!(
            matches!(self.primitive_topology, PrimitiveTopology::TriangleList),
            "can only duplicate vertices for `TriangleList`s"
        );

        let indices = match self.indices.take() {
            Some(indices) => indices,
            None => return,
        };
        for (_, attributes) in self.attributes.iter_mut() {
            let indices = indices.iter();
            match attributes {
                VertexAttributeValues::Float32(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Float32x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Float32x3(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32x3(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32x3(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Float32x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint8x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm8x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint8x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm8x4(vec) => *vec = duplicate(vec, indices),
            }
        }
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of a mesh.
    ///
    /// Panics if [`Indices`] are set.
    /// Consider calling [Mesh::duplicate_vertices] or export your mesh with normal attributes.
    pub fn compute_flat_normals(&mut self) {
        if self.indices().is_some() {
            panic!("`compute_flat_normals` can't work on indexed geometry. Consider calling `Mesh::duplicate_vertices`.");
        }

        let positions = self
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes should be of type `float3`");

        let normals: Vec<_> = positions
            .chunks_exact(3)
            .map(|p| face_normal(p[0], p[1], p[2]))
            .flat_map(|normal| std::array::IntoIter::new([normal, normal, normal]))
            .collect();

        self.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
    (b - a).cross(c - a).normalize().into()
}

fn remove_resource_save(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
    index: u64,
) {
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(handle, index)
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

#[allow(clippy::type_complexity)]
pub fn mesh_resource_provider_system(
    mut state: Local<MeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    meshes: Res<Assets<Mesh>>,
    mut mesh_events: EventReader<AssetEvent<Mesh>>,
    mut queries: QuerySet<(
        QueryState<&mut RenderPipelines, With<Handle<Mesh>>>,
        QueryState<(Entity, &Handle<Mesh>, &mut RenderPipelines), Changed<Handle<Mesh>>>,
    )>,
) {
    let mut changed_meshes = HashSet::default();
    let render_resource_context = &**render_resource_context;
    for event in mesh_events.iter() {
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
                    data,
                );

                render_resource_context.set_asset_resource(
                    changed_mesh_handle,
                    RenderResourceId::Buffer(index_buffer),
                    INDEX_BUFFER_ASSET_INDEX,
                );
            }

            let interleaved_buffer = mesh.get_vertex_buffer_data();
            if !interleaved_buffer.is_empty() {
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
            }

            if let Some(mesh_entities) = state.mesh_entities.get_mut(changed_mesh_handle) {
                for entity in mesh_entities.entities.iter() {
                    if let Ok(render_pipelines) = queries.q0().get_mut(*entity) {
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
    for (entity, handle, render_pipelines) in queries.q1().iter_mut() {
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
        render_pipeline.specialization.vertex_buffer_layout = mesh.get_vertex_buffer_layout();
        if let PrimitiveTopology::LineStrip | PrimitiveTopology::TriangleStrip =
            mesh.primitive_topology
        {
            render_pipeline.specialization.strip_index_format =
                mesh.indices().map(|indices| indices.into());
        }
    }
    if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
        render_resource_context.get_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX)
    {
        let index_format: IndexFormat = mesh.indices().unwrap().into();
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
