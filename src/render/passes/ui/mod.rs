use crate::{
    asset::*,
    math,
    render::mesh::*,
    render::{instancing::InstanceBufferInfo, *},
};
use legion::prelude::*;
use wgpu::SwapChainOutput;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct RectData {
    pub position: [f32; 2],
    pub dimensions: [f32; 2],
    pub color: [f32; 4],
    pub z_index: f32,
}

pub struct UiPipeline {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub depth_format: wgpu::TextureFormat,
    pub quad: Option<Handle<Mesh>>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl UiPipeline {
    pub fn new() -> Self {
        UiPipeline {
            pipeline: None,
            bind_group: None,
            quad: None,
            depth_format: wgpu::TextureFormat::Depth32Float,
        }
    }

    pub fn create_rect_buffers(
        &self,
        device: &wgpu::Device,
        world: &World,
    ) -> Vec<InstanceBufferInfo> {
        let mut rect_query = <Read<Rect>>::query();
        let rect_count = rect_query.iter_immutable(world).count();

        if rect_count == 0 {
            return Vec::new();
        }

        let mut data = Vec::with_capacity(rect_count);
        // TODO: this probably isn't the best way to handle z-ordering
        let mut z = 0.9999;
        for rect in rect_query.iter_immutable(world) {
            data.push(RectData {
                position: rect.position.into(),
                dimensions: rect.dimensions.into(),
                color: rect.color.into(),
                z_index: z,
            });

            z -= 0.0001;
        }

        let buffer = device.create_buffer_with_data(
            data.as_bytes(),
            wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::VERTEX,
        );

        let mesh_id = *self.quad.as_ref().unwrap().id.read().unwrap();

        let mut instance_buffer_infos = Vec::new();
        instance_buffer_infos.push(InstanceBufferInfo {
            mesh_id: mesh_id,
            buffer: buffer,
            instance_count: rect_count,
        });

        instance_buffer_infos
    }
}

impl Pipeline for UiPipeline {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, world: &mut World) {
        let vs_bytes = shader::load_glsl(include_str!("ui.vert"), shader::ShaderStage::Vertex);
        let fs_bytes = shader::load_glsl(include_str!("ui.frag"), shader::ShaderStage::Fragment);

        let bind_group_layout =
            render_graph
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    bindings: &[wgpu::BindGroupLayoutBinding {
                        binding: 0, // global_2d
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    }],
                });

        self.bind_group = Some({
            let global_2d_uniform_buffer = render_graph
                .get_uniform_buffer(render_resources::GLOBAL_2D_UNIFORM_BUFFER_NAME)
                .unwrap();

            // Create bind group
            render_graph
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &bind_group_layout,
                    bindings: &[wgpu::Binding {
                        binding: 0,
                        resource: global_2d_uniform_buffer.get_binding_resource(),
                    }],
                })
        });

        {
            let mut mesh_storage = world
                .resources
                .get_mut::<AssetStorage<Mesh, MeshType>>()
                .unwrap();

            let quad = Mesh::load(MeshType::Quad {
                north_west: math::vec2(-0.5, 0.5),
                north_east: math::vec2(0.5, 0.5),
                south_west: math::vec2(-0.5, -0.5),
                south_east: math::vec2(0.5, -0.5),
            });
            self.quad = Some(mesh_storage.add(quad, "ui_quad"));
        }

        let pipeline_layout =
            render_graph
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[&bind_group_layout],
                });

        let vertex_buffer_descriptor = get_vertex_buffer_descriptor();
        let rect_data_size = mem::size_of::<RectData>();
        let instance_buffer_descriptor = wgpu::VertexBufferDescriptor {
            stride: rect_data_size as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float2,
                    offset: 0,
                    shader_location: 2,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float2,
                    offset: 2 * 4,
                    shader_location: 3,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 4 * 4,
                    shader_location: 4,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float,
                    offset: 8 * 4,
                    shader_location: 5,
                },
            ],
        };

        let vs_module = render_graph.device.create_shader_module(&vs_bytes);
        let fs_module = render_graph.device.create_shader_module(&fs_bytes);

        self.pipeline = Some(render_graph.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
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
                color_states: &[wgpu::ColorStateDescriptor {
                    format: render_graph.swap_chain_descriptor.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
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
                vertex_buffers: &[vertex_buffer_descriptor, instance_buffer_descriptor],
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        ));
    }

    fn render(
        &mut self,
        render_graph: &RenderGraphData,
        pass: &mut wgpu::RenderPass,
        _: &SwapChainOutput,
        world: &mut World,
    ) {
        let instance_buffer_infos = Some(self.create_rect_buffers(&render_graph.device, world));
        pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

        let mut mesh_storage = world
            .resources
            .get_mut::<AssetStorage<Mesh, MeshType>>()
            .unwrap();
        for instance_buffer_info in instance_buffer_infos.as_ref().unwrap().iter() {
            if let Some(mesh_asset) = mesh_storage.get(instance_buffer_info.mesh_id) {
                mesh_asset.setup_buffers(&render_graph.device);
                pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                pass.set_vertex_buffers(1, &[(&instance_buffer_info.buffer, 0)]);
                pass.draw_indexed(
                    0..mesh_asset.indices.len() as u32,
                    0,
                    0..instance_buffer_info.instance_count as u32,
                );
            };
        }
    }

    fn resize(&mut self, _: &RenderGraphData) {}

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}
