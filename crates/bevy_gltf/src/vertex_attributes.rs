use bevy_mesh::{Mesh, MeshVertexAttribute, VertexAttributeValues as Values, VertexFormat};
use bevy_platform::collections::HashMap;
use gltf::{
    accessor::{DataType, Dimensions},
    mesh::util::{ReadColors, ReadJoints, ReadTexCoords, ReadWeights},
};
use thiserror::Error;

use crate::convert_coordinates::ConvertCoordinates;

/// Represents whether integer data requires normalization
#[derive(Copy, Clone)]
struct Normalization(bool);

impl Normalization {
    fn apply_either<T, U>(
        self,
        value: T,
        normalized_ctor: impl Fn(T) -> U,
        unnormalized_ctor: impl Fn(T) -> U,
    ) -> U {
        if self.0 {
            normalized_ctor(value)
        } else {
            unnormalized_ctor(value)
        }
    }
}

/// An error that occurs when accessing buffer data
#[derive(Error, Debug)]
pub(crate) enum AccessFailed {
    #[error("Malformed vertex attribute data")]
    MalformedData,
    #[error("Unsupported vertex attribute format")]
    UnsupportedFormat,
}

/// Helper for reading buffer data
struct BufferAccessor<'a> {
    accessor: gltf::Accessor<'a>,
    buffer_data: &'a Vec<Vec<u8>>,
    normalization: Normalization,
}

impl<'a> BufferAccessor<'a> {
    /// Creates an iterator over the elements in this accessor
    fn iter<T: gltf::accessor::Item>(self) -> Result<gltf::accessor::Iter<'a, T>, AccessFailed> {
        gltf::accessor::Iter::new(self.accessor, |buffer: gltf::Buffer| {
            self.buffer_data.get(buffer.index()).map(Vec::as_slice)
        })
        .ok_or(AccessFailed::MalformedData)
    }

    /// Applies the element iterator to a constructor or fails if normalization is required
    fn with_no_norm<T: gltf::accessor::Item, U>(
        self,
        ctor: impl Fn(gltf::accessor::Iter<'a, T>) -> U,
    ) -> Result<U, AccessFailed> {
        if self.normalization.0 {
            return Err(AccessFailed::UnsupportedFormat);
        }
        self.iter().map(ctor)
    }

    /// Applies the element iterator and the normalization flag to a constructor
    fn with_norm<T: gltf::accessor::Item, U>(
        self,
        ctor: impl Fn(gltf::accessor::Iter<'a, T>, Normalization) -> U,
    ) -> Result<U, AccessFailed> {
        let normalized = self.normalization;
        self.iter().map(|v| ctor(v, normalized))
    }
}

/// An enum of the iterators user by different vertex attribute formats
enum VertexAttributeIter<'a> {
    // For reading native WGPU formats
    F32(gltf::accessor::Iter<'a, f32>),
    U32(gltf::accessor::Iter<'a, u32>),
    F32x2(gltf::accessor::Iter<'a, [f32; 2]>),
    U32x2(gltf::accessor::Iter<'a, [u32; 2]>),
    F32x3(gltf::accessor::Iter<'a, [f32; 3]>),
    U32x3(gltf::accessor::Iter<'a, [u32; 3]>),
    F32x4(gltf::accessor::Iter<'a, [f32; 4]>),
    U32x4(gltf::accessor::Iter<'a, [u32; 4]>),
    S16x2(gltf::accessor::Iter<'a, [i16; 2]>, Normalization),
    U16x2(gltf::accessor::Iter<'a, [u16; 2]>, Normalization),
    S16x4(gltf::accessor::Iter<'a, [i16; 4]>, Normalization),
    U16x4(gltf::accessor::Iter<'a, [u16; 4]>, Normalization),
    S8x2(gltf::accessor::Iter<'a, [i8; 2]>, Normalization),
    U8x2(gltf::accessor::Iter<'a, [u8; 2]>, Normalization),
    S8x4(gltf::accessor::Iter<'a, [i8; 4]>, Normalization),
    U8x4(gltf::accessor::Iter<'a, [u8; 4]>, Normalization),
    // Additional on-disk formats used for RGB colors
    U16x3(gltf::accessor::Iter<'a, [u16; 3]>, Normalization),
    U8x3(gltf::accessor::Iter<'a, [u8; 3]>, Normalization),
}

impl<'a> VertexAttributeIter<'a> {
    /// Creates an iterator over the elements in a vertex attribute accessor
    fn from_accessor(
        accessor: gltf::Accessor<'a>,
        buffer_data: &'a Vec<Vec<u8>>,
    ) -> Result<VertexAttributeIter<'a>, AccessFailed> {
        let normalization = Normalization(accessor.normalized());
        let format = (accessor.data_type(), accessor.dimensions());
        let acc = BufferAccessor {
            accessor,
            buffer_data,
            normalization,
        };
        match format {
            (DataType::F32, Dimensions::Scalar) => acc.with_no_norm(VertexAttributeIter::F32),
            (DataType::U32, Dimensions::Scalar) => acc.with_no_norm(VertexAttributeIter::U32),
            (DataType::F32, Dimensions::Vec2) => acc.with_no_norm(VertexAttributeIter::F32x2),
            (DataType::U32, Dimensions::Vec2) => acc.with_no_norm(VertexAttributeIter::U32x2),
            (DataType::F32, Dimensions::Vec3) => acc.with_no_norm(VertexAttributeIter::F32x3),
            (DataType::U32, Dimensions::Vec3) => acc.with_no_norm(VertexAttributeIter::U32x3),
            (DataType::F32, Dimensions::Vec4) => acc.with_no_norm(VertexAttributeIter::F32x4),
            (DataType::U32, Dimensions::Vec4) => acc.with_no_norm(VertexAttributeIter::U32x4),
            (DataType::I16, Dimensions::Vec2) => acc.with_norm(VertexAttributeIter::S16x2),
            (DataType::U16, Dimensions::Vec2) => acc.with_norm(VertexAttributeIter::U16x2),
            (DataType::I16, Dimensions::Vec4) => acc.with_norm(VertexAttributeIter::S16x4),
            (DataType::U16, Dimensions::Vec4) => acc.with_norm(VertexAttributeIter::U16x4),
            (DataType::I8, Dimensions::Vec2) => acc.with_norm(VertexAttributeIter::S8x2),
            (DataType::U8, Dimensions::Vec2) => acc.with_norm(VertexAttributeIter::U8x2),
            (DataType::I8, Dimensions::Vec4) => acc.with_norm(VertexAttributeIter::S8x4),
            (DataType::U8, Dimensions::Vec4) => acc.with_norm(VertexAttributeIter::U8x4),
            (DataType::U16, Dimensions::Vec3) => acc.with_norm(VertexAttributeIter::U16x3),
            (DataType::U8, Dimensions::Vec3) => acc.with_norm(VertexAttributeIter::U8x3),
            _ => Err(AccessFailed::UnsupportedFormat),
        }
    }

    /// Materializes values for any supported format of vertex attribute
    fn into_any_values(self, convert_coordinates: bool) -> Result<Values, AccessFailed> {
        match self {
            VertexAttributeIter::F32(it) => Ok(Values::Float32(it.collect())),
            VertexAttributeIter::U32(it) => Ok(Values::Uint32(it.collect())),
            VertexAttributeIter::F32x2(it) => Ok(Values::Float32x2(it.collect())),
            VertexAttributeIter::U32x2(it) => Ok(Values::Uint32x2(it.collect())),
            VertexAttributeIter::F32x3(it) => Ok(if convert_coordinates {
                // The following f32x3 values need to be converted to the correct coordinate system
                // - Positions
                // - Normals
                //
                // See <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview>
                Values::Float32x3(it.map(ConvertCoordinates::convert_coordinates).collect())
            } else {
                Values::Float32x3(it.collect())
            }),
            VertexAttributeIter::U32x3(it) => Ok(Values::Uint32x3(it.collect())),
            VertexAttributeIter::F32x4(it) => Ok(if convert_coordinates {
                // The following f32x4 values need to be converted to the correct coordinate system
                // - Tangents
                //
                // See <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview>
                Values::Float32x4(it.map(ConvertCoordinates::convert_coordinates).collect())
            } else {
                Values::Float32x4(it.collect())
            }),
            VertexAttributeIter::U32x4(it) => Ok(Values::Uint32x4(it.collect())),
            VertexAttributeIter::S16x2(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Snorm16x2, Values::Sint16x2))
            }
            VertexAttributeIter::U16x2(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Unorm16x2, Values::Uint16x2))
            }
            VertexAttributeIter::S16x4(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Snorm16x4, Values::Sint16x4))
            }
            VertexAttributeIter::U16x4(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Unorm16x4, Values::Uint16x4))
            }
            VertexAttributeIter::S8x2(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Snorm8x2, Values::Sint8x2))
            }
            VertexAttributeIter::U8x2(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Unorm8x2, Values::Uint8x2))
            }
            VertexAttributeIter::S8x4(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Snorm8x4, Values::Sint8x4))
            }
            VertexAttributeIter::U8x4(it, n) => {
                Ok(n.apply_either(it.collect(), Values::Unorm8x4, Values::Uint8x4))
            }
            _ => Err(AccessFailed::UnsupportedFormat),
        }
    }

    /// Materializes RGBA values, converting compatible formats to Float32x4
    fn into_rgba_values(self) -> Result<Values, AccessFailed> {
        match self {
            VertexAttributeIter::U8x3(it, Normalization(true)) => Ok(Values::Float32x4(
                ReadColors::RgbU8(it).into_rgba_f32().collect(),
            )),
            VertexAttributeIter::U16x3(it, Normalization(true)) => Ok(Values::Float32x4(
                ReadColors::RgbU16(it).into_rgba_f32().collect(),
            )),
            VertexAttributeIter::F32x3(it) => Ok(Values::Float32x4(
                ReadColors::RgbF32(it).into_rgba_f32().collect(),
            )),
            VertexAttributeIter::U8x4(it, Normalization(true)) => Ok(Values::Float32x4(
                ReadColors::RgbaU8(it).into_rgba_f32().collect(),
            )),
            VertexAttributeIter::U16x4(it, Normalization(true)) => Ok(Values::Float32x4(
                ReadColors::RgbaU16(it).into_rgba_f32().collect(),
            )),
            s => s.into_any_values(false),
        }
    }

    /// Materializes joint index values, converting compatible formats to Uint16x4
    fn into_joint_index_values(self) -> Result<Values, AccessFailed> {
        match self {
            VertexAttributeIter::U8x4(it, Normalization(false)) => {
                Ok(Values::Uint16x4(ReadJoints::U8(it).into_u16().collect()))
            }
            s => s.into_any_values(false),
        }
    }

    /// Materializes joint weight values, converting compatible formats to Float32x4
    fn into_joint_weight_values(self) -> Result<Values, AccessFailed> {
        match self {
            VertexAttributeIter::U8x4(it, Normalization(true)) => {
                Ok(Values::Float32x4(ReadWeights::U8(it).into_f32().collect()))
            }
            VertexAttributeIter::U16x4(it, Normalization(true)) => {
                Ok(Values::Float32x4(ReadWeights::U16(it).into_f32().collect()))
            }
            s => s.into_any_values(false),
        }
    }

    /// Materializes texture coordinate values, converting compatible formats to Float32x2
    fn into_tex_coord_values(self) -> Result<Values, AccessFailed> {
        match self {
            VertexAttributeIter::U8x2(it, Normalization(true)) => Ok(Values::Float32x2(
                ReadTexCoords::U8(it).into_f32().collect(),
            )),
            VertexAttributeIter::U16x2(it, Normalization(true)) => Ok(Values::Float32x2(
                ReadTexCoords::U16(it).into_f32().collect(),
            )),
            s => s.into_any_values(false),
        }
    }
}

enum ConversionMode {
    Any,
    Rgba,
    JointIndex,
    JointWeight,
    TexCoord,
}

#[derive(Error, Debug)]
pub(crate) enum ConvertAttributeError {
    #[error("Vertex attribute {0} has format {1:?} but expected {3:?} for target attribute {2}")]
    WrongFormat(String, VertexFormat, String, VertexFormat),
    #[error("{0} in accessor {1}")]
    AccessFailed(AccessFailed, usize),
    #[error("Unknown vertex attribute {0}")]
    UnknownName(String),
}

pub(crate) fn convert_attribute(
    semantic: gltf::Semantic,
    accessor: gltf::Accessor,
    buffer_data: &Vec<Vec<u8>>,
    custom_vertex_attributes: &HashMap<Box<str>, MeshVertexAttribute>,
    convert_coordinates: bool,
) -> Result<(MeshVertexAttribute, Values), ConvertAttributeError> {
    if let Some((attribute, conversion, convert_coordinates)) = match &semantic {
        gltf::Semantic::Positions => Some((
            Mesh::ATTRIBUTE_POSITION,
            ConversionMode::Any,
            convert_coordinates,
        )),
        gltf::Semantic::Normals => Some((
            Mesh::ATTRIBUTE_NORMAL,
            ConversionMode::Any,
            convert_coordinates,
        )),
        gltf::Semantic::Tangents => Some((
            Mesh::ATTRIBUTE_TANGENT,
            ConversionMode::Any,
            convert_coordinates,
        )),
        gltf::Semantic::Colors(0) => Some((Mesh::ATTRIBUTE_COLOR, ConversionMode::Rgba, false)),
        gltf::Semantic::TexCoords(0) => {
            Some((Mesh::ATTRIBUTE_UV_0, ConversionMode::TexCoord, false))
        }
        gltf::Semantic::TexCoords(1) => {
            Some((Mesh::ATTRIBUTE_UV_1, ConversionMode::TexCoord, false))
        }
        gltf::Semantic::Joints(0) => Some((
            Mesh::ATTRIBUTE_JOINT_INDEX,
            ConversionMode::JointIndex,
            false,
        )),
        gltf::Semantic::Weights(0) => Some((
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            ConversionMode::JointWeight,
            false,
        )),
        gltf::Semantic::Extras(name) => custom_vertex_attributes
            .get(name.as_str())
            .map(|attr| (*attr, ConversionMode::Any, false)),
        _ => None,
    } {
        let raw_iter = VertexAttributeIter::from_accessor(accessor.clone(), buffer_data);
        let converted_values = raw_iter.and_then(|iter| match conversion {
            ConversionMode::Any => iter.into_any_values(convert_coordinates),
            ConversionMode::Rgba => iter.into_rgba_values(),
            ConversionMode::TexCoord => iter.into_tex_coord_values(),
            ConversionMode::JointIndex => iter.into_joint_index_values(),
            ConversionMode::JointWeight => iter.into_joint_weight_values(),
        });
        match converted_values {
            Ok(values) => {
                let loaded_format = VertexFormat::from(&values);
                if attribute.format == loaded_format {
                    Ok((attribute, values))
                } else {
                    Err(ConvertAttributeError::WrongFormat(
                        semantic.to_string(),
                        loaded_format,
                        attribute.name.to_string(),
                        attribute.format,
                    ))
                }
            }
            Err(err) => Err(ConvertAttributeError::AccessFailed(err, accessor.index())),
        }
    } else {
        Err(ConvertAttributeError::UnknownName(semantic.to_string()))
    }
}
