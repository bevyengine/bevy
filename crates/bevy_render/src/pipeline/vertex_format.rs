use crate::Color;
use bevy_math::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum VertexFormat {
    Uchar2 = 1,
    Uchar4 = 3,
    Char2 = 5,
    Char4 = 7,
    Uchar2Norm = 9,
    Uchar4Norm = 11,
    Char2Norm = 14,
    Char4Norm = 16,
    Ushort2 = 18,
    Ushort4 = 20,
    Short2 = 22,
    Short4 = 24,
    Ushort2Norm = 26,
    Ushort4Norm = 28,
    Short2Norm = 30,
    Short4Norm = 32,
    Half2 = 34,
    Half4 = 36,
    Float = 37,
    Float2 = 38,
    Float3 = 39,
    Float4 = 40,
    Uint = 41,
    Uint2 = 42,
    Uint3 = 43,
    Uint4 = 44,
    Int = 45,
    Int2 = 46,
    Int3 = 47,
    Int4 = 48,
}

impl VertexFormat {
    pub fn get_size(&self) -> u64 {
        match *self {
            VertexFormat::Uchar2 => 2,
            VertexFormat::Uchar4 => 4,
            VertexFormat::Char2 => 2,
            VertexFormat::Char4 => 4,
            VertexFormat::Uchar2Norm => 2,
            VertexFormat::Uchar4Norm => 4,
            VertexFormat::Char2Norm => 2,
            VertexFormat::Char4Norm => 4,
            VertexFormat::Ushort2 => 2 * 2,
            VertexFormat::Ushort4 => 2 * 4,
            VertexFormat::Short2 => 2 * 2,
            VertexFormat::Short4 => 2 * 4,
            VertexFormat::Ushort2Norm => 2 * 2,
            VertexFormat::Ushort4Norm => 2 * 4,
            VertexFormat::Short2Norm => 2 * 2,
            VertexFormat::Short4Norm => 2 * 4,
            VertexFormat::Half2 => 2 * 2,
            VertexFormat::Half4 => 2 * 4,
            VertexFormat::Float => 4,
            VertexFormat::Float2 => 4 * 2,
            VertexFormat::Float3 => 4 * 3,
            VertexFormat::Float4 => 4 * 4,
            VertexFormat::Uint => 4,
            VertexFormat::Uint2 => 4 * 2,
            VertexFormat::Uint3 => 4 * 3,
            VertexFormat::Uint4 => 4 * 4,
            VertexFormat::Int => 4,
            VertexFormat::Int2 => 4 * 2,
            VertexFormat::Int3 => 4 * 3,
            VertexFormat::Int4 => 4 * 4,
        }
    }
}

pub trait AsVertexFormats {
    fn as_vertex_formats() -> &'static [VertexFormat];
}

impl AsVertexFormats for f32 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float]
    }
}

impl AsVertexFormats for Vec2 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float2]
    }
}

impl AsVertexFormats for Vec3 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float3]
    }
}

impl AsVertexFormats for Vec4 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float4]
    }
}

impl AsVertexFormats for Mat4 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[
            VertexFormat::Float4,
            VertexFormat::Float4,
            VertexFormat::Float4,
            VertexFormat::Float4,
        ]
    }
}

impl AsVertexFormats for Color {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float4]
    }
}

impl AsVertexFormats for [f32; 2] {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float2]
    }
}

impl AsVertexFormats for [f32; 3] {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float3]
    }
}

impl AsVertexFormats for [f32; 4] {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float4]
    }
}
