use pathfinder_geometry::{rect::RectI, vector::Vector2I};
use pathfinder_gpu::{
    BufferData, BufferTarget, BufferUploadMode, Device, RenderState, RenderTarget, ShaderKind,
    TextureData, TextureDataRef, TextureFormat, TextureSamplingFlags, VertexAttrDescriptor,
};
use pathfinder_resources::ResourceLoader;
use std::cell::Cell;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::{collections::HashMap, mem, rc::Rc, slice, time::Duration};

#[derive(Debug)]
pub struct WebGpuDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,

    /// This encoder is "finished" and inserted into `command_buffers` and replaced with a new one on `begin_commands` and `end_frame`. This ensures that there always is a current `command_encoder`.
    ///
    /// Note: maybe this is too "clever" and will cause bugs down the road. A more naive alternative would be `RefCell<Option<wgpu::CommandEncoder>>` and only having this be set between `begin_commands` and `end_commands`. Any additional work done by this implementation internally will just push new command buffers if this field is `None`.
    current_command_encoder: RefCell<wgpu::CommandEncoder>,
    command_buffers: RefCell<Vec<wgpu::CommandBuffer>>,
    main_depth_stencil_texture: wgpu::Texture,
    samplers: Vec<wgpu::Sampler>,
    swap_chain: wgpu::SwapChain,
    swap_chain_output: wgpu::SwapChainOutput,
    // From metal backend
    // layer: CoreAnimationLayer,
    // shared_event: SharedEvent,
    // shared_event_listener: SharedEventListener,
    // next_timer_query_event_value: Cell<u64>,
}

impl WebGpuDevice {
    pub fn new(window: impl raw_window_handle::HasRawWindowHandle, size: Vector2I) -> Self {
        futures::executor::block_on(async {
            let surface = wgpu::Surface::create(&window);

            let adapter = wgpu::Adapter::request(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
                },
                wgpu::BackendBit::PRIMARY,
            )
            .await
            .unwrap();

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    extensions: wgpu::Extensions {
                        anisotropic_filtering: false,
                    },
                    limits: wgpu::Limits::default(),
                })
                .await;

            let samplers = (0..16)
                .map(|sampling_flags_value| {
                    let sampling_flags =
                        TextureSamplingFlags::from_bits(sampling_flags_value).unwrap();

                    device.create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: if sampling_flags.contains(TextureSamplingFlags::REPEAT_U) {
                            wgpu::AddressMode::Repeat
                        } else {
                            wgpu::AddressMode::ClampToEdge
                        },
                        address_mode_v: if sampling_flags.contains(TextureSamplingFlags::REPEAT_V) {
                            wgpu::AddressMode::Repeat
                        } else {
                            wgpu::AddressMode::ClampToEdge
                        },
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: if sampling_flags.contains(TextureSamplingFlags::NEAREST_MAG) {
                            wgpu::FilterMode::Nearest
                        } else {
                            wgpu::FilterMode::Linear
                        },
                        min_filter: if sampling_flags.contains(TextureSamplingFlags::NEAREST_MIN) {
                            wgpu::FilterMode::Nearest
                        } else {
                            wgpu::FilterMode::Linear
                        },
                        mipmap_filter: wgpu::FilterMode::Nearest, // "Not mipmapped"
                        lod_min_clamp: 0.0,
                        lod_max_clamp: std::f32::MAX,
                        compare: wgpu::CompareFunction::Never,
                    })
                })
                .collect();

            let main_depth_stencil_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("main depth texture"),
                size: wgpu::Extent3d {
                    width: size.x().expect_unsigned(),
                    height: size.y().expect_unsigned(),
                    depth: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            });

            let mut swap_chain = device.create_swap_chain(
                &surface,
                &wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    width: size.x().expect_unsigned(),
                    height: size.y().expect_unsigned(),
                    present_mode: wgpu::PresentMode::Fifo,
                },
            );

            let swap_chain_output = swap_chain.get_next_texture().unwrap();

            // A (potentially empty) command encoder to catch any internal backend work that happens before the first user call to `begin_commands`.
            let initialization_command_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("init encoder"),
                });

            WebGpuDevice {
                device,
                queue,
                samplers,
                swap_chain,
                swap_chain_output,
                command_buffers: RefCell::new(Vec::new()),
                main_depth_stencil_texture,
                current_command_encoder: RefCell::new(initialization_command_encoder),
            }
        })
    }

    /// Finishes the current command encoder and pushes it onto the frame's command buffers. Creates a new "current" command encoder that new commands should write to.
    fn finish_current_command_encoder(&self) {
        let next_command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("init encoder"),
                });
        let prev_command_encoder = self.current_command_encoder.replace(next_command_encoder);
        self.command_buffers
            .borrow_mut()
            .push(prev_command_encoder.finish());
    }

    fn borrow_current_command_encoder(&self) -> std::cell::RefMut<wgpu::CommandEncoder> {
        self.current_command_encoder.borrow_mut()
    }

    pub fn end_frame(&mut self) {
        self.finish_current_command_encoder();
        self.queue.submit(self.command_buffers.borrow().as_slice());
        self.command_buffers.borrow_mut().clear();
        self.swap_chain_output = self.swap_chain.get_next_texture().unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct WebGpuBuffer {
    /// Lazily initialized
    inner: Rc<RefCell<Option<wgpu::Buffer>>>,
}

#[derive(Debug)]
pub struct WebGpuFramebuffer {}

#[derive(Debug)]
pub struct WebGpuProgram {
    name: Option<String>,
    vertex_shader: WebGpuShader,
    fragment_shader: WebGpuShader,
}

#[derive(Debug)]
pub struct WebGpuShader {
    name: Option<String>,

    shader_vertex_attributes: Option<HashMap<String, u32>>,
    /// TODO unneeded?
    kind: ShaderKind,
    module: wgpu::ShaderModule,
}

#[derive(Debug)]
pub struct WebGpuTexture {
    inner: wgpu::Texture,
    size: Vector2I,
    sampling_flags: Cell<TextureSamplingFlags>,
    format: TextureFormat,
    dirty: Cell<bool>,
}

/// For texture readback.
#[derive(Debug)]
pub struct WebGpuTextureDataReceiver {}

#[derive(Debug)]
pub struct WebGpuTimerQuery {}

#[derive(Debug)]
pub struct WebGpuUniform {
    name: String,
}

#[derive(Debug)]
pub struct WebGpuVertexArray {
    descriptor: (),
    vertex_buffers: RefCell<Vec<WebGpuBuffer>>,
    index_buffer: RefCell<Option<WebGpuBuffer>>,
}

#[derive(Debug)]
pub struct WebGpuVertexAttr {
    name: String,
    bind_location: u32,
}

/// Extension method for `i32` to reduce the code duplication of converting and unwrapping into a `u32`.
trait ExpectUnsigned {
    fn expect_unsigned(self) -> u32;
}

impl ExpectUnsigned for i32 {
    fn expect_unsigned(self) -> u32 {
        u32::try_from(self).expect("number must be unsigned")
    }
}

impl Device for WebGpuDevice {
    type Buffer = WebGpuBuffer;
    type Framebuffer = WebGpuFramebuffer;
    type Program = WebGpuProgram;
    type Shader = WebGpuShader;
    type Texture = WebGpuTexture;
    type TextureDataReceiver = WebGpuTextureDataReceiver;
    type TimerQuery = WebGpuTimerQuery;
    type Uniform = WebGpuUniform;
    type VertexArray = WebGpuVertexArray;
    type VertexAttr = WebGpuVertexAttr;

    fn create_texture(&self, format: TextureFormat, size: Vector2I) -> Self::Texture {
        WebGpuTexture {
            inner: self.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: size.x().expect_unsigned(),
                    height: size.y().expect_unsigned(),
                    depth: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: match format {
                    TextureFormat::R8 => wgpu::TextureFormat::R8Unorm,
                    TextureFormat::R16F => wgpu::TextureFormat::R16Float,
                    TextureFormat::RGBA8 => wgpu::TextureFormat::Rgba8Unorm,
                    TextureFormat::RGBA16F => wgpu::TextureFormat::Rgba16Float,
                    TextureFormat::RGBA32F => wgpu::TextureFormat::Rgba32Float,
                },
                usage: wgpu::TextureUsage::UNINITIALIZED, // TODO
            }),
            size,
            format,
            sampling_flags: Cell::new(TextureSamplingFlags::empty()),
            dirty: Cell::new(false),
        }
    }

    fn create_texture_from_data(
        &self,
        format: TextureFormat,
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
        let suffix = match kind {
            ShaderKind::Vertex => 'v',
            ShaderKind::Fragment => 'f',
        };
        let path = format!("shaders/spirv/{}.{}s.spv", name, suffix);
        self.create_shader_from_source(name, &resources.slurp(&path).unwrap(), kind)
    }

    fn create_shader_from_source(
        &self,
        name: &str,
        source: &[u8],
        kind: ShaderKind,
    ) -> Self::Shader {
        let reflect_module = spirv_reflect::ShaderModule::load_u8_data(source).unwrap();
        let entry_point_name = reflect_module.get_entry_point_name();
        let shader_vertex_attributes = if reflect_module
            .get_shader_stage()
            .contains(spirv_reflect::types::variable::ReflectShaderStageFlags::VERTEX)
        {
            Some(
                reflect_module
                    .enumerate_input_variables(Some(&entry_point_name))
                    .unwrap()
                    .into_iter()
                    .filter_map(|interface_variable| {
                        // The naming convention in the shaders is that all attributes start with "a". `get_vertex_attr` drops this, so we will also drop the "a" so that string comparisons later will work.
                        // This also filters `gl_*` builtins.
                        if interface_variable.name.starts_with("a") {
                            Some((
                                interface_variable.name[1..].to_owned(),
                                interface_variable.location,
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<String, u32>>(),
            )
        } else {
            None
        };

        const SPIRV_WORD_LEN: usize = mem::size_of::<u32>();
        assert!(
            source.len() % SPIRV_WORD_LEN == 0,
            "spirv bytecode not a whole number of 32-bit words"
        );

        let module_bytecode: &[u32] = unsafe {
            slice::from_raw_parts(source.as_ptr() as *const _, source.len() / SPIRV_WORD_LEN)
        };

        WebGpuShader {
            name: if cfg!(debug_assertions) {
                Some(name.to_owned())
            } else {
                None
            },
            kind,
            shader_vertex_attributes,
            module: self.device.create_shader_module(module_bytecode),
        }
    }

    fn create_vertex_array(&self) -> Self::VertexArray {
        // self.device.create_buffer(&wgpu::BufferDescriptor{ label: None, size: (), usage: ()});

        WebGpuVertexArray {
            descriptor: (), // TODO
            vertex_buffers: RefCell::new(Vec::new()),
            index_buffer: RefCell::new(None),
        }
    }

    fn create_program_from_shaders(
        &self,
        _resources: &dyn ResourceLoader,
        name: &str,
        vertex_shader: Self::Shader,
        fragment_shader: Self::Shader,
    ) -> Self::Program {
        // render pipeline?
        // self.device
        //     .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        //         layout: todo!(),
        //         vertex_stage: wgpu::ProgrammableStageDescriptor {
        //             module: vertex_shader.module,
        //             entry_point: "main",
        //         },
        //         fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
        //             module: fragment_shader.module,
        //             entry_point: "main",
        //         }),
        //         rasterization_state: todo!(),
        //         primitive_topology: todo!(),
        //         color_states: todo!(),
        //         depth_stencil_state: todo!(),
        //         vertex_state: todo!(),
        //         sample_count: todo!(),
        //         sample_mask: todo!(),
        //         alpha_to_coverage_enabled: todo!(),
        //     });

        // WebGPU's program is part of the render pipeline, which includes all GPU state, so we defer creating it until we know our state??
        WebGpuProgram {
            name: if cfg!(debug_assertions) {
                Some(name.to_owned())
            } else {
                None
            },
            vertex_shader,
            fragment_shader,
        }
    }

    fn get_vertex_attr(&self, program: &Self::Program, name: &str) -> Option<Self::VertexAttr> {
        dbg!(name, &program.vertex_shader.shader_vertex_attributes);
        program
            .vertex_shader
            .shader_vertex_attributes
            .as_ref()
            .expect("vertex shader must have attribute table")
            .get(name)
            .map(|bind_location| WebGpuVertexAttr {
                name: name.to_owned(),
                bind_location: *bind_location,
            })
    }

    fn get_uniform(&self, program: &Self::Program, name: &str) -> Self::Uniform {
        // TODO check for validity in program? why is program passed?
        WebGpuUniform {
            name: name.to_owned(),
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
                .push((*buffer).clone()),
            BufferTarget::Index => {
                *vertex_array.index_buffer.borrow_mut() = Some((*buffer).clone())
            }
        }
    }

    fn configure_vertex_attr(
        &self,
        vertex_array: &Self::VertexArray,
        attr: &Self::VertexAttr,
        descriptor: &VertexAttrDescriptor,
    ) {
        // self.device
        //     .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //         bind_group_layouts: (),
        //     });

        // pub size: usize,
        // pub class: VertexAttrClass,
        // pub attr_type: VertexAttrType,
        // pub stride: usize,
        // pub offset: usize,
        // pub divisor: u32,
        // pub buffer_index: u32,

        wgpu::VertexAttributeDescriptor {
            offset: descriptor.offset as u64,
            format: {
                use pathfinder_gpu::VertexAttrClass as Class;
                use pathfinder_gpu::VertexAttrType as Type;
                use wgpu::VertexFormat as Format;

                match (descriptor.class, descriptor.attr_type, descriptor.size) {
                    (Class::Int, Type::I8, 2) => Format::Char2,
                    // (Class::Int, Type::I8, 3) => Format::Char3,
                    (Class::Int, Type::I8, 4) => Format::Char4,
                    (Class::Int, Type::U8, 2) => Format::Uchar2,
                    // (Class::Int, Type::U8, 3) => Format::Uchar3,
                    (Class::Int, Type::U8, 4) => Format::Uchar4,
                    (Class::FloatNorm, Type::U8, 2) => Format::Uchar2Norm,
                    // (Class::FloatNorm, Type::U8, 3) => Format::Uchar3Norm,
                    (Class::FloatNorm, Type::U8, 4) => Format::Uchar4Norm,
                    (Class::FloatNorm, Type::I8, 2) => Format::Char2Norm,
                    // (Class::FloatNorm, Type::I8, 3) => Format::Char3Norm,
                    (Class::FloatNorm, Type::I8, 4) => Format::Char4Norm,
                    (Class::Int, Type::I16, 2) => Format::Short2,
                    // (Class::Int, Type::I16, 3) => Format::Short3,
                    (Class::Int, Type::I16, 4) => Format::Short4,
                    (Class::Int, Type::U16, 2) => Format::Ushort2,
                    // (Class::Int, Type::U16, 3) => Format::Ushort3,
                    (Class::Int, Type::U16, 4) => Format::Ushort4,
                    (Class::FloatNorm, Type::U16, 2) => Format::Ushort2Norm,
                    // (Class::FloatNorm, Type::U16, 3) => Format::Ushort3Norm,
                    (Class::FloatNorm, Type::U16, 4) => Format::Ushort4Norm,
                    (Class::FloatNorm, Type::I16, 2) => Format::Short2Norm,
                    // (Class::FloatNorm, Type::I16, 3) => Format::Short3Norm,
                    (Class::FloatNorm, Type::I16, 4) => Format::Short4Norm,
                    (Class::Float, Type::F32, 1) => Format::Float,
                    (Class::Float, Type::F32, 2) => Format::Float2,
                    (Class::Float, Type::F32, 3) => Format::Float3,
                    (Class::Float, Type::F32, 4) => Format::Float4,
                    // (Class::Int, Type::I8, 1) => Format::Char,
                    // (Class::Int, Type::U8, 1) => Format::Uchar,
                    // (Class::FloatNorm, Type::I8, 1) => Format::CharNorm,
                    // (Class::FloatNorm, Type::U8, 1) => Format::UcharNorm,
                    // (Class::Int, Type::I16, 1) => Format::Short,
                    // (Class::Int, Type::U16, 1) => Format::Ushort,
                    // (Class::FloatNorm, Type::U16, 1) => Format::UshortNorm,
                    // (Class::FloatNorm, Type::I16, 1) => Format::ShortNorm,
                    (attr_class, attr_type, attr_size) => panic!(
                        "Unsupported vertex class/type/size combination: {:?}/{:?}/{}!",
                        attr_class, attr_type, attr_size
                    ),
                }
            },
            shader_location: attr.bind_location,
        };

        // perhaps more to do?
        // todo!()
    }

    fn create_framebuffer(&self, texture: Self::Texture) -> Self::Framebuffer {
        todo!()
    }

    fn create_buffer(&self) -> Self::Buffer {
        WebGpuBuffer {
            inner: Rc::new(RefCell::new(None)),
        }
    }

    fn allocate_buffer<T>(
        &self,
        buffer: &Self::Buffer,
        data: BufferData<T>,
        target: BufferTarget,
        mode: BufferUploadMode,
    ) {
        // assert_eq!(
        //     *buffer,
        //     WebGpuBuffer::Uninitialized,
        //     "tried to initialized an already initialized buffer"
        // );

        // TODO use mode?

        let usage = match target {
            BufferTarget::Vertex => wgpu::BufferUsage::VERTEX,
            BufferTarget::Index => wgpu::BufferUsage::INDEX,
        } | wgpu::BufferUsage::empty();

        let (new_buffer, len) = match data {
            BufferData::Uninitialized(size) => (
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: size as u64,
                    usage,
                }),
                size,
            ),
            BufferData::Memory(data) => {
                let data_len_in_bytes = data.len() * mem::size_of::<T>();
                let data =
                    unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, data_len_in_bytes) };
                (
                    self.device.create_buffer_with_data(data, usage),
                    data_len_in_bytes,
                )
            }
        };

        let mut old_buffer = buffer.inner.borrow_mut();
        if let Some(old_buffer) = &*old_buffer {
            let mut encoder = self.borrow_current_command_encoder();
            encoder.copy_buffer_to_buffer(&new_buffer, 0, &old_buffer, 0, len as u64);
        } else {
            *old_buffer = Some(new_buffer);
        }
    }

    fn framebuffer_texture<'f>(&self, framebuffer: &'f Self::Framebuffer) -> &'f Self::Texture {
        todo!()
    }

    fn destroy_framebuffer(&self, framebuffer: Self::Framebuffer) -> Self::Texture {
        todo!()
    }

    fn texture_format(&self, texture: &Self::Texture) -> TextureFormat {
        texture.format
    }

    fn texture_size(&self, texture: &Self::Texture) -> Vector2I {
        texture.size
    }

    fn set_texture_sampling_mode(&self, texture: &Self::Texture, flags: TextureSamplingFlags) {
        texture.sampling_flags.set(flags);
    }

    /// Upload `data` to a buffer and copy to texture in a new command buffer.
    fn upload_to_texture(&self, texture: &Self::Texture, rect: RectI, data: TextureDataRef) {
        /// Hack to avoid a dependency on the `half` crate.
        #[allow(non_camel_case_types)]
        type f16 = u16;

        let data = unsafe {
            let data_ptr = data.check_and_extract_data_ptr(texture.size, texture.format);
            let data_len = match data {
                TextureDataRef::U8(data) => data.len() * mem::size_of::<u8>(),
                TextureDataRef::F16(data) => data.len() * mem::size_of::<f16>(),
                TextureDataRef::F32(data) => data.len() * mem::size_of::<f32>(),
            };
            slice::from_raw_parts(data_ptr as *const u8, data_len)
        };

        let data_buffer = self
            .device
            .create_buffer_with_data(data, wgpu::BufferUsage::COPY_SRC);

        let mut encoder = self.borrow_current_command_encoder();
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &data_buffer,
                offset: 0,
                bytes_per_row: {
                    texture.size.x().expect_unsigned()
                        * match texture.format {
                            TextureFormat::R8 => mem::size_of::<u8>(),
                            TextureFormat::R16F => mem::size_of::<f16>(),
                            TextureFormat::RGBA8 => 2 * mem::size_of::<u8>(),
                            TextureFormat::RGBA16F => 4 * mem::size_of::<f16>(),
                            TextureFormat::RGBA32F => 4 * mem::size_of::<f32>(),
                        } as u32
                },
                rows_per_image: texture.size.y().expect_unsigned(),
            },
            wgpu::TextureCopyView {
                texture: &texture.inner,
                mip_level: 1,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::Extent3d {
                width: texture.size.x().expect_unsigned(),
                height: texture.size.y().expect_unsigned(),
                depth: 1,
            },
        );
    }

    fn read_pixels(
        &self,
        target: &RenderTarget<Self>,
        viewport: RectI,
    ) -> Self::TextureDataReceiver {
        todo!()
    }

    fn begin_commands(&self) {
        self.finish_current_command_encoder();

        // assert!(
        //     self.current_command_encoder.borrow().is_none(),
        //     "begin_commands and end_commands must be called in pairs"
        // );
        // *self.current_command_encoder.borrow_mut() = Some(
        //     self.device
        //         .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None }),
        // );
    }

    fn end_commands(&self) {
        // no-op. see: `Self::finish_current_command_encoder`.

        // self.command_buffers.borrow_mut().push(
        //     self.current_command_encoder
        //         .borrow_mut()
        //         .take()
        //         .expect("begin_commands and end_commands must be called in pairss")
        //         .finish(),
        // );
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
        todo!()
    }

    fn begin_timer_query(&self, query: &Self::TimerQuery) {
        todo!()
    }

    fn end_timer_query(&self, query: &Self::TimerQuery) {
        todo!()
    }

    fn try_recv_timer_query(&self, query: &Self::TimerQuery) -> Option<Duration> {
        todo!()
    }

    fn recv_timer_query(&self, query: &Self::TimerQuery) -> Duration {
        todo!()
    }

    fn try_recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> Option<TextureData> {
        todo!()
    }

    fn recv_texture_data(&self, receiver: &Self::TextureDataReceiver) -> TextureData {
        todo!()
    }
}
