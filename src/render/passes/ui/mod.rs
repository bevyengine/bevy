use crate::{render::*, asset::*, render::mesh::*};
use legion::prelude::*;
use wgpu::SwapChainOutput;

pub struct UiPipeline {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub depth_format: wgpu::TextureFormat,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl UiPipeline {
    pub fn new() -> Self {
        UiPipeline {
            pipeline: None,
            bind_group: None,
            depth_format: wgpu::TextureFormat::Depth32Float
        }
    }
}

impl Pipeline for UiPipeline {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, _: &mut World) {
        let vs_bytes = shader::load_glsl(
            include_str!("ui.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("ui.frag"),
            shader::ShaderStage::Fragment,
        );

        let bind_group_layout =
        render_graph.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0, // global_2d
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
            ],
        });

        self.bind_group = Some({

            let global_2d_uniform_buffer = render_graph.get_uniform_buffer(render_resources::GLOBAL_2D_UNIFORM_BUFFER_NAME).unwrap();

            // Create bind group
            render_graph.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: global_2d_uniform_buffer.get_binding_resource(),
                    },
                ],
            })
        });

        let pipeline_layout = render_graph.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
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
                cull_mode: wgpu::CullMode::None,
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

    fn render(&mut self, render_graph: &RenderGraphData, pass: &mut wgpu::RenderPass, _: &SwapChainOutput, world: &mut World) {
        pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        let mut last_mesh_id = None;
        let mut mesh_query =
            <(Read<Handle<Mesh>>, Read<Mesh2d>)>::query()
            .filter(!component::<Instanced>());
        for (mesh, _) in mesh_query.iter_immutable(world) {
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
                pass.draw_indexed(0 .. mesh_asset.indices.len() as u32, 0, 0 .. 1);
            };

            last_mesh_id = Some(current_mesh_id); 
        }
    }
    
    fn resize(&mut self, _: &RenderGraphData) {
    }

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}