use bevy_math::{Vec2, Vec3, Vec3A, Vec4};
use bevy_mesh::{
    encode_compressed_normals, encode_compressed_positions, encode_compressed_tangents,
    morph::MorphAttributes, Mesh, MeshAttributeCompressionFlags, MeshVertexAttribute, UvChannel,
    VertexAttributeValues as Values, VertexFormat,
};
use bevy_platform::collections::HashMap;
use bevy_shape::{Aabb2d, Aabb3d};
use gltf::{
    accessor::{DataType, Dimensions},
    mesh::util::{ReadColors, ReadJoints, ReadWeights},
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
pub enum AccessFailed {
    /// Accessing the data failed because of an issue like a mismatch in stride,
    /// or a buffer view slice failing.
    #[error("Malformed vertex attribute data")]
    MalformedData,
    /// The format supplied is unsupported for this operation.
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
    // Additional on-disk formats used for RGB colors and quantized vectors
    U16x3(gltf::accessor::Iter<'a, [u16; 3]>, Normalization),
    U8x3(gltf::accessor::Iter<'a, [u8; 3]>, Normalization),
    S16x3(gltf::accessor::Iter<'a, [i16; 3]>, Normalization),
    S8x3(gltf::accessor::Iter<'a, [i8; 3]>, Normalization),
}

/// An integer vertex attribute component.
trait Dequantize: Copy + Into<i32> {
    const MIN: i32;
    const MAX: i32;

    /// The component as a float, scaled into [-1, 1] or [0, 1] when normalized.
    fn dequantize(self, normalized: bool) -> f32 {
        let value = self.into() as f32;
        if normalized {
            (value / Self::MAX as f32).max(-1.0)
        } else {
            value
        }
    }

    /// The component remapped from `MIN..=MAX` onto `0..=u16::MAX`, which is exact for 8- and
    /// 16-bit components.
    fn to_unorm16(self) -> u16 {
        ((self.into() - Self::MIN) as u32 * u32::from(u16::MAX) / (Self::MAX - Self::MIN) as u32)
            as u16
    }
}

macro_rules! impl_dequantize {
    ($($ty:ty),*) => {$(
        impl Dequantize for $ty {
            const MIN: i32 = <$ty>::MIN as i32;
            const MAX: i32 = <$ty>::MAX as i32;
        }
    )*};
}

impl_dequantize!(i8, u8, i16, u16);

fn dequantize<T: Dequantize, const N: usize>(
    it: impl Iterator<Item = [T; N]>,
    Normalization(normalized): Normalization,
) -> impl Iterator<Item = [f32; N]> {
    it.map(move |v| v.map(|c| c.dequantize(normalized)))
}

/// `value` converted from glTF's coordinate system when `convert` is set.
///
/// See <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview>
fn maybe_convert<T: ConvertCoordinates>(value: T, convert: bool) -> T {
    if convert {
        value.convert_coordinates()
    } else {
        value
    }
}

/// Collects `it`, converted from glTF's coordinate system when `convert` is set.
fn converted<T: ConvertCoordinates>(it: impl Iterator<Item = T>, convert: bool) -> Vec<T> {
    it.map(|value| maybe_convert(value, convert)).collect()
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
            (DataType::I16, Dimensions::Vec3) => acc.with_norm(VertexAttributeIter::S16x3),
            (DataType::I8, Dimensions::Vec3) => acc.with_norm(VertexAttributeIter::S8x3),
            _ => Err(AccessFailed::UnsupportedFormat),
        }
    }

    /// Materializes float values, converting integer formats to Float32x2, Float32x3 or Float32x4
    fn into_float_values(self, convert: bool) -> Result<Values, AccessFailed> {
        Ok(match self {
            Self::S8x2(it, n) => Values::Float32x2(dequantize(it, n).collect()),
            Self::U8x2(it, n) => Values::Float32x2(dequantize(it, n).collect()),
            Self::S16x2(it, n) => Values::Float32x2(dequantize(it, n).collect()),
            Self::U16x2(it, n) => Values::Float32x2(dequantize(it, n).collect()),
            Self::S8x3(it, n) => Values::Float32x3(converted(dequantize(it, n), convert)),
            Self::U8x3(it, n) => Values::Float32x3(converted(dequantize(it, n), convert)),
            Self::S16x3(it, n) => Values::Float32x3(converted(dequantize(it, n), convert)),
            Self::U16x3(it, n) => Values::Float32x3(converted(dequantize(it, n), convert)),
            Self::S8x4(it, n) => Values::Float32x4(converted(dequantize(it, n), convert)),
            Self::U8x4(it, n) => Values::Float32x4(converted(dequantize(it, n), convert)),
            Self::S16x4(it, n) => Values::Float32x4(converted(dequantize(it, n), convert)),
            Self::U16x4(it, n) => Values::Float32x4(converted(dequantize(it, n), convert)),
            s => return s.into_any_values(convert),
        })
    }

    /// Materializes normal or tangent values, converting signed normalized formats to Float32x3 or Float32x4
    fn into_direction_values(self, convert: bool) -> Result<Values, AccessFailed> {
        match self {
            s @ (Self::S8x3(_, Normalization(true))
            | Self::S16x3(_, Normalization(true))
            | Self::S8x4(_, Normalization(true))
            | Self::S16x4(_, Normalization(true))) => s.into_float_values(convert),
            s => s.into_any_values(convert),
        }
    }

    /// Materializes morph target position displacements, converting signed formats to floats
    fn into_displacements(self, convert: bool) -> Result<Vec<[f32; 3]>, AccessFailed> {
        Ok(match self {
            Self::F32x3(it) => converted(it, convert),
            Self::S8x3(it, n) => converted(dequantize(it, n), convert),
            Self::S16x3(it, n) => converted(dequantize(it, n), convert),
            _ => return Err(AccessFailed::UnsupportedFormat),
        })
    }

    /// Materializes morph target normal or tangent displacements, converting signed normalized formats to floats
    fn into_direction_displacements(self, convert: bool) -> Result<Vec<[f32; 3]>, AccessFailed> {
        match self {
            Self::S8x3(_, Normalization(false)) | Self::S16x3(_, Normalization(false)) => {
                Err(AccessFailed::UnsupportedFormat)
            }
            s => s.into_displacements(convert),
        }
    }

    /// Materializes values for any supported format of vertex attribute
    fn into_any_values(self, convert_coordinates: bool) -> Result<Values, AccessFailed> {
        match self {
            VertexAttributeIter::F32(it) => Ok(Values::Float32(it.collect())),
            VertexAttributeIter::U32(it) => Ok(Values::Uint32(it.collect())),
            VertexAttributeIter::F32x2(it) => Ok(Values::Float32x2(it.collect())),
            VertexAttributeIter::U32x2(it) => Ok(Values::Uint32x2(it.collect())),
            VertexAttributeIter::F32x3(it) => {
                Ok(Values::Float32x3(converted(it, convert_coordinates)))
            }
            VertexAttributeIter::U32x3(it) => Ok(Values::Uint32x3(it.collect())),
            VertexAttributeIter::F32x4(it) => {
                Ok(Values::Float32x4(converted(it, convert_coordinates)))
            }
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
}

enum ConversionMode {
    Any,
    Float,
    Direction,
    Rgba,
    JointIndex,
    JointWeight,
}

/// Errors that can occur during the `convert_attribute` function.
#[derive(Error, Debug)]
pub enum ConvertAttributeError {
    /// The loaded format ws different than the attribute's intended format.
    /// Such as if a `Float32x3` was loaded as a `Float32x2`.
    #[error("Vertex attribute {0} has format {1:?} but expected {3:?} for target attribute {2}")]
    WrongFormat(String, VertexFormat, String, VertexFormat),
    /// Fetching values from the glTF Accessor failed
    #[error("{0} in accessor {1}")]
    AccessFailed(AccessFailed, usize),
    /// A vertex attribute name was not one of the gltf crate's well-known attributes,
    /// nor was it registered by a user as a custom attribute. Therefore it is unknown.
    #[error("Unknown vertex attribute {0}")]
    UnknownName(String),
}

/// map glTF vertex attributes into their `MeshVertexAttribute` forms, optionally
/// converting values if necessary.
pub fn convert_attribute(
    semantic: gltf::Semantic,
    accessor: gltf::Accessor,
    buffer_data: &Vec<Vec<u8>>,
    custom_vertex_attributes: &HashMap<Box<str>, MeshVertexAttribute>,
    convert_coordinates: bool,
) -> Result<(MeshVertexAttribute, Values), ConvertAttributeError> {
    if let Some((attribute, conversion, convert_coordinates)) = match &semantic {
        gltf::Semantic::Positions => Some((
            Mesh::ATTRIBUTE_POSITION,
            ConversionMode::Float,
            convert_coordinates,
        )),
        gltf::Semantic::Normals => Some((
            Mesh::ATTRIBUTE_NORMAL,
            ConversionMode::Direction,
            convert_coordinates,
        )),
        gltf::Semantic::Tangents => Some((
            Mesh::ATTRIBUTE_TANGENT,
            ConversionMode::Direction,
            convert_coordinates,
        )),
        gltf::Semantic::Colors(0) => Some((Mesh::ATTRIBUTE_COLOR, ConversionMode::Rgba, false)),
        gltf::Semantic::TexCoords(0) => Some((Mesh::ATTRIBUTE_UV_0, ConversionMode::Float, false)),
        gltf::Semantic::TexCoords(1) => Some((Mesh::ATTRIBUTE_UV_1, ConversionMode::Float, false)),
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
            ConversionMode::Float => iter.into_float_values(convert_coordinates),
            ConversionMode::Direction => iter.into_direction_values(convert_coordinates),
            ConversionMode::Rgba => iter.into_rgba_values(),
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

/// The primitive's `POSITION` bounds in the units and coordinate system of its loaded positions.
///
/// glTF stores the bounds of an integer accessor as integers, before
/// normalization.
pub(crate) fn position_bounds(
    primitive: &gltf::Primitive,
    convert_coordinates: bool,
) -> (Vec3, Vec3) {
    let bounds = primitive.bounding_box();
    // `bounding_box` already requires the accessor.
    let positions = primitive.get(&gltf::Semantic::Positions).unwrap();
    let normalized = positions.normalized();
    let dequantize = |bound: [f32; 3]| {
        let bound = Vec3::from(bound.map(|c| match positions.data_type() {
            DataType::I8 => (c as i8).dequantize(normalized),
            DataType::U8 => (c as u8).dequantize(normalized),
            DataType::I16 => (c as i16).dequantize(normalized),
            DataType::U16 => (c as u16).dequantize(normalized),
            _ => c,
        }));
        maybe_convert(bound, convert_coordinates)
    };
    let (a, b) = (dequantize(bounds.min), dequantize(bounds.max));
    (a.min(b), a.max(b))
}

fn compressed_positions<T: Dequantize>(
    it: impl Iterator<Item = [T; 3]>,
    normalization: Normalization,
    convert: bool,
    aabb: Aabb3d,
) -> Vec<[i16; 4]> {
    let positions = dequantize(it, normalization).map(|p| Vec3A::from(maybe_convert(p, convert)));
    encode_compressed_positions(positions, aabb)
}

fn compressed_normals<T: Dequantize>(
    it: impl Iterator<Item = [T; 3]>,
    normalization: Normalization,
    convert: bool,
) -> Vec<[i16; 2]> {
    encode_compressed_normals(
        dequantize(it, normalization).map(|n| Vec3::from(maybe_convert(n, convert))),
    )
}

fn compressed_tangents<T: Dequantize>(
    it: impl Iterator<Item = [T; 4]>,
    normalization: Normalization,
    convert: bool,
) -> Vec<[i16; 2]> {
    encode_compressed_tangents(
        dequantize(it, normalization).map(|t| Vec4::from(maybe_convert(t, convert))),
    )
}

/// The texture coordinates remapped onto `0..=u16::MAX` without loss, and the range that maps
/// back to their dequantized values.
fn compressed_uvs<T: Dequantize>(
    it: impl Iterator<Item = [T; 2]>,
    Normalization(normalized): Normalization,
) -> (Vec<[u16; 2]>, Aabb2d) {
    let scale = if normalized { 1.0 / T::MAX as f32 } else { 1.0 };
    let range = Aabb2d {
        min: Vec2::splat(T::MIN as f32 * scale),
        max: Vec2::splat(T::MAX as f32 * scale),
    };
    (it.map(|uv| uv.map(T::to_unorm16)).collect(), range)
}

/// Inserts a quantized `POSITION`, `NORMAL`, `TANGENT` or `TEXCOORD_0/1` accessor into `mesh` in
/// the compressed format [`MeshAttributeCompressionFlags`] describes, encoded straight from the
/// accessor.
///
/// Texture coordinates keep every bit of their source. Returns `Ok(false)` without changing `mesh`
/// for float accessors, other semantics, and formats `KHR_mesh_quantization` doesn't allow.
pub(crate) fn insert_quantized_attribute(
    mesh: &mut Mesh,
    semantic: &gltf::Semantic,
    accessor: &gltf::Accessor,
    primitive: &gltf::Primitive,
    buffer_data: &Vec<Vec<u8>>,
    convert: bool,
) -> Result<bool, ConvertAttributeError> {
    use VertexAttributeIter as It;

    if accessor.data_type() == DataType::F32 {
        return Ok(false);
    }
    let iter = VertexAttributeIter::from_accessor(accessor.clone(), buffer_data)
        .map_err(|err| ConvertAttributeError::AccessFailed(err, accessor.index()))?;
    match semantic {
        gltf::Semantic::Positions => {
            let (min, max) = position_bounds(primitive, convert);
            let aabb = Aabb3d {
                min: min.into(),
                max: max.into(),
            };
            let positions = match iter {
                It::S8x3(it, n) => compressed_positions(it, n, convert, aabb),
                It::U8x3(it, n) => compressed_positions(it, n, convert, aabb),
                It::S16x3(it, n) => compressed_positions(it, n, convert, aabb),
                It::U16x3(it, n) => compressed_positions(it, n, convert, aabb),
                _ => return Ok(false),
            };
            mesh.insert_compressed_positions(positions, aabb);
        }
        gltf::Semantic::Normals => {
            let normals = match iter {
                It::S8x3(it, n @ Normalization(true)) => compressed_normals(it, n, convert),
                It::S16x3(it, n @ Normalization(true)) => compressed_normals(it, n, convert),
                _ => return Ok(false),
            };
            mesh.insert_compressed_normals(normals);
        }
        gltf::Semantic::Tangents => {
            let tangents = match iter {
                It::S8x4(it, n @ Normalization(true)) => compressed_tangents(it, n, convert),
                It::S16x4(it, n @ Normalization(true)) => compressed_tangents(it, n, convert),
                _ => return Ok(false),
            };
            mesh.insert_compressed_tangents(tangents);
        }
        gltf::Semantic::TexCoords(channel @ (0 | 1)) => {
            let (uvs, range) = match iter {
                It::S8x2(it, n) => compressed_uvs(it, n),
                It::U8x2(it, n) => compressed_uvs(it, n),
                It::S16x2(it, n) => compressed_uvs(it, n),
                It::U16x2(it, n) => compressed_uvs(it, n),
                _ => return Ok(false),
            };
            let channel = if *channel == 0 {
                UvChannel::Uv0
            } else {
                UvChannel::Uv1
            };
            mesh.insert_compressed_uvs(channel, uvs, range);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// The compression flags of the primitive's quantized attributes.
///
/// Each compressed format has at least the precision `KHR_mesh_quantization`
/// allows for its attribute.
pub(crate) fn quantized_attribute_compression(
    primitive: &gltf::Primitive,
) -> MeshAttributeCompressionFlags {
    primitive
        .attributes()
        .filter(|(_, accessor)| accessor.data_type() != DataType::F32)
        .filter_map(|(semantic, _)| match semantic {
            gltf::Semantic::Positions => Some(MeshAttributeCompressionFlags::COMPRESS_POSITION),
            gltf::Semantic::Normals => Some(MeshAttributeCompressionFlags::COMPRESS_NORMAL),
            gltf::Semantic::Tangents => Some(MeshAttributeCompressionFlags::COMPRESS_TANGENT),
            gltf::Semantic::TexCoords(0) => Some(MeshAttributeCompressionFlags::COMPRESS_UV0),
            gltf::Semantic::TexCoords(1) => Some(MeshAttributeCompressionFlags::COMPRESS_UV1),
            _ => None,
        })
        .collect()
}

/// The per-vertex displacements of each of the primitive's morph targets, in order.
pub(crate) fn morph_targets(
    primitive: &gltf::Primitive,
    buffer_data: &Vec<Vec<u8>>,
    convert_coordinates: bool,
) -> Result<Vec<MorphAttributes>, ConvertAttributeError> {
    fn read<'a>(
        accessor: Option<gltf::Accessor<'a>>,
        buffer_data: &'a Vec<Vec<u8>>,
        into: impl FnOnce(VertexAttributeIter<'a>) -> Result<Vec<[f32; 3]>, AccessFailed>,
    ) -> Result<Vec<[f32; 3]>, ConvertAttributeError> {
        let Some(accessor) = accessor else {
            return Ok(Vec::new());
        };
        // An accessor without a buffer view or sparse values is all zeros.
        if accessor.view().is_none() && accessor.sparse().is_none() {
            return Ok(vec![[0.0; 3]; accessor.count()]);
        }
        let index = accessor.index();
        VertexAttributeIter::from_accessor(accessor, buffer_data)
            .and_then(into)
            .map_err(|err| ConvertAttributeError::AccessFailed(err, index))
    }

    let mut attributes = Vec::new();
    for target in primitive.morph_targets() {
        let positions = read(target.positions(), buffer_data, |iter| {
            iter.into_displacements(convert_coordinates)
        })?;
        let normals = read(target.normals(), buffer_data, |iter| {
            iter.into_direction_displacements(convert_coordinates)
        })?;
        let tangents = read(target.tangents(), buffer_data, |iter| {
            iter.into_direction_displacements(convert_coordinates)
        })?;

        let count = positions.len().max(normals.len()).max(tangents.len());
        let at = |values: &[[f32; 3]], i: usize| values.get(i).map_or(Vec3::ZERO, |&v| v.into());
        attributes.extend((0..count).map(|i| {
            MorphAttributes::from([at(&positions, i), at(&normals, i), at(&tangents, i)])
        }));
    }
    Ok(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &[u8]) -> gltf::Gltf {
        gltf::Gltf::from_slice_without_validation(json).unwrap()
    }

    #[test]
    fn rejects_normals_outside_khr_mesh_quantization() {
        let gltf = parse(
            br#"
{
    "asset": { "version": "2.0" },
    "buffers": [{ "byteLength": 4 }],
    "bufferViews": [{ "buffer": 0, "byteLength": 4 }],
    "accessors": [{
        "bufferView": 0, "componentType": 5121, "normalized": true, "count": 1, "type": "VEC3"
    }]
}
"#,
        );
        let normals = convert_attribute(
            gltf::Semantic::Normals,
            gltf.accessors().next().unwrap(),
            &vec![vec![0; 4]],
            &HashMap::default(),
            false,
        );
        assert!(normals.is_err());
    }

    #[test]
    fn dequantizes_normalized_position_bounds() {
        let gltf = parse(
            br#"
{
    "asset": { "version": "2.0" },
    "accessors": [{
        "componentType": 5123, "normalized": true, "count": 1, "type": "VEC3",
        "min": [0, 0, 0], "max": [65535, 0, 0]
    }],
    "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }]
}
"#,
        );
        let primitive = gltf.meshes().next().unwrap().primitives().next().unwrap();
        assert_eq!(position_bounds(&primitive, false), (Vec3::ZERO, Vec3::X));
    }

    /// An accessor without a buffer view reads as zeros, even when it is a
    /// target's only attribute.
    #[test]
    fn reads_float_and_zero_morph_targets() {
        let gltf = parse(
            br#"
{
    "asset": { "version": "2.0" },
    "buffers": [{ "byteLength": 12 }],
    "bufferViews": [{ "buffer": 0, "byteLength": 12 }],
    "accessors": [
        { "bufferView": 0, "componentType": 5126, "count": 1, "type": "VEC3" },
        { "componentType": 5126, "count": 1, "type": "VEC3" }
    ],
    "meshes": [{ "primitives": [{
        "attributes": {},
        "targets": [{ "POSITION": 0, "NORMAL": 1 }, { "NORMAL": 1 }]
    }] }]
}
"#,
        );
        let buffer = [1.0f32, 2.0, 3.0].map(f32::to_le_bytes).concat();
        let primitive = gltf.meshes().next().unwrap().primitives().next().unwrap();
        assert_eq!(
            morph_targets(&primitive, &vec![buffer], true).unwrap(),
            vec![
                MorphAttributes::from([Vec3::new(-1.0, 2.0, -3.0), Vec3::ZERO, Vec3::ZERO]),
                MorphAttributes::from([Vec3::ZERO; 3]),
            ]
        );
    }
}
