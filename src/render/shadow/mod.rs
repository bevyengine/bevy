use crate::{render::*, asset::*};
use wgpu::{BindGroupLayout, CommandEncoder, Device, VertexBufferDescriptor, SwapChainOutput};
use legion::prelude::*;
use zerocopy::AsBytes;
use std::{mem, sync::Arc};

pub struct ShadowPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buf: wgpu::Buffer,
    pub shadow_texture: wgpu::Texture,
    pub shadow_view: wgpu::TextureView,
    pub shadow_sampler: wgpu::Sampler,
    pub light_uniform_buffer: Arc::<UniformBuffer>,
    pub lights_are_dirty: bool,
}

#[repr(C)]
pub struct ShadowUniforms {
    pub proj: [[f32; 4]; 4],
}

impl Pass for ShadowPass {
    fn render(&mut self, device: &Device, _: &SwapChainOutput, encoder: &mut CommandEncoder, world: &mut World) {
        let mut light_query = <Read<Light>>::query();
        let mut mesh_query = <(Read<Material>, Read<Handle<Mesh>>)>::query();
        let light_count = light_query.iter(world).count();

        if self.lights_are_dirty {
            self.lights_are_dirty = false;
            let size = mem::size_of::<LightRaw>();
            let total_size = size * light_count;
            let temp_buf_data =
                device.create_buffer_mapped(total_size, wgpu::BufferUsage::COPY_SRC);
            for (light, slot) in light_query
                .iter(world)
                .zip(temp_buf_data.data.chunks_exact_mut(size))
            {
                slot.copy_from_slice(light.to_raw().as_bytes());
            }
            encoder.copy_buffer_to_buffer(
                &temp_buf_data.finish(),
                0,
                &self.light_uniform_buffer.buffer,
                0,
                total_size as wgpu::BufferAddress,
            );
        }

        for (i, light) in light_query.iter_immutable(world).enumerate() {
            // The light uniform buffer already has the projection,
            // let's just copy it over to the shadow uniform buffer.
            encoder.copy_buffer_to_buffer(
                &self.light_uniform_buffer.buffer,
                (i * mem::size_of::<LightRaw>()) as wgpu::BufferAddress,
                &self.uniform_buf,
                0,
                64,
            );

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &light.target_view,
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
            for (entity, mesh) in mesh_query.iter_immutable(world) {
                if let Some(mesh_asset) = mesh_storage.get(*mesh.id.read().unwrap()) {
                    mesh_asset.setup_buffers(device);

                    pass.set_bind_group(1, entity.bind_group.as_ref().unwrap(), &[]);
                    pass.set_index_buffer(&mesh_asset.index_buffer.as_ref().unwrap(), 0);
                    pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                    pass.draw_indexed(0 .. mesh_asset.indices.len() as u32, 0, 0 .. 1);

                };
            }
        }
    }
}

impl ShadowPass {
    pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub const SHADOW_SIZE: wgpu::Extent3d = wgpu::Extent3d {
        width: 512,
        height: 512,
        depth: 1,
    };

    pub fn new(device: &Device, light_uniform_buffer: Arc::<UniformBuffer>, vertex_buffer_descriptor: VertexBufferDescriptor, local_bind_group_layout: &BindGroupLayout, max_lights: u32) -> ShadowPass {
        // Create pipeline layout
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0, // global
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout, &local_bind_group_layout],
        });

        let uniform_size = mem::size_of::<ShadowUniforms>() as wgpu::BufferAddress;
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            size: uniform_size,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniform_buf,
                    range: 0 .. uniform_size,
                },
            }],
        });

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: Self::SHADOW_SIZE,
            array_layer_count: max_lights,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::SHADOW_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });

        let shadow_view = shadow_texture.create_default_view();

        // Create other resources
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
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

        // Create the render pipeline
        let vs_bytes =
            shader::load_glsl(include_str!("shadow.vert"), shader::ShaderStage::Vertex);
        let fs_bytes =
            shader::load_glsl(include_str!("shadow.frag"), shader::ShaderStage::Fragment);
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
                depth_bias: 2, // corresponds to bilinear filtering
                depth_bias_slope_scale: 2.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: Self::SHADOW_FORMAT,
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
        });

        ShadowPass {
            pipeline,
            bind_group,
            uniform_buf,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            light_uniform_buffer,
            lights_are_dirty: true,
        }
    }
}
