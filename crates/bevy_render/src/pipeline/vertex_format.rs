use crate::Color;
use bevy_math::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum VertexFormat {
    Uint8x2 = 1,
    Uint8x4 = 3,
    Sint8x2 = 5,
    Sint8x4 = 7,
    Unorm8x2 = 9,
    Unorm8x4 = 11,
    Snorm8x2 = 14,
    Snorm8x4 = 16,
    Uint16x2 = 18,
    Uint16x4 = 20,
    Sint16x2 = 22,
    Sint16x4 = 24,
    Unorm16x2 = 26,
    Unorm16x4 = 28,
    Snorm16x2 = 30,
    Snorm16x4 = 32,
    Float16x2 = 34,
    Float16x4 = 36,
    Float32 = 37,
    Float32x2 = 38,
    Float32x3 = 39,
    Float32x4 = 40,
    Uint32 = 41,
    Uint32x2 = 42,
    Uint32x3 = 43,
    Uint32x4 = 44,
    Sint32 = 45,
    Sint32x2 = 46,
    Sint32x3 = 47,
    Sint32x4 = 48,
}

impl VertexFormat {
    pub fn get_size(&self) -> u64 {
        match *self {
            VertexFormat::Uint8x2 => 2,
            VertexFormat::Uint8x4 => 4,
            VertexFormat::Sint8x2 => 2,
            VertexFormat::Sint8x4 => 4,
            VertexFormat::Unorm8x2 => 2,
            VertexFormat::Unorm8x4 => 4,
            VertexFormat::Snorm8x2 => 2,
            VertexFormat::Snorm8x4 => 4,
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
        }
    }
}

pub trait AsVertexFormats {
    fn as_vertex_formats() -> &'static [VertexFormat];
}

impl AsVertexFormats for f32 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32]
    }
}

impl AsVertexFormats for Vec2 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x2]
    }
}

impl AsVertexFormats for Vec3 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x3]
    }
}

impl AsVertexFormats for Vec4 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x4]
    }
}

impl AsVertexFormats for Mat4 {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[
            VertexFormat::Float32x4,
            VertexFormat::Float32x4,
            VertexFormat::Float32x4,
            VertexFormat::Float32x4,
        ]
    }
}

impl AsVertexFormats for Color {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x4]
    }
}

impl AsVertexFormats for [f32; 2] {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x2]
    }
}

impl AsVertexFormats for [f32; 3] {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x3]
    }
}

impl AsVertexFormats for [f32; 4] {
    fn as_vertex_formats() -> &'static [VertexFormat] {
        &[VertexFormat::Float32x4]
    }
}
