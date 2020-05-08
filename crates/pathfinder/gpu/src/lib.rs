// pathfinder/gpu/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Minimal abstractions over GPU device capabilities.

#[macro_use]
extern crate bitflags;

use half::f16;
use image::ImageFormat;
use pathfinder_color::ColorF;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform3d::Transform4F;
use pathfinder_geometry::vector::{Vector2I, vec2i};
use pathfinder_resources::ResourceLoader;
use pathfinder_simd::default::{F32x2, F32x4, I32x2};
use std::os::raw::c_void;
use std::time::Duration;

pub trait Device: Sized {
    type Buffer;
    type Framebuffer;
    type Program;
    type Shader;
    type Texture;
    type TextureDataReceiver;
    type TimerQuery;
    type Uniform;
    type VertexArray;
    type VertexAttr;

    fn create_texture(&self, format: TextureFormat, size: Vector2I) -> Self::Texture;
    fn create_texture_from_data(&self, format: TextureFormat, size: Vector2I, data: TextureDataRef)
                                -> Self::Texture;
    fn create_shader(&self, resources: &dyn ResourceLoader, name: &str, kind: ShaderKind)
                     -> Self::Shader;
    fn create_shader_from_source(&self, name: &str, source: &[u8], kind: ShaderKind)
                                 -> Self::Shader;
    fn create_vertex_array(&self) -> Self::VertexArray;
    fn create_program_from_shaders(
        &self,
        resources: &dyn ResourceLoader,
        name: &str,
        vertex_shader: Self::Shader,
        fragment_shader: Self::Shader,
    ) -> Self::Program;
    fn get_vertex_attr(&self, program: &Self::Program, name: &str) -> Option<Self::VertexAttr>;
    fn get_uniform(&self, program: &Self::Program, name: &str) -> Self::Uniform;
    fn bind_buffer(&self,
                   vertex_array: &Self::VertexArray,
                   buffer: &Self::Buffer,
                   target: BufferTarget);
    fn configure_vertex_attr(&self,
                             vertex_array: &Self::VertexArray,
                             attr: &Self::VertexAttr,
                             descriptor: &VertexAttrDescriptor);
    fn create_framebuffer(&self, texture: Self::Texture) -> Self::Framebuffer;
    fn create_buffer(&self) -> Self::Buffer;
    fn allocate_buffer<T>(
        &self,
        buffer: &Self::Buffer,
        data: BufferData<T>,
        target: BufferTarget,
        mode: BufferUploadMode,
    );
    fn framebuffer_texture<'f>(&self, framebuffer: &'f Self::Framebuffer) -> &'f Self::Texture;
    fn destroy_framebuffer(&self, framebuffer: Self::Framebuffer) -> Self::Texture;
    fn texture_format(&self, texture: &Self::Texture) -> TextureFormat;
    fn texture_size(&self, texture: &Self::Texture) -> Vector2I;
    fn set_texture_sampling_mode(&self, texture: &Self::Texture, flags: TextureSamplingFlags);
    fn upload_to_texture(&self, texture: &Self::Texture, rect: RectI, data: TextureDataRef);
    fn read_pixels(&self, target: &RenderTarget<Self>, viewport: RectI)
                   -> Self::TextureDataReceiver;
    fn begin_commands(&self);
    fn end_commands(&self);
    fn draw_arrays(&self, index_count: u32, render_state: &RenderState<Self>);
    fn draw_elements(&self, index_count: u32, render_state: &RenderState<Self>);
    fn draw_elements_instanced(&self,
                               index_count: u32,
                               instance_count: u32,
                               render_state: &RenderState<Self>);
    fn create_timer_query(&self) -> Self::TimerQuery;
    fn begin_timer_query(&self, query: &Self::TimerQuery);
    fn end_timer_query(&self, query: &Self::TimerQuery);
    fn try_recv_timer_query(&self, query: &Self::TimerQuery) -> Option<Duration>;
    fn recv_timer_query(&self, query: &Self::TimerQuery) -> Duration;
    fn try_recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> Option<TextureData>;
    fn recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> TextureData;

    fn create_texture_from_png(&self, resources: &dyn ResourceLoader, name: &str) -> Self::Texture {
        let data = resources.slurp(&format!("textures/{}.png", name)).unwrap();
        let image = image::load_from_memory_with_format(&data, ImageFormat::Png)
            .unwrap()
            .to_luma();
        let size = vec2i(image.width() as i32, image.height() as i32);
        self.create_texture_from_data(TextureFormat::R8, size, TextureDataRef::U8(&image))
    }

    fn create_program_from_shader_names(
        &self,
        resources: &dyn ResourceLoader,
        program_name: &str,
        vertex_shader_name: &str,
        fragment_shader_name: &str,
    ) -> Self::Program {
        let vertex_shader = self.create_shader(resources, vertex_shader_name, ShaderKind::Vertex);
        let fragment_shader =
            self.create_shader(resources, fragment_shader_name, ShaderKind::Fragment);
        self.create_program_from_shaders(resources, program_name, vertex_shader, fragment_shader)
    }

    fn create_program(&self, resources: &dyn ResourceLoader, name: &str) -> Self::Program {
        self.create_program_from_shader_names(resources, name, name, name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextureFormat {
    R8,
    R16F,
    RGBA8,
    RGBA16F,
    RGBA32F,
}

#[derive(Clone, Copy, Debug)]
pub enum VertexAttrType {
    F32,
    I16,
    I8,
    U16,
    U8,
}

#[derive(Clone, Copy, Debug)]
pub enum BufferData<'a, T> {
    Uninitialized(usize),
    Memory(&'a [T]),
}

#[derive(Clone, Copy, Debug)]
pub enum BufferTarget {
    Vertex,
    Index,
}

#[derive(Clone, Copy, Debug)]
pub enum BufferUploadMode {
    Static,
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderKind {
    Vertex,
    Fragment,
}

#[derive(Clone, Copy)]
pub enum UniformData {
    Float(f32),
    IVec2(I32x2),
    IVec3([i32; 3]),
    Int(i32),
    Mat2(F32x4),
    Mat4([F32x4; 4]),
    Vec2(F32x2),
    Vec3([f32; 3]),
    Vec4(F32x4),
    TextureUnit(u32),
}

#[derive(Clone, Copy)]
pub enum Primitive {
    Triangles,
    Lines,
}

#[derive(Clone)]
pub struct RenderState<'a, D> where D: Device {
    pub target: &'a RenderTarget<'a, D>,
    pub program: &'a D::Program,
    pub vertex_array: &'a D::VertexArray,
    pub primitive: Primitive,
    pub uniforms: &'a [(&'a D::Uniform, UniformData)],
    pub textures: &'a [&'a D::Texture],
    pub viewport: RectI,
    pub options: RenderOptions,
}

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub blend: Option<BlendState>,
    pub depth: Option<DepthState>,
    pub stencil: Option<StencilState>,
    pub clear_ops: ClearOps,
    pub color_mask: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClearOps {
    pub color: Option<ColorF>,
    pub depth: Option<f32>,
    pub stencil: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub enum RenderTarget<'a, D> where D: Device {
    Default,
    Framebuffer(&'a D::Framebuffer),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendState {
    pub dest_rgb_factor: BlendFactor,
    pub dest_alpha_factor: BlendFactor,
    pub src_rgb_factor: BlendFactor,
    pub src_alpha_factor: BlendFactor,
    pub op: BlendOp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendFactor {
    Zero,
    One,
    SrcAlpha,
    OneMinusSrcAlpha,
    DestAlpha,
    OneMinusDestAlpha,
    DestColor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct DepthState {
    pub func: DepthFunc,
    pub write: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum DepthFunc {
    Less,
    Always,
}

#[derive(Clone, Copy, Debug)]
pub struct StencilState {
    pub func: StencilFunc,
    pub reference: u32,
    pub mask: u32,
    pub write: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum StencilFunc {
    Always,
    Equal,
}

impl Default for RenderOptions {
    #[inline]
    fn default() -> RenderOptions {
        RenderOptions {
            blend: None,
            depth: None,
            stencil: None,
            clear_ops: ClearOps::default(),
            color_mask: true,
        }
    }
}

impl Default for BlendOp {
    #[inline]
    fn default() -> BlendOp {
        BlendOp::Add
    }
}

impl Default for StencilState {
    #[inline]
    fn default() -> StencilState {
        StencilState {
            func: StencilFunc::default(),
            reference: 0,
            mask: !0,
            write: false,
        }
    }
}

impl Default for DepthFunc {
    #[inline]
    fn default() -> DepthFunc {
        DepthFunc::Less
    }
}

impl Default for StencilFunc {
    #[inline]
    fn default() -> StencilFunc {
        StencilFunc::Always
    }
}

#[derive(Clone, Debug)]
pub enum TextureData {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F16(Vec<f16>),
    F32(Vec<f32>),
}

#[derive(Clone, Copy, Debug)]
pub enum TextureDataRef<'a> {
    U8(&'a [u8]),
    F16(&'a [f16]),
    F32(&'a [f32]),
}

impl UniformData {
    #[inline]
    pub fn from_transform_3d(transform: &Transform4F) -> UniformData {
        UniformData::Mat4([transform.c0, transform.c1, transform.c2, transform.c3])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VertexAttrDescriptor {
    pub size: usize,
    pub class: VertexAttrClass,
    pub attr_type: VertexAttrType,
    pub stride: usize,
    pub offset: usize,
    pub divisor: u32,
    pub buffer_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexAttrClass {
    Float,
    FloatNorm,
    Int,
}

impl TextureFormat {
    #[inline]
    pub fn channels(self) -> usize {
        match self {
            TextureFormat::R8 | TextureFormat::R16F => 1,
            TextureFormat::RGBA8 | TextureFormat::RGBA16F | TextureFormat::RGBA32F => 4,
        }
    }

    #[inline]
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            TextureFormat::R8 => 1,
            TextureFormat::R16F => 2,
            TextureFormat::RGBA8 => 4,
            TextureFormat::RGBA16F => 8,
            TextureFormat::RGBA32F => 16,
        }
    }
}

impl ClearOps {
    #[inline]
    pub fn has_ops(&self) -> bool {
        self.color.is_some() || self.depth.is_some() || self.stencil.is_some()
    }
}

impl Default for BlendState {
    #[inline]
    fn default() -> BlendState {
        BlendState {
            src_rgb_factor: BlendFactor::One,
            dest_rgb_factor: BlendFactor::OneMinusSrcAlpha,
            src_alpha_factor: BlendFactor::One,
            dest_alpha_factor: BlendFactor::One,
            op: BlendOp::Add,
        }
    }
}

bitflags! {
    pub struct TextureSamplingFlags: u8 {
        const REPEAT_U    = 0x01;
        const REPEAT_V    = 0x02;
        const NEAREST_MIN = 0x04;
        const NEAREST_MAG = 0x08;
    }
}

impl<'a> TextureDataRef<'a> {
    #[doc(hidden)]
    pub fn check_and_extract_data_ptr(self, minimum_size: Vector2I, format: TextureFormat)
                                      -> *const c_void {
        let channels = match (format, self) {
            (TextureFormat::R8, TextureDataRef::U8(_)) => 1,
            (TextureFormat::RGBA8, TextureDataRef::U8(_)) => 4,
            (TextureFormat::RGBA16F, TextureDataRef::F16(_)) => 4,
            (TextureFormat::RGBA32F, TextureDataRef::F32(_)) => 4,
            _ => panic!("Unimplemented texture format!"),
        };

        let area = minimum_size.x() as usize * minimum_size.y() as usize;

        match self {
            TextureDataRef::U8(data) => {
                assert!(data.len() >= area * channels);
                data.as_ptr() as *const c_void
            }
            TextureDataRef::F16(data) => {
                assert!(data.len() >= area * channels);
                data.as_ptr() as *const c_void
            }
            TextureDataRef::F32(data) => {
                assert!(data.len() >= area * channels);
                data.as_ptr() as *const c_void
            }
        }
    }
}
