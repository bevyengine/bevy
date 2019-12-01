use crate::{temp::*, render::shadow::ShadowPass};

use std::mem;
use zerocopy::{AsBytes, FromBytes};
use wgpu::{Device, BindGroupLayout, VertexBufferDescriptor, SwapChainDescriptor};

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct ForwardUniforms {
    pub proj: [[f32; 4]; 4],
    pub num_lights: [u32; 4],
}

pub struct ForwardPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub forward_uniform_buffer: wgpu::Buffer,
    pub light_uniform_buffer: wgpu::Buffer,
    pub depth_texture: wgpu::TextureView,
}

impl ForwardPass {
    pub const MAX_LIGHTS: usize = 10;
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    
    pub fn new(device: &Device, forward_uniforms: ForwardUniforms, shadow_pass: &ShadowPass, vertex_buffer_descriptor: VertexBufferDescriptor, local_bind_group_layout: &BindGroupLayout, swap_chain_descriptor: &SwapChainDescriptor) -> ForwardPass {
        let vs_bytes = load_glsl(
            include_str!("forward.vert"),
            ShaderStage::Vertex,
        );
        let fs_bytes = load_glsl(
            include_str!("forward.frag"),
            ShaderStage::Fragment,
        );

        let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0, // global
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 1, // lights
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2Array,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 3,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
            ],
        });

        let uniform_size = mem::size_of::<ForwardUniforms>() as wgpu::BufferAddress;
        let forward_uniform_buffer = device.create_buffer_with_data(
            forward_uniforms.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let light_uniform_size =
        (Self::MAX_LIGHTS * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;

        let light_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: light_uniform_size,
            usage: wgpu::BufferUsage::UNIFORM
                | wgpu::BufferUsage::COPY_SRC
                | wgpu::BufferUsage::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &forward_uniform_buffer,
                        range: 0 .. uniform_size,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &light_uniform_buffer,
                        range: 0 .. light_uniform_size,
                    },
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&shadow_pass.shadow_view),
                },
                wgpu::Binding {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&shadow_pass.shadow_sampler),
                },
            ],
        });


        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout, local_bind_group_layout],
        });

        let vs_module = device.create_shader_module(&vs_bytes);
        let fs_module = device.create_shader_module(&fs_bytes);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: swap_chain_descriptor.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[vertex_buffer_descriptor],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });


        ForwardPass {
            pipeline,
            bind_group,
            forward_uniform_buffer,
            light_uniform_buffer,
            depth_texture: Self::get_depth_texture(device, swap_chain_descriptor)
        }
    }


    pub fn update_swap_chain_descriptor(&mut self, device: &Device, swap_chain_descriptor: &SwapChainDescriptor) {
        self.depth_texture = Self::get_depth_texture(device, swap_chain_descriptor);
    }

    fn get_depth_texture(device: &Device, swap_chain_descriptor: &SwapChainDescriptor) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: swap_chain_descriptor.width,
                height: swap_chain_descriptor.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        texture.create_default_view()
    }
}

