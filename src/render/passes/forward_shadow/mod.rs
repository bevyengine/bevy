use crate::{render::*, render::passes::shadow, asset::*, render::mesh::*};
use legion::prelude::*;
use wgpu::SwapChainOutput;

pub struct ForwardShadowPassNew {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub depth_format: wgpu::TextureFormat,
}

impl ForwardShadowPassNew {
    pub fn new() -> Self {
        ForwardShadowPassNew {
            pipeline: None,
            bind_group: None,
            depth_format: wgpu::TextureFormat::Depth32Float,
        }
    }
}

impl Pipeline for ForwardShadowPassNew {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, _world: &mut World) {
        let vs_bytes = shader::load_glsl(
            include_str!("forward_shadow.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("forward_shadow.frag"),
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

        self.bind_group = Some({
            let forward_uniform_buffer = render_graph.get_uniform_buffer(render_resources::FORWARD_UNIFORM_BUFFER_NAME).unwrap();
            let light_uniform_buffer = render_graph.get_uniform_buffer(render_resources::LIGHT_UNIFORM_BUFFER_NAME).unwrap();
            let shadow_sampler = render_graph.get_sampler(shadow::SHADOW_SAMPLER_NAME).unwrap();
            let shadow_texture = render_graph.get_texture(shadow::SHADOW_TEXTURE_NAME).unwrap();

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
                    },
                    wgpu::Binding {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(shadow_texture),
                    },
                    wgpu::Binding {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(shadow_sampler),
                    },
                ],
            })
        });

        let material_bind_group_layout = render_graph.get_bind_group_layout(render_resources::MATERIAL_BIND_GROUP_LAYOUT_NAME).unwrap();

        let pipeline_layout = render_graph.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout, material_bind_group_layout],
        });

        let vs_module = render_graph.device.create_shader_module(&vs_bytes);
        let fs_module = render_graph.device.create_shader_module(&fs_bytes);

        let vertex_buffer_descriptor = get_vertex_buffer_descriptor();

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

    fn render(&mut self, render_graph: &RenderGraphData, pass: &mut wgpu::RenderPass, _swap_chain_output: &SwapChainOutput, world: &mut World) {
        let mut mesh_query = <(Read<Material>, Read<Handle<Mesh>>)>::query();
        pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        for (material, mesh) in mesh_query.iter_immutable(world) {
            if let Some(mesh_asset) = mesh_storage.get(*mesh.id.read().unwrap()) {
                mesh_asset.setup_buffers(&render_graph.device);
                pass.set_bind_group(1, material.bind_group.as_ref().unwrap(), &[]);
                pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                pass.draw_indexed(0 .. mesh_asset.indices.len() as u32, 0, 0 .. 1);
            };
        }
    }

    fn resize(&mut self, _render_graph: &RenderGraphData) {
    }

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}