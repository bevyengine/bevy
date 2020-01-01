use crate::{render::*, asset::*, render::mesh::*, math, LocalToWorld};
use legion::prelude::*;
use std::mem;
use zerocopy::AsBytes;
use wgpu::{Buffer, CommandEncoder, Device, VertexBufferDescriptor, SwapChainDescriptor, SwapChainOutput};


pub struct InstanceBufferInfo {
    pub buffer: wgpu::Buffer,
    pub instance_count: usize,
    pub mesh_id: usize,
}

pub struct ForwardInstancedPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub forward_uniform_buffer: wgpu::Buffer,
    pub depth_texture: wgpu::TextureView,
    pub instance_buffer_infos: Vec<InstanceBufferInfo>,
}

impl Pipeline for ForwardInstancedPass {
    fn render(&mut self, device: &Device, frame: &SwapChainOutput, encoder: &mut CommandEncoder, world: &mut World, _: &RenderResources) { 
        self.instance_buffer_infos = ForwardInstancedPass::create_instance_buffer_infos(device, world);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                attachment: &self.depth_texture,
                depth_load_op: wgpu::LoadOp::Clear,
                depth_store_op: wgpu::StoreOp::Store,
                stencil_load_op: wgpu::LoadOp::Clear,
                stencil_store_op: wgpu::StoreOp::Store,
                clear_depth: 1.0,
                clear_stencil: 0,
            }),
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        for instance_buffer_info in self.instance_buffer_infos.iter() {
            if let Some(mesh_asset) = mesh_storage.get(instance_buffer_info.mesh_id) {
                mesh_asset.setup_buffers(device);
                pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                pass.set_vertex_buffers(1, &[(&instance_buffer_info.buffer, 0)]);
                pass.draw_indexed(0 .. mesh_asset.indices.len() as u32, 0, 0 .. instance_buffer_info.instance_count as u32);
            };
        }
    }
    
    fn resize(&mut self, device: &Device, frame: &SwapChainDescriptor) {
        self.depth_texture = Self::get_depth_texture(device, frame);
    }

    fn get_camera_uniform_buffer(&self) -> Option<&Buffer> { 
        Some(&self.forward_uniform_buffer)
    }
}

impl ForwardInstancedPass {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
 
    pub fn new(device: &Device, world: &World, render_resources: &RenderResources, vertex_buffer_descriptor: VertexBufferDescriptor, swap_chain_descriptor: &SwapChainDescriptor) -> Self {
        let vs_bytes = shader::load_glsl(
            include_str!("forward_instanced.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("forward_instanced.frag"),
            shader::ShaderStage::Fragment,
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
                }
            ],
        });

        let light_count = <Read<Light>>::query().iter_immutable(world).count();
        let forward_uniforms = ForwardUniforms {
            proj: math::Mat4::identity().to_cols_array_2d(),
            num_lights: [light_count as u32, 0, 0, 0],
        };

        let uniform_size = mem::size_of::<ForwardUniforms>() as wgpu::BufferAddress;
        let forward_uniform_buffer = device.create_buffer_with_data(
            forward_uniforms.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

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
                        buffer: &render_resources.light_uniform_buffer.buffer,
                        range: 0 .. render_resources.light_uniform_buffer.size,
                    },
                }
            ],
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
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
            vertex_buffers: &[vertex_buffer_descriptor, instance_buffer_descriptor],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let instance_buffer_infos = ForwardInstancedPass::create_instance_buffer_infos(device, world);

        ForwardInstancedPass {
            pipeline,
            bind_group,
            forward_uniform_buffer,
            depth_texture: Self::get_depth_texture(device, swap_chain_descriptor),
            instance_buffer_infos
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

