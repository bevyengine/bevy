use crate::shaders;
use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    pipeline::{PipelineDescriptor, VertexBufferDescriptor},
    render_resource::{BufferInfo, BufferUsage, RenderResource},
    renderer::{RenderContext, RenderResourceContext},
    shader::{Shader, ShaderSource, ShaderStage, ShaderStages},
    texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage},
};
use pathfinder_canvas::vec2i;
use pathfinder_geometry::{rect::RectI, vector::Vector2I};
use pathfinder_gpu::{
    BufferData, BufferTarget, BufferUploadMode, Device, RenderState, RenderTarget, ShaderKind,
    TextureData, TextureDataRef, TextureSamplingFlags, VertexAttrDescriptor,
};
use pathfinder_resources::ResourceLoader;
use std::{cell::RefCell, mem, rc::Rc, time::Duration};
use zerocopy::AsBytes;

pub struct BevyPathfinderDevice<'a> {
    render_context: RefCell<&'a mut dyn RenderContext>,
    shaders: RefCell<&'a mut AssetStorage<Shader>>,
}

impl<'a> BevyPathfinderDevice<'a> {
    pub fn new(render_context: &'a mut dyn RenderContext, shaders: &'a mut AssetStorage<Shader>) -> Self {
        BevyPathfinderDevice {
            render_context: RefCell::new(render_context),
            shaders: RefCell::new(shaders),
        }
    }
}

pub struct BevyTimerQuery {}
pub struct BevyTextureDataReceiver {}
pub struct BevyUniform {
    name: String,
}

#[derive(Debug)]
pub struct BevyVertexArray {
    descriptor: (),
    vertex_buffers: RefCell<Vec<RenderResource>>,
    index_buffer: RefCell<Option<RenderResource>>,
}

#[derive(Debug)]
pub struct BevyVertexAttr {
    name: String,
    bind_location: u32,
}

pub struct BevyTexture {
    handle: RenderResource,
    descriptor: TextureDescriptor,
}

pub struct BevyBuffer {
    handle: Rc<RefCell<Option<RenderResource>>>,
}

impl<'a> Device for BevyPathfinderDevice<'a> {
    type Buffer = BevyBuffer;
    type Framebuffer = RenderResource;
    type Program = PipelineDescriptor;
    type Shader = Handle<Shader>;
    type Texture = BevyTexture;
    type TextureDataReceiver = BevyTextureDataReceiver;
    type TimerQuery = BevyTimerQuery;
    type Uniform = BevyUniform;
    type VertexArray = BevyVertexArray;
    type VertexAttr = BevyVertexAttr;
    fn create_texture(
        &self,
        format: pathfinder_gpu::TextureFormat,
        size: Vector2I,
    ) -> Self::Texture {
        let descriptor = TextureDescriptor {
            size: Extent3d {
                depth: 1,
                width: size.x() as u32,
                height: size.y() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: match format {
                pathfinder_gpu::TextureFormat::R8 => TextureFormat::R8Unorm,
                pathfinder_gpu::TextureFormat::R16F => TextureFormat::R16Float,
                pathfinder_gpu::TextureFormat::RGBA8 => TextureFormat::Rgba8Unorm,
                pathfinder_gpu::TextureFormat::RGBA16F => TextureFormat::Rgba16Float,
                pathfinder_gpu::TextureFormat::RGBA32F => TextureFormat::Rgba32Float,
            },
            usage: TextureUsage::WRITE_ALL, // TODO: this might be overly safe
        };
        BevyTexture {
            handle: self
                .render_context
                .borrow()
                .resources()
                .create_texture(&descriptor),
            descriptor,
        }
    }
    fn create_texture_from_data(
        &self,
        format: pathfinder_gpu::TextureFormat,
        size: Vector2I,
        data: TextureDataRef,
    ) -> Self::Texture {
        let texture = self.create_texture(format, size);
        self.upload_to_texture(&texture, RectI::new(Vector2I::default(), size), data);
        texture
    }
    fn create_shader(
        &self,
        _resources: &dyn ResourceLoader,
        name: &str,
        kind: ShaderKind,
    ) -> Self::Shader {
        let shader_bytes = match (name, kind) {
            ("blit", ShaderKind::Fragment) => shaders::BLIT_FS,
            ("blit", ShaderKind::Vertex) => shaders::BLIT_VS,
            // ("debug_solid", ShaderKind::Fragment) => shaders::DEMO_GROUND_FS,
            // ("demo_ground", ShaderKind::Vertex) => shaders::DEMO_GROUND_VS,
            ("fill", ShaderKind::Fragment) => shaders::FILL_FS,
            ("fill", ShaderKind::Vertex) => shaders::FILL_VS,
            ("reproject", ShaderKind::Fragment) => shaders::REPROJECT_FS,
            ("reproject", ShaderKind::Vertex) => shaders::REPROJECT_VS,
            ("stencil", ShaderKind::Fragment) => shaders::STENCIL_FS,
            ("stencil", ShaderKind::Vertex) => shaders::STENCIL_VS,
            ("tile_clip", ShaderKind::Fragment) => shaders::TILE_CLIP_FS,
            ("tile_clip", ShaderKind::Vertex) => shaders::TILE_CLIP_VS,
            ("tile_copy", ShaderKind::Fragment) => shaders::TILE_COPY_FS,
            ("tile_copy", ShaderKind::Vertex) => shaders::TILE_COPY_VS,
            ("tile", ShaderKind::Fragment) => shaders::TILE_FS,
            ("tile", ShaderKind::Vertex) => shaders::TILE_VS,
            _ => panic!("encountered unexpected shader {} {:?}", name, kind),
        };

        self.create_shader_from_source(name, shader_bytes, kind)
    }
    fn create_shader_from_source(
        &self,
        _name: &str,
        source: &[u8],
        kind: ShaderKind,
    ) -> Self::Shader {
        let stage = match kind {
            ShaderKind::Fragment => ShaderStage::Fragment,
            ShaderKind::Vertex => ShaderStage::Vertex,
        };
        let shader = Shader::new(stage, ShaderSource::spirv_from_bytes(source));
        let mut shaders = self.shaders.borrow_mut();
        let handle = shaders.add(shader);
        self.render_context
            .borrow()
            .resources()
            .create_shader_module(handle, &mut shaders);
        handle
    }

    fn create_vertex_array(&self) -> Self::VertexArray {
        BevyVertexArray {
            descriptor: (),
            index_buffer: RefCell::new(None),
            vertex_buffers: RefCell::new(Vec::new()),
        }
    }
    fn create_program_from_shaders(
        &self,
        _resources: &dyn ResourceLoader,
        name: &str,
        vertex_shader: Self::Shader,
        fragment_shader: Self::Shader,
    ) -> Self::Program {
        println!("{}", name);
        let mut descriptor = PipelineDescriptor::new(ShaderStages {
            vertex: vertex_shader,
            fragment: Some(fragment_shader),
        });
        descriptor.reflect_layout(&self.shaders.borrow(), false, None, None);
        descriptor
    }
    fn get_vertex_attr(
        &self,
        descriptor: &PipelineDescriptor,
        name: &str,
    ) -> Option<Self::VertexAttr> {
        let layout = descriptor.get_layout().unwrap();
        panic!("{:?}", layout);
    }
    fn get_uniform(&self, _program: &PipelineDescriptor, name: &str) -> Self::Uniform {
        BevyUniform {
            name: name.to_string(),
        }
    }
    fn bind_buffer(
        &self,
        vertex_array: &Self::VertexArray,
        buffer: &Self::Buffer,
        target: BufferTarget,
    ) {
        match target {
            BufferTarget::Vertex => vertex_array
                .vertex_buffers
                .borrow_mut()
                .push(buffer.handle.borrow().unwrap().clone()),
            BufferTarget::Index => {
                *vertex_array.index_buffer.borrow_mut() = Some(buffer.handle.borrow().unwrap().clone())
            }
        }
    }
    fn configure_vertex_attr(
        &self,
        vertex_array: &Self::VertexArray,
        attr: &Self::VertexAttr,
        descriptor: &VertexAttrDescriptor,
    ) {
        todo!()
    }
    fn create_framebuffer(&self, texture: Self::Texture) -> Self::Framebuffer {
        todo!()
    }
    fn create_buffer(&self) -> Self::Buffer {
        BevyBuffer {
            handle: Rc::new(RefCell::new(None)),
        }
    }
    fn allocate_buffer<T>(
        &self,
        buffer: &BevyBuffer,
        data: BufferData<T>,
        _target: BufferTarget,
        mode: BufferUploadMode,
    ) {
        let buffer_usage = match mode {
            BufferUploadMode::Dynamic => BufferUsage::WRITE_ALL,
            BufferUploadMode::Static => BufferUsage::COPY_DST,
        };
        // TODO: use mod
        match data {
            BufferData::Uninitialized(size) => {
                let size = size * mem::size_of::<T>();
                let new_buffer =
                    self.render_context
                        .borrow()
                        .resources()
                        .create_buffer(BufferInfo {
                            size,
                            buffer_usage,
                            ..Default::default()
                        });
                *buffer.handle.borrow_mut() = Some(new_buffer);
            }
            BufferData::Memory(slice) => {
                let size = slice.len() * mem::size_of::<T>();
                let new_buffer =
                    self.render_context
                        .borrow()
                        .resources()
                        .create_buffer_with_data(BufferInfo {
                            size,
                            buffer_usage,
                            ..Default::default()
                        }, slice_to_u8(slice));
                *buffer.handle.borrow_mut() = Some(new_buffer);
            }
        }
    }
    fn framebuffer_texture<'f>(&self, framebuffer: &'f Self::Framebuffer) -> &'f Self::Texture {
        todo!()
    }
    fn destroy_framebuffer(&self, framebuffer: Self::Framebuffer) -> Self::Texture {
        todo!()
    }
    fn texture_format(&self, texture: &BevyTexture) -> pathfinder_gpu::TextureFormat {
        match texture.descriptor.format {
            TextureFormat::R8Unorm => pathfinder_gpu::TextureFormat::R8,
            TextureFormat::R16Float => pathfinder_gpu::TextureFormat::R16F,
            TextureFormat::Rgba8Unorm => pathfinder_gpu::TextureFormat::RGBA8,
            TextureFormat::Rgba16Float => pathfinder_gpu::TextureFormat::RGBA16F,
            TextureFormat::Rgba32Float => pathfinder_gpu::TextureFormat::RGBA32F,
            _ => panic!("unexpected texture format {:?}", texture.descriptor.format),
        }
    }
    fn texture_size(&self, texture: &BevyTexture) -> Vector2I {
        vec2i(
            texture.descriptor.size.width as i32,
            texture.descriptor.size.height as i32,
        )
    }
    fn set_texture_sampling_mode(&self, texture: &Self::Texture, flags: TextureSamplingFlags) {
        todo!()
    }
    fn upload_to_texture(&self, texture: &BevyTexture, rect: RectI, data: TextureDataRef) {
        let texture_size = self.texture_size(texture);
        assert!(rect.size().x() >= 0);
        assert!(rect.size().y() >= 0);
        assert!(rect.max_x() <= texture_size.x());
        assert!(rect.max_y() <= texture_size.y());

        let format = self.texture_format(&texture);
        let width = rect.size().x() as u32;
        let height = rect.size().y() as u32;
        let origin = [rect.origin().x() as u32, rect.origin().y() as u32, 0];
        let bytes_per_pixel = format.bytes_per_pixel() as u32;
        let size = (width * height * bytes_per_pixel) as usize;

        let staging_buffer = self
            .render_context
            .borrow()
            .resources()
            .create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::COPY_SRC,
                    size,
                    ..Default::default()
                },
                get_texture_bytes(&data),
            );

        let stride = bytes_per_pixel * width;

        self.render_context.borrow_mut().copy_buffer_to_texture(
            staging_buffer,
            0,
            stride,
            texture.handle,
            origin,
            0,
            0,
            Extent3d {
                width,
                height,
                depth: 1,
            },
        )
    }
    fn read_pixels(
        &self,
        target: &RenderTarget<Self>,
        viewport: RectI,
    ) -> Self::TextureDataReceiver {
        // TODO: this might actually be optional, which is great because otherwise this requires a command buffer sync
        todo!()
    }
    fn begin_commands(&self) {
        // TODO: maybe not needed?
        // todo!()
    }
    fn end_commands(&self) {
        // TODO: maybe not needed?
        // todo!()
    }
    fn draw_arrays(&self, index_count: u32, render_state: &RenderState<Self>) {
        todo!()
    }
    fn draw_elements(&self, index_count: u32, render_state: &RenderState<Self>) {
        todo!()
    }
    fn draw_elements_instanced(
        &self,
        index_count: u32,
        instance_count: u32,
        render_state: &RenderState<Self>,
    ) {
        todo!()
    }
    fn create_timer_query(&self) -> Self::TimerQuery {
        // TODO: maybe not needed
        BevyTimerQuery {}
    }
    fn begin_timer_query(&self, query: &Self::TimerQuery) {
        // TODO: maybe not needed
        todo!()
    }
    fn end_timer_query(&self, query: &Self::TimerQuery) {
        // TODO: maybe not needed
        todo!()
    }
    fn try_recv_timer_query(&self, query: &Self::TimerQuery) -> Option<std::time::Duration> {
        // TODO: maybe not needed
        None
    }
    fn recv_timer_query(&self, query: &Self::TimerQuery) -> std::time::Duration {
        // TODO: maybe not needed
        Duration::from_millis(0)
    }
    fn try_recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> Option<TextureData> {
        // TODO: maybe not needed
        None
    }
    fn recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> TextureData {
        // TODO: maybe not needed
        todo!()
    }
}

fn get_texture_bytes<'a>(data_ref: &'a TextureDataRef) -> &'a [u8] {
    match data_ref {
        TextureDataRef::U8(data) => data,
        TextureDataRef::F16(data) => {
            panic!("we dont do half measures");
        }
        TextureDataRef::F32(data) => data.as_bytes(),
    }
}

fn slice_to_u8<T>(slice: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * mem::size_of::<T>(),
        )
    }
}