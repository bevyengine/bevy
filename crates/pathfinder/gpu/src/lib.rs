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
    type Fence;
    type Framebuffer;
    type Program;
    type Shader;
    type StorageBuffer;
    type Texture;
    type TextureDataReceiver;
    type TimerQuery;
    type Uniform;
    type VertexArray;
    type VertexAttr;

    fn feature_level(&self) -> FeatureLevel;
    fn create_texture(&self, format: TextureFormat, size: Vector2I) -> Self::Texture;
    fn create_texture_from_data(&self, format: TextureFormat, size: Vector2I, data: TextureDataRef)
                                -> Self::Texture;
    fn create_shader(&self, resources: &dyn ResourceLoader, name: &str, kind: ShaderKind)
                     -> Self::Shader;
    fn create_shader_from_source(&self, name: &str, source: &[u8], kind: ShaderKind)
                                 -> Self::Shader;
    fn create_vertex_array(&self) -> Self::VertexArray;
    fn create_program_from_shaders(&self,
                                   resources: &dyn ResourceLoader,
                                   name: &str,
                                   shaders: ProgramKind<Self::Shader>)
                                   -> Self::Program;
    fn set_compute_program_local_size(&self,
                                      program: &mut Self::Program,
                                      local_size: ComputeDimensions);
    fn get_vertex_attr(&self, program: &Self::Program, name: &str) -> Option<Self::VertexAttr>;
    fn get_uniform(&self, program: &Self::Program, name: &str) -> Self::Uniform;
    fn get_storage_buffer(&self, program: &Self::Program, name: &str, binding: u32)
                          -> Self::StorageBuffer;
    fn bind_buffer(&self,
                   vertex_array: &Self::VertexArray,
                   buffer: &Self::Buffer,
                   target: BufferTarget);
    fn configure_vertex_attr(&self,
                             vertex_array: &Self::VertexArray,
                             attr: &Self::VertexAttr,
                             descriptor: &VertexAttrDescriptor);
    fn create_framebuffer(&self, texture: Self::Texture) -> Self::Framebuffer;
    fn create_buffer(&self, mode: BufferUploadMode) -> Self::Buffer;
    fn allocate_buffer<T>(&self,
                          buffer: &Self::Buffer,
                          data: BufferData<T>,
                          target: BufferTarget);
    fn upload_to_buffer<T>(&self,
                           buffer: &Self::Buffer,
                           position: usize,
                           data: &[T],
                           target: BufferTarget);
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
    fn dispatch_compute(&self, dimensions: ComputeDimensions, state: &ComputeState<Self>);
    fn add_fence(&self) -> Self::Fence;
    fn wait_for_fence(&self, fence: &Self::Fence);
    fn create_timer_query(&self) -> Self::TimerQuery;
    fn begin_timer_query(&self, query: &Self::TimerQuery);
    fn end_timer_query(&self, query: &Self::TimerQuery);
    fn try_recv_timer_query(&self, query: &Self::TimerQuery) -> Option<Duration>;
    fn recv_timer_query(&self, query: &Self::TimerQuery) -> Duration;
    fn try_recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> Option<TextureData>;
    fn recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> TextureData;

    fn create_texture_from_png(&self,
                               resources: &dyn ResourceLoader,
                               name: &str,
                               format: TextureFormat)
                               -> Self::Texture {
        let data = resources.slurp(&format!("textures/{}.png", name)).unwrap();
        let image = image::load_from_memory_with_format(&data, ImageFormat::Png).unwrap();
        match format {
            TextureFormat::R8 => {
                let image = image.to_luma();
                let size = vec2i(image.width() as i32, image.height() as i32);
                self.create_texture_from_data(format, size, TextureDataRef::U8(&image))
            }
            TextureFormat::RGBA8 => {
                let image = image.to_rgba();
                let size = vec2i(image.width() as i32, image.height() as i32);
                self.create_texture_from_data(format, size, TextureDataRef::U8(&image))
            }
            _ => unimplemented!(),
        }
    }

    fn create_program_from_shader_names(
        &self,
        resources: &dyn ResourceLoader,
        program_name: &str,
        shader_names: ProgramKind<&str>,
    ) -> Self::Program {
        let shaders = match shader_names {
            ProgramKind::Raster { vertex, fragment } => {
                ProgramKind::Raster {
                    vertex: self.create_shader(resources, vertex, ShaderKind::Vertex),
                    fragment: self.create_shader(resources, fragment, ShaderKind::Fragment),
                }
            }
            ProgramKind::Compute(compute) => {
                ProgramKind::Compute(self.create_shader(resources, compute, ShaderKind::Compute))
            }
        };
        self.create_program_from_shaders(resources, program_name, shaders)
    }

    fn create_raster_program(&self, resources: &dyn ResourceLoader, name: &str) -> Self::Program {
        let shaders = ProgramKind::Raster { vertex: name, fragment: name };
        self.create_program_from_shader_names(resources, name, shaders)
    }

    fn create_compute_program(&self, resources: &dyn ResourceLoader, name: &str) -> Self::Program {
        let shaders = ProgramKind::Compute(name);
        self.create_program_from_shader_names(resources, name, shaders)
    }
}

/// These are rough analogues to D3D versions; don't expect them to represent exactly the feature
/// set of the versions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FeatureLevel {
    D3D10,
    D3D11,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextureFormat {
    R8,
    R16F,
    RGBA8,
    RGBA16F,
    RGBA32F,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexAttrType {
    F32,
    I32,
    I16,
    I8,
    U32,
    U16,
    U8,
}

impl VertexAttrType {
    pub fn get_size(&self) -> usize {
        match *self {
            VertexAttrType::F32 => 4,
            VertexAttrType::I32 => 4,
            VertexAttrType::I16 => 2,
            VertexAttrType::I8 => 1,
            VertexAttrType::U32 => 4,
            VertexAttrType::U16 => 2,
            VertexAttrType::U8 => 1,
        }
    }
}

#[cfg(feature = "shader_alignment_32_bits")]
pub const ALIGNED_U8_ATTR: VertexAttrType = VertexAttrType::U32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub const ALIGNED_U8_ATTR: VertexAttrType = VertexAttrType::U8;

#[cfg(feature = "shader_alignment_32_bits")]
pub const ALIGNED_U16_ATTR: VertexAttrType = VertexAttrType::U32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub const ALIGNED_U16_ATTR: VertexAttrType = VertexAttrType::U16;

#[cfg(feature = "shader_alignment_32_bits")]
pub const ALIGNED_I8_ATTR: VertexAttrType = VertexAttrType::I32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub const ALIGNED_I8_ATTR: VertexAttrType = VertexAttrType::I8;

#[cfg(feature = "shader_alignment_32_bits")]
pub const ALIGNED_I16_ATTR: VertexAttrType = VertexAttrType::I32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub const ALIGNED_I16_ATTR: VertexAttrType = VertexAttrType::I16;

#[derive(Clone, Copy, Debug)]
pub enum BufferData<'a, T> {
    Uninitialized(usize),
    Memory(&'a [T]),
}

#[derive(Clone, Copy, Debug)]
pub enum BufferTarget {
    Vertex,
    Index,
    Storage,
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
    Compute,
}

#[derive(Clone, Copy, Debug)]
pub enum ProgramKind<T> {
    Raster {
        vertex: T,
        fragment: T,
    },
    Compute(T),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputeDimensions {
    pub x: u32,
    pub y: u32,
    pub z: u32,
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
    ImageUnit(u32),
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
    pub images: &'a [ImageBinding<'a, D>],
    pub viewport: RectI,
    pub options: RenderOptions,
}

#[derive(Clone)]
pub struct ComputeState<'a, D> where D: Device {
    pub program: &'a D::Program,
    pub uniforms: &'a [(&'a D::Uniform, UniformData)],
    pub textures: &'a [&'a D::Texture],
    pub images: &'a [ImageBinding<'a, D>],
    pub storage_buffers: &'a [(&'a D::StorageBuffer, &'a D::Buffer)],
}

#[derive(Clone, Debug)]
pub struct ImageBinding<'a, D> where D: Device {
    pub texture: &'a D::Texture,
    pub access: ImageAccess,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexAttrDescriptor {
    pub size: usize,
    pub class: VertexAttrClass,
    pub attr_type: VertexAttrType,
    pub stride: usize,
    pub offset: usize,
    pub divisor: u32,
    pub buffer_index: u32,
}

impl VertexAttrDescriptor {
    pub const fn datatype_only(class: VertexAttrClass, attr_type: VertexAttrType, size: usize) -> Self {
        VertexAttrDescriptor {
            size,
            class,
            attr_type,
            divisor: 0,
            buffer_index: 0,
            stride: 0,
            offset: 0,
        }
    }
}

pub struct VertexBufferDescriptor {
    pub index: u32,
    pub divisor: u32,
    pub vertex_attrs: Vec<VertexAttrDescriptor>,
}

impl VertexBufferDescriptor {
    pub fn update_attrs(&mut self) {
        let mut offset = 0;
        for attr in self.vertex_attrs.iter_mut() {
            attr.buffer_index = self.index;
            attr.divisor = self.divisor;
            attr.offset = offset;
            offset += attr.size * attr.attr_type.get_size();
        }

        for attr in self.vertex_attrs.iter_mut() {
            attr.stride = offset;
        }
    }

    pub fn configure_vertex_attrs<D: Device>(&self, device: &D, vertex_array: &D::VertexArray, attrs: &[D::VertexAttr]) {
        for (attr, descriptor) in attrs.iter().zip(self.vertex_attrs.iter()) {
            device.configure_vertex_attr(vertex_array, attr, &descriptor);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageAccess {
    Read,
    Write,
    ReadWrite,
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
