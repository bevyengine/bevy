use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    pipeline::{
        state_descriptors::CompareFunction, InputStepMode, PipelineDescriptor,
        VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat,
    },
    render_resource::{BufferInfo, BufferUsage, RenderResource},
    renderer::RenderContext,
    shader::{Shader, ShaderSource, ShaderStage, ShaderStages},
    texture::{
        AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsage,
    },
};
use pathfinder_canvas::vec2i;
use pathfinder_geometry::{rect::RectI, vector::Vector2I};
use pathfinder_gpu::{
    BufferData, BufferTarget, BufferUploadMode, Device, ProgramKind, RenderState, RenderTarget,
    ShaderKind, TextureData, TextureDataRef, TextureSamplingFlags, VertexAttrClass,
    VertexAttrDescriptor, VertexAttrType, FeatureLevel,
};
use pathfinder_resources::ResourceLoader;
use std::{borrow::Cow, cell::RefCell, collections::HashMap, mem, rc::Rc, time::Duration};
use zerocopy::AsBytes;

pub struct BevyPathfinderDevice<'a> {
    render_context: RefCell<&'a mut dyn RenderContext>,
    shaders: RefCell<&'a mut AssetStorage<Shader>>,
    samplers: RefCell<HashMap<u8, RenderResource>>,
}

impl<'a> BevyPathfinderDevice<'a> {
    pub fn new(
        render_context: &'a mut dyn RenderContext,
        shaders: &'a mut AssetStorage<Shader>,
    ) -> Self {
        BevyPathfinderDevice {
            render_context: RefCell::new(render_context),
            shaders: RefCell::new(shaders),
            samplers: RefCell::new(HashMap::new()),
        }
    }
}

pub struct BevyTimerQuery {}
pub struct BevyTextureDataReceiver {}
pub struct BevyUniform {
    pub name: String,
}

pub struct BevyVertexAttr {
    attr: RefCell<VertexAttributeDescriptor>,
}

#[derive(Debug)]
pub struct BevyVertexArray {
    requested_descriptors: RefCell<HashMap<u32, VertexBufferDescriptor>>,
    vertex_buffers: RefCell<Vec<BevyBuffer>>,
    index_buffer: RefCell<Option<BevyBuffer>>,
}

pub struct BevyTexture {
    handle: RenderResource,
    texture_descriptor: TextureDescriptor,
    sampler_resource: RefCell<Option<RenderResource>>,
}

#[derive(Debug, Clone)]
pub struct BevyBuffer {
    handle: Rc<RefCell<Option<RenderResource>>>,
    mode: BufferUploadMode,
}

impl<'a> Device for BevyPathfinderDevice<'a> {
    type Buffer = BevyBuffer;
    type Fence = ();
    type Framebuffer = BevyTexture;
    type Program = PipelineDescriptor;
    type Shader = Handle<Shader>;
    type StorageBuffer = ();
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
            texture_descriptor: descriptor,
            sampler_resource: RefCell::new(None),
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
        resources: &dyn ResourceLoader,
        name: &str,
        kind: ShaderKind,
    ) -> Self::Shader {
        // let shader_bytes = match (name, kind) {
        //     ("blit", ShaderKind::Fragment) => shaders::BLIT_FS,
        //     ("blit", ShaderKind::Vertex) => shaders::BLIT_VS,
        //     // ("debug_solid", ShaderKind::Fragment) => shaders::DEMO_GROUND_FS,
        //     // ("demo_ground", ShaderKind::Vertex) => shaders::DEMO_GROUND_VS,
        //     ("fill", ShaderKind::Fragment) => shaders::FILL_FS,
        //     ("fill", ShaderKind::Vertex) => shaders::FILL_VS,
        //     ("reproject", ShaderKind::Fragment) => shaders::REPROJECT_FS,
        //     ("reproject", ShaderKind::Vertex) => shaders::REPROJECT_VS,
        //     ("stencil", ShaderKind::Fragment) => shaders::STENCIL_FS,
        //     ("stencil", ShaderKind::Vertex) => shaders::STENCIL_VS,
        //     ("tile_clip", ShaderKind::Fragment) => shaders::TILE_CLIP_FS,
        //     ("tile_clip", ShaderKind::Vertex) => shaders::TILE_CLIP_VS,
        //     ("tile_copy", ShaderKind::Fragment) => shaders::TILE_COPY_FS,
        //     ("tile_copy", ShaderKind::Vertex) => shaders::TILE_COPY_VS,
        //     ("tile", ShaderKind::Fragment) => shaders::TILE_FS,
        //     ("tile", ShaderKind::Vertex) => shaders::TILE_VS,
        //     _ => panic!("encountered unexpected shader {} {:?}", name, kind),
        // };
        let suffix = match kind {
            ShaderKind::Vertex => 'v',
            ShaderKind::Fragment => 'f',
            ShaderKind::Compute => 'c',
        };
        let path = format!("shaders/vulkan/{}.{}s.spv", name, suffix);
        let bytes = resources.slurp(&path).unwrap();

        self.create_shader_from_source(name, &bytes, kind)
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
            ShaderKind::Compute => panic!("bevy does not currently support compute shaders"),
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
            requested_descriptors: RefCell::new(HashMap::new()),
            index_buffer: RefCell::new(None),
            vertex_buffers: RefCell::new(Vec::new()),
        }
    }
    fn create_program_from_shaders(
        &self,
        _resources: &dyn ResourceLoader,
        _name: &str,
        shaders: ProgramKind<Handle<Shader>>,
    ) -> Self::Program {
        match shaders {
            ProgramKind::Compute(_) => panic!("bevy does not currently support compute shaders"),
            ProgramKind::Raster { vertex, fragment } => {
                let mut descriptor = PipelineDescriptor::new(ShaderStages {
                    vertex,
                    fragment: Some(fragment),
                });
                descriptor.reflect_layout(&self.shaders.borrow(), false, None, None);
                descriptor
            }
        }
    }
    fn get_vertex_attr(
        &self,
        descriptor: &PipelineDescriptor,
        name: &str,
    ) -> Option<Self::VertexAttr> {
        // TODO: this probably isn't actually used for anything. try to optimize
        let layout = descriptor.get_layout().unwrap();
        let attribute_name = format!("a{}", name);
        for buffer_descriptor in layout.vertex_buffer_descriptors.iter() {
            let attribute = buffer_descriptor
                .attributes
                .iter()
                .find(|a| a.name == attribute_name)
                .cloned();
            if attribute.is_some() {
                return attribute.map(|a| BevyVertexAttr {
                    attr: RefCell::new(a),
                });
            }
        }

        // // TODO: remove this
        // panic!("failed to find attribute {} ", attribute_name);

        None
    }
    fn get_uniform(&self, _program: &PipelineDescriptor, name: &str) -> Self::Uniform {
        BevyUniform {
            name: name.to_string(),
        }
    }
    fn bind_buffer(
        &self,
        vertex_array: &BevyVertexArray,
        buffer: &BevyBuffer,
        target: BufferTarget,
    ) {
        match target {
            BufferTarget::Vertex => vertex_array
                .vertex_buffers
                .borrow_mut()
                .push(buffer.clone()),
            BufferTarget::Index => {
                *vertex_array.index_buffer.borrow_mut() =
                    Some(buffer.clone())
            }
            _ => panic!("Buffers bound to vertex arrays must be vertex or index buffers!"),
        }
    }
    fn configure_vertex_attr(
        &self,
        vertex_array: &BevyVertexArray,
        bevy_attr: &BevyVertexAttr,
        descriptor: &VertexAttrDescriptor,
    ) {
        let format = match (descriptor.class, descriptor.attr_type, descriptor.size) {
            (VertexAttrClass::Int, VertexAttrType::I8, 2) => VertexFormat::Char2,
            // (VertexAttrClass::Int, VertexAttrType::I8, 3) => VertexFormat::Char3,
            (VertexAttrClass::Int, VertexAttrType::I8, 4) => VertexFormat::Char4,
            (VertexAttrClass::Int, VertexAttrType::U8, 2) => VertexFormat::Uchar2,
            // (VertexAttrClass::Int, VertexAttrType::U8, 3) => VertexFormat::Uchar3,
            (VertexAttrClass::Int, VertexAttrType::U8, 4) => VertexFormat::Uchar4,
            (VertexAttrClass::FloatNorm, VertexAttrType::U8, 2) => VertexFormat::Uchar2Norm,
            // (VertexAttrClass::FloatNorm, VertexAttrType::U8, 3) => {
            //     VertexFormat::UChar3Normalized
            // }
            (VertexAttrClass::FloatNorm, VertexAttrType::U8, 4) => VertexFormat::Uchar4Norm,
            (VertexAttrClass::FloatNorm, VertexAttrType::I8, 2) => VertexFormat::Char2Norm,
            // (VertexAttrClass::FloatNorm, VertexAttrType::I8, 3) => {
            //     VertexFormat::Char3Norm
            // }
            (VertexAttrClass::FloatNorm, VertexAttrType::I8, 4) => VertexFormat::Char4Norm,
            (VertexAttrClass::Int, VertexAttrType::I16, 2) => VertexFormat::Short2,
            // (VertexAttrClass::Int, VertexAttrType::I16, 3) => VertexFormat::Short3,
            (VertexAttrClass::Int, VertexAttrType::I16, 4) => VertexFormat::Short4,
            (VertexAttrClass::Int, VertexAttrType::U16, 2) => VertexFormat::Ushort2,
            // (VertexAttrClass::Int, VertexAttrType::U16, 3) => VertexFormat::UShort3,
            (VertexAttrClass::Int, VertexAttrType::U16, 4) => VertexFormat::Ushort4,
            (VertexAttrClass::FloatNorm, VertexAttrType::U16, 2) => VertexFormat::Ushort2Norm,
            // (VertexAttrClass::FloatNorm, VertexAttrType::U16, 3) => {
            //     VertexFormat::UShort3Normalized
            // }
            (VertexAttrClass::FloatNorm, VertexAttrType::U16, 4) => VertexFormat::Ushort4Norm,
            (VertexAttrClass::FloatNorm, VertexAttrType::I16, 2) => VertexFormat::Short2Norm,
            // (VertexAttrClass::FloatNorm, VertexAttrType::I16, 3) => {
            //     VertexFormat::Short3Normalized
            // }
            (VertexAttrClass::FloatNorm, VertexAttrType::I16, 4) => VertexFormat::Short4Norm,
            (VertexAttrClass::Int, VertexAttrType::I32, 1) => VertexFormat::Int,
            (VertexAttrClass::Int, VertexAttrType::I32, 2) => VertexFormat::Int2,
            (VertexAttrClass::Int, VertexAttrType::I32, 3) => VertexFormat::Int3,
            (VertexAttrClass::Int, VertexAttrType::I32, 4) => VertexFormat::Int4,
            (VertexAttrClass::Int, VertexAttrType::U32, 1) => VertexFormat::Uint,
            (VertexAttrClass::Int, VertexAttrType::U32, 2) => VertexFormat::Uint2,
            (VertexAttrClass::Int, VertexAttrType::U32, 3) => VertexFormat::Uint3,
            (VertexAttrClass::Int, VertexAttrType::U32, 4) => VertexFormat::Uint4,
            (VertexAttrClass::Float, VertexAttrType::F32, 1) => VertexFormat::Float,
            (VertexAttrClass::Float, VertexAttrType::F32, 2) => VertexFormat::Float2,
            (VertexAttrClass::Float, VertexAttrType::F32, 3) => VertexFormat::Float3,
            (VertexAttrClass::Float, VertexAttrType::F32, 4) => VertexFormat::Float4,
            // (VertexAttrClass::Int, VertexAttrType::I8, 1) => VertexFormat::Char,
            // (VertexAttrClass::Int, VertexAttrType::U8, 1) => VertexFormat::UChar,
            // (VertexAttrClass::FloatNorm, VertexAttrType::I8, 1) => VertexFormat::CharNormalized,
            // (VertexAttrClass::FloatNorm, VertexAttrType::U8, 1) => {
            //     VertexFormat::UCharNormalized
            // }
            // (VertexAttrClass::Int, VertexAttrType::I16, 1) => VertexFormat::Short,
            // (VertexAttrClass::Int, VertexAttrType::U16, 1) => VertexFormat::UShort,
            // (VertexAttrClass::FloatNorm, VertexAttrType::U16, 1) => {
            //     VertexFormat::UShortNormalized
            // }
            // (VertexAttrClass::FloatNorm, VertexAttrType::I16, 1) => {
            //     VertexFormat::ShortNormalized
            // }
            (attr_class, attr_type, attr_size) => panic!(
                "Unsupported vertex class/type/size combination: {:?}/{:?}/{}!",
                attr_class, attr_type, attr_size
            ),
        };

        let mut requested_descriptors = vertex_array.requested_descriptors.borrow_mut();
        let buffer_index = descriptor.buffer_index;
        let step_mode = if descriptor.divisor == 0 {
            InputStepMode::Vertex
        } else {
            InputStepMode::Instance
        };

        assert!(
            descriptor.divisor <= 1,
            "instanced step size greater than 1 not supported"
        );

        let vertex_buffer_descriptor =
            requested_descriptors
                .entry(buffer_index)
                .or_insert_with(|| VertexBufferDescriptor {
                    name: Cow::Borrowed("placeholder"),
                    attributes: Vec::new(),
                    step_mode,
                    stride: descriptor.stride as u64,
                });

        let mut attr = bevy_attr.attr.borrow_mut();
        attr.format = format;
        attr.offset = descriptor.offset as u64;

        vertex_buffer_descriptor.attributes.push(attr.clone());
    }

    fn create_framebuffer(&self, texture: BevyTexture) -> BevyTexture {
        texture
    }

    fn create_buffer(&self, mode: BufferUploadMode) -> Self::Buffer {
        BevyBuffer {
            handle: Rc::new(RefCell::new(None)),
            mode,
        }
    }

    fn allocate_buffer<T>(&self, buffer: &BevyBuffer, data: BufferData<T>, _target: BufferTarget) {
        let buffer_usage = match buffer.mode {
            BufferUploadMode::Dynamic => BufferUsage::WRITE_ALL,
            BufferUploadMode::Static => BufferUsage::COPY_DST,
        };
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
                let new_buffer = self
                    .render_context
                    .borrow()
                    .resources()
                    .create_buffer_with_data(
                        BufferInfo {
                            size,
                            buffer_usage,
                            ..Default::default()
                        },
                        slice_to_u8(slice),
                    );
                *buffer.handle.borrow_mut() = Some(new_buffer);
            }
        }
    }

    fn framebuffer_texture<'f>(&self, framebuffer: &'f BevyTexture) -> &'f BevyTexture {
        framebuffer
    }

    fn destroy_framebuffer(&self, framebuffer: BevyTexture) -> BevyTexture {
        // TODO: should this deallocate the bevy texture?
        framebuffer
    }

    fn texture_format(&self, texture: &BevyTexture) -> pathfinder_gpu::TextureFormat {
        match texture.texture_descriptor.format {
            TextureFormat::R8Unorm => pathfinder_gpu::TextureFormat::R8,
            TextureFormat::R16Float => pathfinder_gpu::TextureFormat::R16F,
            TextureFormat::Rgba8Unorm => pathfinder_gpu::TextureFormat::RGBA8,
            TextureFormat::Rgba16Float => pathfinder_gpu::TextureFormat::RGBA16F,
            TextureFormat::Rgba32Float => pathfinder_gpu::TextureFormat::RGBA32F,
            _ => panic!(
                "unexpected texture format {:?}",
                texture.texture_descriptor.format
            ),
        }
    }

    fn texture_size(&self, texture: &BevyTexture) -> Vector2I {
        vec2i(
            texture.texture_descriptor.size.width as i32,
            texture.texture_descriptor.size.height as i32,
        )
    }

    fn set_texture_sampling_mode(&self, texture: &BevyTexture, flags: TextureSamplingFlags) {
        let mut samplers = self.samplers.borrow_mut();
        let resource = samplers.entry(flags.bits()).or_insert_with(|| {
            let descriptor = SamplerDescriptor {
                address_mode_u: if flags.contains(TextureSamplingFlags::REPEAT_U) {
                    AddressMode::Repeat
                } else {
                    AddressMode::ClampToEdge
                },
                address_mode_v: if flags.contains(TextureSamplingFlags::REPEAT_V) {
                    AddressMode::Repeat
                } else {
                    AddressMode::ClampToEdge
                },
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: if flags.contains(TextureSamplingFlags::NEAREST_MAG) {
                    FilterMode::Nearest
                } else {
                    FilterMode::Linear
                },
                min_filter: if flags.contains(TextureSamplingFlags::NEAREST_MIN) {
                    FilterMode::Nearest
                } else {
                    FilterMode::Linear
                },
                mipmap_filter: FilterMode::Nearest,
                lod_min_clamp: -100.0,
                lod_max_clamp: 100.0,
                compare_function: CompareFunction::Always,
            };
            self.render_context
                .borrow_mut()
                .resources()
                .create_sampler(&descriptor)
        });
        *texture.sampler_resource.borrow_mut() = Some(*resource);
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
        _target: &RenderTarget<Self>,
        _viewport: RectI,
    ) -> Self::TextureDataReceiver {
        // TODO: this might actually be optional, which is great because otherwise this requires a command buffer sync
        todo!()
    }
    fn begin_commands(&self) {
        // NOTE: the Bevy Render Graph handles command buffer creation
    }
    fn end_commands(&self) {
        // NOTE: the Bevy Render Graph handles command buffer submission
    }
    fn draw_arrays(&self, _index_count: u32, _render_state: &RenderState<Self>) {
        todo!()
    }
    fn draw_elements(&self, _index_count: u32, _render_state: &RenderState<Self>) {
        todo!()
    }
    fn draw_elements_instanced(
        &self,
        _index_count: u32,
        _instance_count: u32,
        _render_state: &RenderState<Self>,
    ) {
        todo!()
    }
    fn create_timer_query(&self) -> Self::TimerQuery {
        // TODO: maybe not needed
        BevyTimerQuery {}
    }
    fn begin_timer_query(&self, _query: &Self::TimerQuery) {}
    fn end_timer_query(&self, _query: &Self::TimerQuery) {}
    fn try_recv_timer_query(&self, _query: &Self::TimerQuery) -> Option<std::time::Duration> {
        None
    }
    fn recv_timer_query(&self, _query: &Self::TimerQuery) -> std::time::Duration {
        Duration::from_millis(0)
    }
    fn try_recv_texture_data(&self, _receiver: &Self::TextureDataReceiver) -> Option<TextureData> {
        None
    }
    fn recv_texture_data(&self, _receiver: &Self::TextureDataReceiver) -> TextureData {
        todo!()
    }
    fn feature_level(&self) -> pathfinder_gpu::FeatureLevel {
        // TODO: change to 11 when compute is added
        FeatureLevel::D3D10
    }
    fn set_compute_program_local_size(
        &self,
        _program: &mut Self::Program,
        _local_size: pathfinder_gpu::ComputeDimensions,
    ) {
    }
    fn get_storage_buffer(
        &self,
        _program: &Self::Program,
        _name: &str,
        _binding: u32,
    ) -> Self::StorageBuffer {
        panic!("Compute shader is unsupported in Bevy!");
    }
    fn upload_to_buffer<T>(
        &self,
        buffer: &BevyBuffer,
        position: usize,
        data: &[T],
        _target: BufferTarget,
    ) {
        let data_slice = &data[position..];
        let temp_buffer = self
            .render_context
            .borrow()
            .resources()
            .create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::COPY_DST,
                    ..Default::default()
                },
                slice_to_u8(data_slice),
            );
        let buffer_handle = buffer.handle.borrow().unwrap();
        self.render_context.borrow_mut().copy_buffer_to_buffer(
            temp_buffer,
            0,
            buffer_handle,
            0,
            data_slice.len() as u64,
        )
    }
    fn dispatch_compute(
        &self,
        _dimensions: pathfinder_gpu::ComputeDimensions,
        _state: &pathfinder_gpu::ComputeState<Self>,
    ) {
        panic!("Compute shader is unsupported in Bevy!");
    }
    fn add_fence(&self) -> Self::Fence {}
    fn wait_for_fence(&self, _fence: &Self::Fence) {}
}

fn get_texture_bytes<'a>(data_ref: &'a TextureDataRef) -> &'a [u8] {
    match data_ref {
        TextureDataRef::U8(data) => data,
        TextureDataRef::F16(data) => {
            slice_to_u8(data)
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
