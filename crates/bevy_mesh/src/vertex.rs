use alloc::sync::Arc;
use bevy_derive::EnumVariantMeta;
use bevy_ecs::system::Resource;
use bevy_math::Vec3;
use bevy_utils::HashSet;
use bytemuck::cast_slice;
use core::hash::{Hash, Hasher};
use derive_more::derive::{Display, Error};
use wgpu::{BufferAddress, VertexAttribute, VertexFormat, VertexStepMode};

#[derive(Debug, Clone, Copy)]
pub struct MeshVertexAttribute {
    /// The friendly name of the vertex attribute
    pub name: &'static str,

    /// The _unique_ id of the vertex attribute. This will also determine sort ordering
    /// when generating vertex buffers. Built-in / standard attributes will use "close to zero"
    /// indices. When in doubt, use a random / very large u64 to avoid conflicts.
    pub id: MeshVertexAttributeId,

    /// The format of the vertex attribute.
    pub format: VertexFormat,
}

impl MeshVertexAttribute {
    pub const fn new(name: &'static str, id: u64, format: VertexFormat) -> Self {
        Self {
            name,
            id: MeshVertexAttributeId(id),
            format,
        }
    }

    pub const fn at_shader_location(&self, shader_location: u32) -> VertexAttributeDescriptor {
        VertexAttributeDescriptor::new(shader_location, self.id, self.name)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct MeshVertexAttributeId(u64);

impl From<MeshVertexAttribute> for MeshVertexAttributeId {
    fn from(attribute: MeshVertexAttribute) -> Self {
        attribute.id
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MeshVertexBufferLayout {
    pub(crate) attribute_ids: Vec<MeshVertexAttributeId>,
    pub(crate) layout: VertexBufferLayout,
}

impl MeshVertexBufferLayout {
    pub fn new(attribute_ids: Vec<MeshVertexAttributeId>, layout: VertexBufferLayout) -> Self {
        Self {
            attribute_ids,
            layout,
        }
    }

    #[inline]
    pub fn contains(&self, attribute_id: impl Into<MeshVertexAttributeId>) -> bool {
        self.attribute_ids.contains(&attribute_id.into())
    }

    #[inline]
    pub fn attribute_ids(&self) -> &[MeshVertexAttributeId] {
        &self.attribute_ids
    }

    #[inline]
    pub fn layout(&self) -> &VertexBufferLayout {
        &self.layout
    }

    pub fn get_layout(
        &self,
        attribute_descriptors: &[VertexAttributeDescriptor],
    ) -> Result<VertexBufferLayout, MissingVertexAttributeError> {
        let mut attributes = Vec::with_capacity(attribute_descriptors.len());
        for attribute_descriptor in attribute_descriptors {
            if let Some(index) = self
                .attribute_ids
                .iter()
                .position(|id| *id == attribute_descriptor.id)
            {
                let layout_attribute = &self.layout.attributes[index];
                attributes.push(VertexAttribute {
                    format: layout_attribute.format,
                    offset: layout_attribute.offset,
                    shader_location: attribute_descriptor.shader_location,
                });
            } else {
                return Err(MissingVertexAttributeError {
                    id: attribute_descriptor.id,
                    name: attribute_descriptor.name,
                    pipeline_type: None,
                });
            }
        }

        Ok(VertexBufferLayout {
            array_stride: self.layout.array_stride,
            step_mode: self.layout.step_mode,
            attributes,
        })
    }
}

#[derive(Error, Display, Debug)]
#[display("Mesh is missing requested attribute: {name} ({id:?}, pipeline type: {pipeline_type:?})")]
pub struct MissingVertexAttributeError {
    pub pipeline_type: Option<&'static str>,
    id: MeshVertexAttributeId,
    name: &'static str,
}

pub struct VertexAttributeDescriptor {
    pub shader_location: u32,
    pub id: MeshVertexAttributeId,
    name: &'static str,
}

impl VertexAttributeDescriptor {
    pub const fn new(shader_location: u32, id: MeshVertexAttributeId, name: &'static str) -> Self {
        Self {
            shader_location,
            id,
            name,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MeshAttributeData {
    pub(crate) attribute: MeshVertexAttribute,
    pub(crate) values: VertexAttributeValues,
}

pub(crate) fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
    (b - a).cross(c - a).normalize().into()
}

pub trait VertexFormatSize {
    fn get_size(self) -> u64;
}

impl VertexFormatSize for VertexFormat {
    #[allow(clippy::match_same_arms)]
    fn get_size(self) -> u64 {
        match self {
            VertexFormat::Uint8x2 => 2,
            VertexFormat::Uint8x4 => 4,
            VertexFormat::Sint8x2 => 2,
            VertexFormat::Sint8x4 => 4,
            VertexFormat::Unorm8x2 => 2,
            VertexFormat::Unorm8x4 => 4,
            VertexFormat::Snorm8x2 => 2,
            VertexFormat::Snorm8x4 => 4,
            VertexFormat::Unorm10_10_10_2 => 4,
            VertexFormat::Uint16x2 => 2 * 2,
            VertexFormat::Uint16x4 => 2 * 4,
            VertexFormat::Sint16x2 => 2 * 2,
            VertexFormat::Sint16x4 => 2 * 4,
            VertexFormat::Unorm16x2 => 2 * 2,
            VertexFormat::Unorm16x4 => 2 * 4,
            VertexFormat::Snorm16x2 => 2 * 2,
            VertexFormat::Snorm16x4 => 2 * 4,
            VertexFormat::Float16x2 => 2 * 2,
            VertexFormat::Float16x4 => 2 * 4,
            VertexFormat::Float32 => 4,
            VertexFormat::Float32x2 => 4 * 2,
            VertexFormat::Float32x3 => 4 * 3,
            VertexFormat::Float32x4 => 4 * 4,
            VertexFormat::Uint32 => 4,
            VertexFormat::Uint32x2 => 4 * 2,
            VertexFormat::Uint32x3 => 4 * 3,
            VertexFormat::Uint32x4 => 4 * 4,
            VertexFormat::Sint32 => 4,
            VertexFormat::Sint32x2 => 4 * 2,
            VertexFormat::Sint32x3 => 4 * 3,
            VertexFormat::Sint32x4 => 4 * 4,
            VertexFormat::Float64 => 8,
            VertexFormat::Float64x2 => 8 * 2,
            VertexFormat::Float64x3 => 8 * 3,
            VertexFormat::Float64x4 => 8 * 4,
        }
    }
}

/// Contains an array where each entry describes a property of a single vertex.
/// Matches the [`VertexFormats`](VertexFormat).
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
    /// Returns the number of vertices in this [`VertexAttributeValues`]. For a single
    /// mesh, all of the [`VertexAttributeValues`] must have the same length.
    #[allow(clippy::match_same_arms)]
    pub fn len(&self) -> usize {
        match self {
            VertexAttributeValues::Float32(values) => values.len(),
            VertexAttributeValues::Sint32(values) => values.len(),
            VertexAttributeValues::Uint32(values) => values.len(),
            VertexAttributeValues::Float32x2(values) => values.len(),
            VertexAttributeValues::Sint32x2(values) => values.len(),
            VertexAttributeValues::Uint32x2(values) => values.len(),
            VertexAttributeValues::Float32x3(values) => values.len(),
            VertexAttributeValues::Sint32x3(values) => values.len(),
            VertexAttributeValues::Uint32x3(values) => values.len(),
            VertexAttributeValues::Float32x4(values) => values.len(),
            VertexAttributeValues::Sint32x4(values) => values.len(),
            VertexAttributeValues::Uint32x4(values) => values.len(),
            VertexAttributeValues::Sint16x2(values) => values.len(),
            VertexAttributeValues::Snorm16x2(values) => values.len(),
            VertexAttributeValues::Uint16x2(values) => values.len(),
            VertexAttributeValues::Unorm16x2(values) => values.len(),
            VertexAttributeValues::Sint16x4(values) => values.len(),
            VertexAttributeValues::Snorm16x4(values) => values.len(),
            VertexAttributeValues::Uint16x4(values) => values.len(),
            VertexAttributeValues::Unorm16x4(values) => values.len(),
            VertexAttributeValues::Sint8x2(values) => values.len(),
            VertexAttributeValues::Snorm8x2(values) => values.len(),
            VertexAttributeValues::Uint8x2(values) => values.len(),
            VertexAttributeValues::Unorm8x2(values) => values.len(),
            VertexAttributeValues::Sint8x4(values) => values.len(),
            VertexAttributeValues::Snorm8x4(values) => values.len(),
            VertexAttributeValues::Uint8x4(values) => values.len(),
            VertexAttributeValues::Unorm8x4(values) => values.len(),
        }
    }

    /// Returns `true` if there are no vertices in this [`VertexAttributeValues`].
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the values as float triples if possible.
    pub fn as_float3(&self) -> Option<&[[f32; 3]]> {
        match self {
            VertexAttributeValues::Float32x3(values) => Some(values),
            _ => None,
        }
    }

    // TODO: add vertex format as parameter here and perform type conversions
    /// Flattens the [`VertexAttributeValues`] into a sequence of bytes. This is
    /// useful for serialization and sending to the GPU.
    #[allow(clippy::match_same_arms)]
    pub fn get_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Float32(values) => cast_slice(values),
            VertexAttributeValues::Sint32(values) => cast_slice(values),
            VertexAttributeValues::Uint32(values) => cast_slice(values),
            VertexAttributeValues::Float32x2(values) => cast_slice(values),
            VertexAttributeValues::Sint32x2(values) => cast_slice(values),
            VertexAttributeValues::Uint32x2(values) => cast_slice(values),
            VertexAttributeValues::Float32x3(values) => cast_slice(values),
            VertexAttributeValues::Sint32x3(values) => cast_slice(values),
            VertexAttributeValues::Uint32x3(values) => cast_slice(values),
            VertexAttributeValues::Float32x4(values) => cast_slice(values),
            VertexAttributeValues::Sint32x4(values) => cast_slice(values),
            VertexAttributeValues::Uint32x4(values) => cast_slice(values),
            VertexAttributeValues::Sint16x2(values) => cast_slice(values),
            VertexAttributeValues::Snorm16x2(values) => cast_slice(values),
            VertexAttributeValues::Uint16x2(values) => cast_slice(values),
            VertexAttributeValues::Unorm16x2(values) => cast_slice(values),
            VertexAttributeValues::Sint16x4(values) => cast_slice(values),
            VertexAttributeValues::Snorm16x4(values) => cast_slice(values),
            VertexAttributeValues::Uint16x4(values) => cast_slice(values),
            VertexAttributeValues::Unorm16x4(values) => cast_slice(values),
            VertexAttributeValues::Sint8x2(values) => cast_slice(values),
            VertexAttributeValues::Snorm8x2(values) => cast_slice(values),
            VertexAttributeValues::Uint8x2(values) => cast_slice(values),
            VertexAttributeValues::Unorm8x2(values) => cast_slice(values),
            VertexAttributeValues::Sint8x4(values) => cast_slice(values),
            VertexAttributeValues::Snorm8x4(values) => cast_slice(values),
            VertexAttributeValues::Uint8x4(values) => cast_slice(values),
            VertexAttributeValues::Unorm8x4(values) => cast_slice(values),
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

/// Describes how the vertex buffer is interpreted.
#[derive(Default, Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexBufferLayout {
    /// The stride, in bytes, between elements of this buffer.
    pub array_stride: BufferAddress,
    /// How often this vertex buffer is "stepped" forward.
    pub step_mode: VertexStepMode,
    /// The list of attributes which comprise a single vertex.
    pub attributes: Vec<VertexAttribute>,
}

impl VertexBufferLayout {
    /// Creates a new densely packed [`VertexBufferLayout`] from an iterator of vertex formats.
    /// Iteration order determines the `shader_location` and `offset` of the [`VertexAttributes`](VertexAttribute).
    /// The first iterated item will have a `shader_location` and `offset` of zero.
    /// The `array_stride` is the sum of the size of the iterated [`VertexFormats`](VertexFormat) (in bytes).
    pub fn from_vertex_formats<T: IntoIterator<Item = VertexFormat>>(
        step_mode: VertexStepMode,
        vertex_formats: T,
    ) -> Self {
        let mut offset = 0;
        let mut attributes = Vec::new();
        for (shader_location, format) in vertex_formats.into_iter().enumerate() {
            attributes.push(VertexAttribute {
                format,
                offset,
                shader_location: shader_location as u32,
            });
            offset += format.size();
        }

        VertexBufferLayout {
            array_stride: offset,
            step_mode,
            attributes,
        }
    }

    /// Returns a [`VertexBufferLayout`] with the shader location of every attribute offset by
    /// `location`.
    pub fn offset_locations_by(mut self, location: u32) -> Self {
        self.attributes.iter_mut().for_each(|attr| {
            attr.shader_location += location;
        });
        self
    }
}

/// Describes the layout of the mesh vertices in GPU memory.
///
/// At most one copy of a mesh vertex buffer layout ever exists in GPU memory at
/// once. Therefore, comparing these for equality requires only a single pointer
/// comparison, and this type's [`PartialEq`] and [`Hash`] implementations take
/// advantage of this. To that end, this type doesn't implement
/// [`bevy_derive::Deref`] or [`bevy_derive::DerefMut`] in order to reduce the
/// possibility of accidental deep comparisons, which would be needlessly
/// expensive.
#[derive(Clone, Debug)]
pub struct MeshVertexBufferLayoutRef(pub Arc<MeshVertexBufferLayout>);

/// Stores the single copy of each mesh vertex buffer layout.
#[derive(Clone, Default, Resource)]
pub struct MeshVertexBufferLayouts(HashSet<Arc<MeshVertexBufferLayout>>);

impl MeshVertexBufferLayouts {
    /// Inserts a new mesh vertex buffer layout in the store and returns a
    /// reference to it, reusing the existing reference if this mesh vertex
    /// buffer layout was already in the store.
    pub fn insert(&mut self, layout: MeshVertexBufferLayout) -> MeshVertexBufferLayoutRef {
        // Because the special `PartialEq` and `Hash` implementations that
        // compare by pointer are on `MeshVertexBufferLayoutRef`, not on
        // `Arc<MeshVertexBufferLayout>`, this compares the mesh vertex buffer
        // structurally, not by pointer.
        MeshVertexBufferLayoutRef(
            self.0
                .get_or_insert_with(&layout, |layout| Arc::new(layout.clone()))
                .clone(),
        )
    }
}

impl PartialEq for MeshVertexBufferLayoutRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for MeshVertexBufferLayoutRef {}

impl Hash for MeshVertexBufferLayoutRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the address of the underlying data, so two layouts that share the same
        // `MeshVertexBufferLayout` will have the same hash.
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}
