use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    pass::{
        LoadOp, PassDescriptor, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, StoreOp, TextureAttachment,
    },
    pipeline::{
        state_descriptors::{
            BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
            CompareFunction, DepthStencilStateDescriptor, StencilOperation,
            StencilStateFaceDescriptor,
        },
        InputStepMode, PipelineDescriptor, VertexAttributeDescriptor, VertexBufferDescriptor,
        VertexFormat,
    },
    render_resource::{BufferInfo, BufferUsage, RenderResource, RenderResourceAssignments},
    renderer::RenderContext,
    shader::{Shader, ShaderSource, ShaderStage, ShaderStages},
    texture::{
        AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsage,
    },
    Color,
};
use byteorder::{NativeEndian, WriteBytesExt};
use pathfinder_canvas::vec2i;
use pathfinder_geometry::{rect::RectI, vector::Vector2I};
use pathfinder_gpu::{
    BufferData, BufferTarget, BufferUploadMode, Device, FeatureLevel, ProgramKind, RenderState,
    RenderTarget, ShaderKind, TextureData, TextureDataRef, TextureSamplingFlags, UniformData,
    VertexAttrClass, VertexAttrDescriptor, VertexAttrType,
};
use pathfinder_resources::ResourceLoader;
use std::{borrow::Cow, cell::RefCell, collections::HashMap, mem, rc::Rc, time::Duration, ops::Range};
use zerocopy::AsBytes;

pub struct BevyPathfinderDevice<'a> {
    render_context: RefCell<&'a mut dyn RenderContext>,
    shaders: RefCell<&'a mut AssetStorage<Shader>>,
    samplers: RefCell<HashMap<u8, RenderResource>>,
    main_color_texture: RenderResource,
    main_depth_stencil_texture: RenderResource,
}

impl<'a> BevyPathfinderDevice<'a> {
    pub fn new(
        render_context: &'a mut dyn RenderContext,
        shaders: &'a mut AssetStorage<Shader>,
        main_color_texture: RenderResource,
        main_depth_stencil_texture: RenderResource,
    ) -> Self {
        BevyPathfinderDevice {
            render_context: RefCell::new(render_context),
            shaders: RefCell::new(shaders),
            samplers: RefCell::new(HashMap::new()),
            main_color_texture,
            main_depth_stencil_texture,
        }
    }

    pub fn prepare_to_draw(&self, render_state: &RenderState<BevyPathfinderDevice>) {
        let pass_descriptor = self.create_pass_descriptor(render_state);
        self.setup_pipline_descriptor(render_state, &pass_descriptor, &render_state.vertex_array.requested_descriptors.borrow());
        // TODO: setup uniforms
        let mut render_context = self.render_context.borrow_mut();
        let mut render_resource_assignments = RenderResourceAssignments::default();
        for (i, vertex_buffer) in render_state
            .vertex_array
            .vertex_buffers
            .borrow()
            .iter()
            .enumerate()
        {
            let resource = vertex_buffer.handle.borrow().unwrap();
            let mut indices_resource = None;
            if i == 0 {
                if let Some(ref index_buffer) = *render_state.vertex_array.index_buffer.borrow() {
                    indices_resource = Some(index_buffer.handle.borrow().unwrap());
                }
            }
            render_resource_assignments.set_vertex_buffer(get_vertex_buffer_name(i), resource, indices_resource);
        }

        // if let Some(ref index_buffer) = *render_state.vertex_array.index_buffer.borrow() {
        //     let resource = index_buffer.handle.borrow().unwrap();
        //     pass.set_index_buffer(resource, 0);
        // }
        render_context.begin_pass(
            &pass_descriptor,
            &render_resource_assignments,
            &mut |pass| {
                let viewport = render_state.viewport;
                pass.set_viewport(
                    viewport.origin().x() as f32,
                    viewport.origin().y() as f32,
                    viewport.size().x() as f32,
                    viewport.size().y() as f32,
                    0.0,
                    1.0,
                );

                if let Some(stencil_state) = render_state.options.stencil {
                    pass.set_stencil_reference(stencil_state.reference);
                }

                let pipeline_descriptor = render_state.program.pipeline_descriptor.borrow();
                pass.set_render_resources(&pipeline_descriptor, &render_resource_assignments);
            },
        )
    }

    fn get_texture_format(&self, _render_resource: RenderResource) -> Option<TextureFormat> {
        // TODO: lookup real texture format
        // let mut texture_format = None;
        // self.render_context.borrow().resources().get_resource_info(
        //     texture_resource,
        //     &mut |info| {
        //         if let Some(info) = info {
        //             match info {
        //                 ResourceInfo::Texture {

        //                 }
        //             }
        //             texture_format = Some(info)
        //         }
        //     },
        // );
        Some(TextureFormat::Bgra8UnormSrgb)
    }

    pub fn setup_pipline_descriptor(
        &self,
        render_state: &RenderState<BevyPathfinderDevice>,
        pass_descriptor: &PassDescriptor,
        requested_vertex_descriptors: &HashMap<u32, VertexBufferDescriptor>,
    ) {
        if self
            .render_context
            .borrow()
            .resources()
            .get_asset_resource(render_state.program.pipeline_handle, 0)
            .is_some()
        {
            return;
        }
        let mut pipeline_descriptor = render_state.program.pipeline_descriptor.borrow_mut();
        {
            let mut layout = pipeline_descriptor.get_layout_mut().unwrap();
            let mut i = 0;
            let mut descriptors = Vec::with_capacity(requested_vertex_descriptors.len());
            loop {
                if let Some(descriptor) = requested_vertex_descriptors.get(&i) {
                    descriptors.push(descriptor.clone());
                    i += 1; 
                } else {
                    break;
                }
            }
            layout.vertex_buffer_descriptors = descriptors;
        }

        let color_texture_format = if let TextureAttachment::RenderResource(texture_resource) =
            pass_descriptor
                .color_attachments
                .first()
                .unwrap()
                .attachment
        {
            self.get_texture_format(texture_resource)
                .expect("expected color attachment RenderResource to have a texture format")
        } else {
            panic!("expected a RenderResource color attachment");
        };

        // TODO: lookup real texture format
        // TODO: make sure colors render correctly
        let mut color_state = ColorStateDescriptor {
            format: color_texture_format,
            color_blend: BlendDescriptor {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendDescriptor {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: if render_state.options.color_mask {
                ColorWrite::all()
            } else {
                ColorWrite::empty()
            },
        };

        if let Some(blend_state) = render_state.options.blend {
            let blend_op = blend_state.op.to_bevy_blend_op();
            color_state.color_blend.src_factor = blend_state.src_rgb_factor.to_bevy_blend_factor();
            color_state.color_blend.dst_factor = blend_state.dest_rgb_factor.to_bevy_blend_factor();
            color_state.color_blend.operation = blend_op;

            color_state.alpha_blend.src_factor =
                blend_state.src_alpha_factor.to_bevy_blend_factor();
            color_state.alpha_blend.dst_factor =
                blend_state.dest_alpha_factor.to_bevy_blend_factor();
            color_state.color_blend.operation = blend_op;
        }

        pipeline_descriptor.color_states.push(color_state);

        if let Some(ref _pass_depth_stencil_descriptor) = pass_descriptor.depth_stencil_attachment {
            // TODO: lookup texture format
            // TODO: maybe we need a stencil-type depth format? TextureFormat::Depth24PlusStencil8
            let depth_format = TextureFormat::Depth32Float;
            let mut descriptor = DepthStencilStateDescriptor {
                format: depth_format,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Less,
                stencil_front: StencilStateFaceDescriptor::IGNORE,
                stencil_back: StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            };

            if let Some(depth_state) = render_state.options.depth {
                let compare_function = depth_state.func.to_bevy_compare_function();
                descriptor.depth_compare = compare_function;
                descriptor.depth_write_enabled = true;
            }

            if let Some(stencil_state) = render_state.options.stencil {
                let compare = stencil_state.func.to_bevy_compare_function();
                let (pass_op, write_mask) = if stencil_state.write {
                    (StencilOperation::Replace, stencil_state.mask)
                } else {
                    (StencilOperation::Keep, 0)
                };

                let stencil_descriptor = StencilStateFaceDescriptor {
                    compare,
                    pass_op,
                    depth_fail_op: StencilOperation::Keep,
                    fail_op: StencilOperation::Keep,
                };

                descriptor.stencil_write_mask = write_mask;
                descriptor.stencil_front = stencil_descriptor.clone();
                descriptor.stencil_back = stencil_descriptor;
            }
        }

        self.render_context
            .borrow()
            .resources()
            .create_render_pipeline(
                render_state.program.pipeline_handle,
                &pipeline_descriptor,
                &self.shaders.borrow(),
            );
    }

    pub fn create_pass_descriptor(
        &self,
        render_state: &RenderState<BevyPathfinderDevice>,
    ) -> PassDescriptor {
        let mut depth_texture = None;
        let color_texture = match render_state.target {
            RenderTarget::Default { .. } => {
                depth_texture = Some(self.main_depth_stencil_texture);
                self.main_color_texture
            }
            RenderTarget::Framebuffer(framebuffer) => framebuffer.handle,
        };

        let mut color_attachment = RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::RenderResource(color_texture),
            clear_color: Color::WHITE,
            load_op: LoadOp::Load,
            store_op: StoreOp::Store,
            resolve_target: None,
        };

        if let Some(color) = render_state.options.clear_ops.color {
            color_attachment.clear_color = Color::rgba(color.r(), color.g(), color.b(), color.a());
            color_attachment.load_op = LoadOp::Clear;
        }

        let depth_stencil_attachment = if let Some(depth_texture) = depth_texture {
            let mut descriptor = RenderPassDepthStencilAttachmentDescriptor {
                attachment: TextureAttachment::RenderResource(depth_texture),
                depth_load_op: LoadOp::Load,
                depth_store_op: StoreOp::Store,
                stencil_load_op: LoadOp::Load,
                stencil_store_op: StoreOp::Store,
                clear_depth: 1.0,
                clear_stencil: 0,
            };

            if let Some(depth) = render_state.options.clear_ops.depth {
                descriptor.clear_depth = depth;
                descriptor.depth_load_op = LoadOp::Clear;
            }

            if let Some(value) = render_state.options.clear_ops.stencil {
                descriptor.clear_stencil = value as u32;
                descriptor.stencil_load_op = LoadOp::Clear;
            }

            Some(descriptor)
        } else {
            None
        };

        PassDescriptor {
            color_attachments: vec![color_attachment],
            depth_stencil_attachment,
            sample_count: 1,
        }
    }
    fn create_uniform_buffer(&self, uniforms: &[(&BevyUniform, UniformData)]) -> UniformBuffer {
        let (mut uniform_buffer_data, mut uniform_buffer_ranges) = (vec![], vec![]);
        for &(_, uniform_data) in uniforms.iter() {
            let start_index = uniform_buffer_data.len();
            match uniform_data {
                UniformData::Float(value) => uniform_buffer_data
                    .write_f32::<NativeEndian>(value)
                    .unwrap(),
                UniformData::IVec2(vector) => {
                    uniform_buffer_data
                        .write_i32::<NativeEndian>(vector.x())
                        .unwrap();
                    uniform_buffer_data
                        .write_i32::<NativeEndian>(vector.y())
                        .unwrap();
                }
                UniformData::IVec3(values) => {
                    uniform_buffer_data
                        .write_i32::<NativeEndian>(values[0])
                        .unwrap();
                    uniform_buffer_data
                        .write_i32::<NativeEndian>(values[1])
                        .unwrap();
                    uniform_buffer_data
                        .write_i32::<NativeEndian>(values[2])
                        .unwrap();
                }
                UniformData::Int(value) => uniform_buffer_data
                    .write_i32::<NativeEndian>(value)
                    .unwrap(),
                UniformData::Mat2(matrix) => {
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(matrix.x())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(matrix.y())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(matrix.z())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(matrix.w())
                        .unwrap();
                }
                UniformData::Mat4(matrix) => {
                    for column in &matrix {
                        uniform_buffer_data
                            .write_f32::<NativeEndian>(column.x())
                            .unwrap();
                        uniform_buffer_data
                            .write_f32::<NativeEndian>(column.y())
                            .unwrap();
                        uniform_buffer_data
                            .write_f32::<NativeEndian>(column.z())
                            .unwrap();
                        uniform_buffer_data
                            .write_f32::<NativeEndian>(column.w())
                            .unwrap();
                    }
                }
                UniformData::Vec2(vector) => {
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(vector.x())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(vector.y())
                        .unwrap();
                }
                UniformData::Vec3(array) => {
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(array[0])
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(array[1])
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(array[2])
                        .unwrap();
                }
                UniformData::Vec4(vector) => {
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(vector.x())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(vector.y())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(vector.z())
                        .unwrap();
                    uniform_buffer_data
                        .write_f32::<NativeEndian>(vector.w())
                        .unwrap();
                }
                UniformData::TextureUnit(_) | UniformData::ImageUnit(_) => {}
            }
            // TODO: this padding might not be necessary
            let end_index = uniform_buffer_data.len();
            while uniform_buffer_data.len() % 256 != 0 {
                uniform_buffer_data.push(0);
            }
            uniform_buffer_ranges.push(start_index..end_index);
        }

        UniformBuffer {
            data: uniform_buffer_data,
            ranges: uniform_buffer_ranges,
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

pub struct BevyProgram {
    pipeline_descriptor: RefCell<PipelineDescriptor>,
    pipeline_handle: Handle<PipelineDescriptor>,
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
    type Program = BevyProgram;
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
    ) -> BevyProgram {
        match shaders {
            ProgramKind::Compute(_) => panic!("bevy does not currently support compute shaders"),
            ProgramKind::Raster { vertex, fragment } => {
                let mut descriptor = PipelineDescriptor::new(ShaderStages {
                    vertex,
                    fragment: Some(fragment),
                });
                descriptor.reflect_layout(&self.shaders.borrow(), false, None, None);
                BevyProgram {
                    pipeline_descriptor: RefCell::new(descriptor),
                    pipeline_handle: Handle::new(),
                }
            }
        }
    }
    fn get_vertex_attr(&self, program: &BevyProgram, name: &str) -> Option<BevyVertexAttr> {
        // TODO: this probably isn't actually used for anything. try to optimize
        let pipeline_descriptor = program.pipeline_descriptor.borrow();
        let layout = pipeline_descriptor.get_layout().unwrap();
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

        None
    }
    fn get_uniform(&self, _program: &BevyProgram, name: &str) -> Self::Uniform {
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
            BufferTarget::Index => *vertex_array.index_buffer.borrow_mut() = Some(buffer.clone()),
            _ => panic!("Buffers bound to vertex arrays must be vertex or index buffers!"),
        }
    }
    fn configure_vertex_attr(
        &self,
        vertex_array: &BevyVertexArray,
        bevy_attr: &BevyVertexAttr,
        descriptor: &VertexAttrDescriptor,
    ) {
        println!("configure");
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
                    name: Cow::Borrowed(get_vertex_buffer_name(buffer_index as usize)),
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

    fn allocate_buffer<T>(&self, buffer: &BevyBuffer, data: BufferData<T>, target: BufferTarget) {
        let usage = match target {
            BufferTarget::Vertex => BufferUsage::VERTEX,
            BufferTarget::Index => BufferUsage::INDEX,
            BufferTarget::Storage => BufferUsage::empty(),
        };

        let buffer_usage = match buffer.mode {
            BufferUploadMode::Dynamic => BufferUsage::WRITE_ALL | usage,
            BufferUploadMode::Static => BufferUsage::COPY_DST | usage,
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
    fn draw_arrays(&self, _index_count: u32, render_state: &RenderState<Self>) {
        self.prepare_to_draw(render_state);
        println!("draw_arrays");
    }
    fn draw_elements(&self, _index_count: u32, render_state: &RenderState<Self>) {
        self.prepare_to_draw(render_state);
        println!("draw_elements");
    }
    fn draw_elements_instanced(
        &self,
        _index_count: u32,
        _instance_count: u32,
        render_state: &RenderState<Self>,
    ) {
        self.prepare_to_draw(render_state);
        println!("draw_elements_instanced");
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
                    buffer_usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
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
        TextureDataRef::F16(data) => slice_to_u8(data),
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

trait ToBevyBlendOp {
    fn to_bevy_blend_op(self) -> BlendOperation;
}

impl ToBevyBlendOp for pathfinder_gpu::BlendOp {
    #[inline]
    fn to_bevy_blend_op(self) -> BlendOperation {
        match self {
            pathfinder_gpu::BlendOp::Add => BlendOperation::Add,
            pathfinder_gpu::BlendOp::Subtract => BlendOperation::Subtract,
            pathfinder_gpu::BlendOp::ReverseSubtract => BlendOperation::ReverseSubtract,
            pathfinder_gpu::BlendOp::Min => BlendOperation::Min,
            pathfinder_gpu::BlendOp::Max => BlendOperation::Max,
        }
    }
}

trait ToBevyBlendFactor {
    fn to_bevy_blend_factor(self) -> BlendFactor;
}

impl ToBevyBlendFactor for pathfinder_gpu::BlendFactor {
    #[inline]
    fn to_bevy_blend_factor(self) -> BlendFactor {
        match self {
            pathfinder_gpu::BlendFactor::Zero => BlendFactor::Zero,
            pathfinder_gpu::BlendFactor::One => BlendFactor::One,
            pathfinder_gpu::BlendFactor::SrcAlpha => BlendFactor::SrcAlpha,
            pathfinder_gpu::BlendFactor::OneMinusSrcAlpha => BlendFactor::OneMinusSrcAlpha,
            pathfinder_gpu::BlendFactor::DestAlpha => BlendFactor::DstAlpha,
            pathfinder_gpu::BlendFactor::OneMinusDestAlpha => BlendFactor::OneMinusDstAlpha,
            pathfinder_gpu::BlendFactor::DestColor => BlendFactor::DstColor,
        }
    }
}

trait ToBevyCompareFunction {
    fn to_bevy_compare_function(self) -> CompareFunction;
}

impl ToBevyCompareFunction for pathfinder_gpu::DepthFunc {
    fn to_bevy_compare_function(self) -> CompareFunction {
        match self {
            pathfinder_gpu::DepthFunc::Always => CompareFunction::Always,
            pathfinder_gpu::DepthFunc::Less => CompareFunction::Less,
        }
    }
}

impl ToBevyCompareFunction for pathfinder_gpu::StencilFunc {
    fn to_bevy_compare_function(self) -> CompareFunction {
        match self {
            pathfinder_gpu::StencilFunc::Always => CompareFunction::Always,
            pathfinder_gpu::StencilFunc::Equal => CompareFunction::Equal,
        }
    }
}

struct UniformBuffer {
    data: Vec<u8>,
    ranges: Vec<Range<usize>>,
}

pub const PATHFINDER_VERTEX_BUFFER_0: &'static str = "P0";  
pub const PATHFINDER_VERTEX_BUFFER_1: &'static str = "P1";  
pub const PATHFINDER_VERTEX_BUFFER_2: &'static str = "P2";  
pub const PATHFINDER_VERTEX_BUFFER_3: &'static str = "P3";  

pub fn get_vertex_buffer_name(index: usize) -> &'static str {
    match index {
        0 => PATHFINDER_VERTEX_BUFFER_0,
        1 => PATHFINDER_VERTEX_BUFFER_1,
        2 => PATHFINDER_VERTEX_BUFFER_2,
        3 => PATHFINDER_VERTEX_BUFFER_3,
        _ => panic!("encountered unknown vertex buffer index"),
    }
}