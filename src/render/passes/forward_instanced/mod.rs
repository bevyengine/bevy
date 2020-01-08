use crate::{render::*, asset::*, render::mesh::*, LocalToWorld};
use legion::prelude::*;
use std::mem;
use zerocopy::AsBytes;
use wgpu::{Device, SwapChainOutput};

pub struct InstanceBufferInfo {
    pub buffer: wgpu::Buffer,
    pub instance_count: usize,
    pub mesh_id: usize,
}

pub struct ForwardInstancedPipeline {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub depth_format: wgpu::TextureFormat,
    pub local_bind_group: Option<wgpu::BindGroup>,
    pub instance_buffer_infos: Option<Vec<InstanceBufferInfo>>,
}

impl ForwardInstancedPipeline {
    pub fn new(depth_format: wgpu::TextureFormat) -> Self {
        ForwardInstancedPipeline {
            pipeline: None,
            depth_format,
            local_bind_group: None,
            instance_buffer_infos: None,
        }
    }

    fn create_instance_buffer_infos(device: &Device, world: &World) -> Vec<InstanceBufferInfo> {
        let mut entities = <(Read<Material>, Read<LocalToWorld>, Read<Handle<Mesh>>, Read<Instanced>)>::query();
        let entities_count = entities.iter_immutable(world).count();
        let size = mem::size_of::<SimpleMaterialUniforms>();

        // TODO: use a staging buffer for more efficient gpu reads
        let temp_buf_data = device
            .create_buffer_mapped(entities_count * size, wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::VERTEX);

        // TODO: generate these buffers for multiple meshes
        
        let mut last_mesh_id = None;
        for ((material, transform, mesh, _), slot) in entities.iter_immutable(world)
            .zip(temp_buf_data.data.chunks_exact_mut(size))
        {

            last_mesh_id = Some(*mesh.id.read().unwrap());
            let (_, _, translation) = transform.0.to_scale_rotation_translation();
            slot.copy_from_slice(
                SimpleMaterialUniforms {
                    position: translation.into(),
                    color: material.color.into(),
                }
                .as_bytes(),
            );
        }
        
        let mut instance_buffer_infos = Vec::new();
        instance_buffer_infos.push(InstanceBufferInfo {
            mesh_id: last_mesh_id.unwrap(),
            buffer: temp_buf_data.finish(),
            instance_count: entities_count,
        });

        instance_buffer_infos
    }

    #[allow(dead_code)]
    fn create_instance_buffer_infos_direct(device: &Device, world: &World) -> Vec<InstanceBufferInfo> {
        let mut entities = <(Read<Material>, Read<LocalToWorld>, Read<Handle<Mesh>>, Read<Instanced>)>::query();
        let entities_count = entities.iter_immutable(world).count();

        let mut last_mesh_id = None;
        let mut data = Vec::with_capacity(entities_count);
        for (material, transform, mesh, _) in entities.iter_immutable(world)
        {

            last_mesh_id = Some(*mesh.id.read().unwrap());
            let (_, _, translation) = transform.0.to_scale_rotation_translation();

            data.push(SimpleMaterialUniforms {
                position: translation.into(),
                color: material.color.into(),
            });
        }

        let buffer = device
            .create_buffer_with_data(data.as_bytes(), wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::VERTEX);
        
    
        let mut instance_buffer_infos = Vec::new();
        instance_buffer_infos.push(InstanceBufferInfo {
            mesh_id: last_mesh_id.unwrap(),
            buffer: buffer,
            instance_count: entities_count,
        });

        instance_buffer_infos
    }
}

impl Pipeline for ForwardInstancedPipeline {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, world: &mut World) {
        let vs_bytes = shader::load_glsl(
            include_str!("forward_instanced.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("forward_instanced.frag"),
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
        
        // TODO: this is the same as normal forward pipeline. we can probably reuse
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

        let simple_material_uniforms_size = mem::size_of::<SimpleMaterialUniforms>();
        let instance_buffer_descriptor = wgpu::VertexBufferDescriptor {
            stride: simple_material_uniforms_size as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float3,
                    offset: 0,
                    shader_location: 2,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 3 * 4,
                    shader_location: 3,
                },
            ],
        };

        let vertex_buffer_descriptor = get_vertex_buffer_descriptor();

        let pipeline_layout = render_graph.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

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
            vertex_buffers: &[vertex_buffer_descriptor, instance_buffer_descriptor],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        }));

        self.instance_buffer_infos = Some(Self::create_instance_buffer_infos(&render_graph.device, world));
    }

    fn render(&mut self, render_graph: &RenderGraphData, pass: &mut wgpu::RenderPass, _: &SwapChainOutput, world: &mut World) {
        self.instance_buffer_infos = Some(Self::create_instance_buffer_infos(&render_graph.device, world));
        pass.set_bind_group(0, self.local_bind_group.as_ref().unwrap(), &[]);

        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        for instance_buffer_info in self.instance_buffer_infos.as_ref().unwrap().iter() {
            if let Some(mesh_asset) = mesh_storage.get(instance_buffer_info.mesh_id) {
                mesh_asset.setup_buffers(&render_graph.device);
                pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                pass.set_vertex_buffers(1, &[(&instance_buffer_info.buffer, 0)]);
                pass.draw_indexed(0 .. mesh_asset.indices.len() as u32, 0, 0 .. instance_buffer_info.instance_count as u32);
            };
        }
    }

    fn resize(&mut self, _: &RenderGraphData) {

    }

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}