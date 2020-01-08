use crate::{asset::*, render::*};
use legion::prelude::*;
use std::mem;
use wgpu::SwapChainOutput;

pub const SHADOW_PIPELINE_UNIFORMS: &str = "shadow_pipeline";
pub const SHADOW_SAMPLER_NAME: &str = "shadow_sampler";

#[repr(C)]
pub struct ShadowUniforms {
    pub proj: [[f32; 4]; 4],
}

pub struct ShadowPipeline {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub shadow_format: wgpu::TextureFormat,
}

impl ShadowPipeline {
    #[allow(dead_code)]
    pub fn new(shadow_format: wgpu::TextureFormat) -> Self {
        ShadowPipeline {
            bind_group: None,
            pipeline: None,
            shadow_format,
        }
    }
}

impl Pipeline for ShadowPipeline {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, _: &mut World) {
        let bind_group_layout = render_graph.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0, // global
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });

        // TODO: stop using "local"
        let local_bind_group_layout = render_graph.get_bind_group_layout("local").unwrap();

        let pipeline_layout = render_graph.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &bind_group_layout,
                local_bind_group_layout,
            ],
        });

        let uniform_size = mem::size_of::<ShadowUniforms>() as wgpu::BufferAddress;
        let uniform_buf = render_graph.device.create_buffer(&wgpu::BufferDescriptor {
            size: uniform_size,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        // Create bind group
        self.bind_group = Some(render_graph.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniform_buf,
                    range: 0..uniform_size,
                },
            }],
        }));

        render_graph.set_uniform_buffer(SHADOW_PIPELINE_UNIFORMS, UniformBuffer {
            buffer: uniform_buf,
            size: uniform_size,
        });

        // Create other resources
        let shadow_sampler = render_graph.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::LessEqual,
        });

        render_graph.set_sampler(SHADOW_SAMPLER_NAME, shadow_sampler);

        let vertex_buffer_descriptor = get_vertex_buffer_descriptor();

        // Create the render pipeline
        let vs_bytes =
            shader::load_glsl(include_str!("shadow.vert"), shader::ShaderStage::Vertex);
        let fs_bytes =
            shader::load_glsl(include_str!("shadow.frag"), shader::ShaderStage::Fragment);
        let vs_module = render_graph.device.create_shader_module(&vs_bytes);
        let fs_module = render_graph.device.create_shader_module(&fs_bytes);

        self.pipeline = Some(render_graph.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                depth_bias: 2, // corresponds to bilinear filtering
                depth_bias_slope_scale: 2.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: self.shadow_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
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
        }));
    }

    fn render(
        &mut self,
        render_graph: &RenderGraphData,
        pass: &mut wgpu::RenderPass,
        _: &SwapChainOutput,
        world: &mut World,
    ) {
        let mut mesh_query =
            <(Read<Material>, Read<Handle<Mesh>>)>::query().filter(!component::<Instanced>());
        pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

        let mut mesh_storage = world
            .resources
            .get_mut::<AssetStorage<Mesh, MeshType>>()
            .unwrap();
        for (entity, mesh) in mesh_query.iter_immutable(world) {
            if let Some(mesh_asset) = mesh_storage.get(*mesh.id.read().unwrap()) {
                mesh_asset.setup_buffers(&render_graph.device);

                pass.set_bind_group(1, entity.bind_group.as_ref().unwrap(), &[]);
                pass.set_index_buffer(&mesh_asset.index_buffer.as_ref().unwrap(), 0);
                pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                pass.draw_indexed(0..mesh_asset.indices.len() as u32, 0, 0..1);
            };
        }
    }

    fn resize(&mut self, _: &RenderGraphData) {}

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}