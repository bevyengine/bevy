use crate::{render::*, asset::*, render::mesh::*};
use legion::prelude::*;
use zerocopy::{AsBytes, FromBytes};
use wgpu::{Device, SwapChainDescriptor, SwapChainOutput};

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct ForwardUniforms {
    pub proj: [[f32; 4]; 4],
    pub num_lights: [u32; 4],
}

pub struct ForwardPipelineNew {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub depth_format: wgpu::TextureFormat,
    pub local_bind_group: Option<wgpu::BindGroup>,
}

pub struct ForwardPass {
    pub depth_format: wgpu::TextureFormat,
}

impl ForwardPass {
    pub fn new(depth_format: wgpu::TextureFormat) -> Self {
        ForwardPass {
            depth_format, 
        }
    }
    fn get_depth_texture(&self, device: &Device, swap_chain_descriptor: &SwapChainDescriptor) -> wgpu::TextureView {
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
            format: self.depth_format,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        texture.create_default_view()
    }
}

const DEPTH_TEXTURE_NAME: &str = "forward_depth";

impl Pass for ForwardPass {
    fn initialize(&self, render_graph: &mut RenderGraphData) { 
        let depth_texture = self.get_depth_texture(&render_graph.device, &render_graph.swap_chain_descriptor);
        render_graph.set_texture(DEPTH_TEXTURE_NAME, depth_texture);
    }
    fn begin<'a>(&self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, frame: &'a wgpu::SwapChainOutput) -> wgpu::RenderPass<'a> {
        let depth_texture = render_graph.get_texture(DEPTH_TEXTURE_NAME);
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color {
                    r: 0.3,
                    g: 0.4,
                    b: 0.5,
                    a: 1.0,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: depth_texture.unwrap(),
                depth_load_op: wgpu::LoadOp::Clear,
                depth_store_op: wgpu::StoreOp::Store,
                stencil_load_op: wgpu::LoadOp::Clear,
                stencil_store_op: wgpu::StoreOp::Store,
                clear_depth: 1.0,
                clear_stencil: 0,
            }),
        })
    }
    fn resize(&self, render_graph: &mut RenderGraphData) {
        let depth_texture = self.get_depth_texture(&render_graph.device, &render_graph.swap_chain_descriptor);
        render_graph.set_texture(DEPTH_TEXTURE_NAME, depth_texture);
    }
}

impl ForwardPipelineNew {
    pub fn new() -> Self {
        ForwardPipelineNew {
            pipeline: None,
            local_bind_group: None,
            depth_format: wgpu::TextureFormat::Depth32Float
        }
    }
}

impl PipelineNew for ForwardPipelineNew {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, world: &mut World) {
        let vs_bytes = shader::load_glsl(
            include_str!("forward.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("forward.frag"),
            shader::ShaderStage::Fragment,
        );

        let bind_group_layout =
        render_graph.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                }
            ],
        });

        self.local_bind_group = Some({

            let forward_uniform_buffer = render_graph.get_uniform_buffer(render_resources::FORWARD_UNIFORM_BUFFER_NAME).unwrap();
            let light_uniform_buffer = render_graph.get_uniform_buffer(render_resources::LIGHT_UNIFORM_BUFFER_NAME).unwrap();

            // Create bind group
            render_graph.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: forward_uniform_buffer.get_binding_resource(),
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: light_uniform_buffer.get_binding_resource(),
                    }
                ],
            })
        });

        // TODO: fix this inline "local"
        let local_bind_group_layout = render_graph.get_bind_group_layout("local").unwrap();

        let pipeline_layout = render_graph.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout, local_bind_group_layout],
        });

        let vertex_buffer_descriptor = get_vertex_buffer_descriptor();

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
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: render_graph.swap_chain_descriptor.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: self.depth_format,
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
        }));
    }
    fn render(&mut self, render_graph: &RenderGraphData, pass: &mut wgpu::RenderPass, frame: &SwapChainOutput, world: &mut World) {
        pass.set_bind_group(0, self.local_bind_group.as_ref().unwrap(), &[]);

        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        let mut last_mesh_id = None;
        let mut mesh_query =
            <(Read<Material>, Read<Handle<Mesh>>)>::query()
            .filter(!component::<Instanced>());
        for (entity, mesh) in mesh_query.iter_immutable(world) {
            let current_mesh_id = *mesh.id.read().unwrap();

            let mut should_load_mesh = last_mesh_id == None;
            if let Some(last) = last_mesh_id {
                should_load_mesh = last != current_mesh_id;
            }

            if should_load_mesh {
                if let Some(mesh_asset) = mesh_storage.get(*mesh.id.read().unwrap()) {
                    mesh_asset.setup_buffers(&render_graph.device);
                    pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                    pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                };
            }

            if let Some(ref mesh_asset) = mesh_storage.get(*mesh.id.read().unwrap()) {
                pass.set_bind_group(1, entity.bind_group.as_ref().unwrap(), &[]);
                pass.draw_indexed(0 .. mesh_asset.indices.len() as u32, 0, 0 .. 1);
            };

            last_mesh_id = Some(current_mesh_id); 
        }
    }
    fn resize(&mut self, render_graph: &RenderGraphData) {

    }
    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}