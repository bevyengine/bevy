use crate::{asset::*, render::*, LocalToWorld, Translation};
use legion::prelude::*;
use std::mem;
use wgpu::{
    Buffer, CommandEncoder, Device, SwapChainDescriptor, SwapChainOutput, VertexBufferDescriptor,
};

pub struct ShadowPassOld {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buf: wgpu::Buffer,
    pub shadow_texture: wgpu::Texture,
    pub shadow_view: wgpu::TextureView,
    pub shadow_sampler: wgpu::Sampler,
    pub lights_are_dirty: bool,
}

pub struct ShadowPass {
    pub shadow_size: wgpu::Extent3d,
    light_index: isize,
    shadow_texture: Option<wgpu::Texture>,
    shadow_format: wgpu::TextureFormat,
    pub max_lights: usize,
}

impl ShadowPass {
    pub fn new(shadow_size: wgpu::Extent3d, shadow_format: wgpu::TextureFormat, max_lights: usize) -> Self {
        ShadowPass {
            light_index: -1,
            shadow_texture: None,
            shadow_size,
            shadow_format,
            max_lights,
        }
    }
}

impl Pass for ShadowPass {
    fn initialize(&self, render_graph: &mut RenderGraphData) {
        let shadow_texture = render_graph.device.create_texture(&wgpu::TextureDescriptor {
            size: self.shadow_size,
            array_layer_count: self.max_lights as u32,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.shadow_format,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });

        let shadow_view = shadow_texture.create_default_view();
    }

    fn begin<'a>(
        &mut self,
        render_graph: &mut RenderGraphData,
        world: &mut World,
        encoder: &'a mut wgpu::CommandEncoder,
        frame: &'a wgpu::SwapChainOutput,
    ) -> Option<wgpu::RenderPass<'a>> {
        if self.light_index == -1 {
            self.light_index = 0;
        }

        let mut light_query = <(Write<Light>, Read<LocalToWorld>, Read<Translation>)>::query();
        let light_count = light_query.iter(world).count();
        for (i, (mut light, _, _)) in light_query.iter(world).enumerate() {
            if i != self.light_index as usize {
                continue;
            }

            if let None = light.target_view {
                light.target_view = Some(self.shadow_texture.as_ref().unwrap().create_view(
                    &wgpu::TextureViewDescriptor {
                        format: ShadowPassOld::SHADOW_FORMAT,
                        dimension: wgpu::TextureViewDimension::D2,
                        aspect: wgpu::TextureAspect::All,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: i as u32,
                        array_layer_count: 1,
                    },
                ));
            }

            // The light uniform buffer already has the projection,
            // let's just copy it over to the shadow uniform buffer.
            let light_uniform_buffer = render_graph.get_uniform_buffer(render_resources::LIGHT_UNIFORM_BUFFER_NAME).unwrap();
            let shadow_pipeline_uniform_buffer = render_graph.get_uniform_buffer(SHADOW_PIPELINE_UNIFORMS).unwrap();
            encoder.copy_buffer_to_buffer(
                &light_uniform_buffer.buffer,
                (i * mem::size_of::<LightRaw>()) as wgpu::BufferAddress,
                &shadow_pipeline_uniform_buffer.buffer,
                0,
                64,
            );

            self.light_index += 1;
            if self.light_index as usize == light_count {
                self.light_index = -1;
            }
            return Some(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: light.target_view.as_ref().unwrap(),
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
            }));
        }

        None
    }

    fn resize(&self, render_graph: &mut RenderGraphData) {}

    fn should_repeat(&self) -> bool {
        return self.light_index != -1;
    }
}

pub struct ShadowPipeline {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub shadow_format: wgpu::TextureFormat,
}

pub const SHADOW_PIPELINE_UNIFORMS: &str = "shadow_pipeline";

impl ShadowPipeline {
    pub fn new(shadow_format: wgpu::TextureFormat, shadow_size: wgpu::Extent3d, max_lights: usize) -> Self {
        ShadowPipeline {
            bind_group: None,
            pipeline: None,
            shadow_format,
        }
    }
}

impl PipelineNew for ShadowPipeline {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, world: &mut World) {
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

        let vertex_buffer_descriptor = get_vertex_buffer_descriptor();

        // Create the render pipeline
        let vs_bytes = shader::load_glsl(include_str!("shadow.vert"), shader::ShaderStage::Vertex);
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
        frame: &SwapChainOutput,
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

    fn resize(&mut self, render_graph: &RenderGraphData) {}

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline.as_ref().unwrap()
    }
}

#[repr(C)]
pub struct ShadowUniforms {
    pub proj: [[f32; 4]; 4],
}

impl Pipeline for ShadowPassOld {
    fn render(
        &mut self,
        device: &Device,
        _: &SwapChainOutput,
        encoder: &mut CommandEncoder,
        world: &mut World,
        render_resources: &RenderResources,
    ) {
        let mut light_query = <(Read<Light>, Read<LocalToWorld>, Read<Translation>)>::query();
        let mut mesh_query =
            <(Read<Material>, Read<Handle<Mesh>>)>::query().filter(!component::<Instanced>());

        for (i, (light, _, _)) in light_query.iter_immutable(world).enumerate() {
            // if let None = light.target_view {
            //     light.target_view = Some(self.shadow_texture.create_view(
            //         &wgpu::TextureViewDescriptor {
            //             format: ShadowPassOld::SHADOW_FORMAT,
            //             dimension: wgpu::TextureViewDimension::D2,
            //             aspect: wgpu::TextureAspect::All,
            //             base_mip_level: 0,
            //             level_count: 1,
            //             base_array_layer: i as u32,
            //             array_layer_count: 1,
            //         },
            //     ));
            // }

            // The light uniform buffer already has the projection,
            // let's just copy it over to the shadow uniform buffer.
            encoder.copy_buffer_to_buffer(
                &render_resources.light_uniform_buffer.buffer,
                (i * mem::size_of::<LightRaw>()) as wgpu::BufferAddress,
                &self.uniform_buf,
                0,
                64,
            );

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: light.target_view.as_ref().unwrap(),
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

            let mut mesh_storage = world
                .resources
                .get_mut::<AssetStorage<Mesh, MeshType>>()
                .unwrap();
            for (entity, mesh) in mesh_query.iter_immutable(world) {
                if let Some(mesh_asset) = mesh_storage.get(*mesh.id.read().unwrap()) {
                    mesh_asset.setup_buffers(device);

                    pass.set_bind_group(1, entity.bind_group.as_ref().unwrap(), &[]);
                    pass.set_index_buffer(&mesh_asset.index_buffer.as_ref().unwrap(), 0);
                    pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
                    pass.draw_indexed(0..mesh_asset.indices.len() as u32, 0, 0..1);
                };
            }
        }
    }

    fn resize(&mut self, _: &Device, _: &SwapChainDescriptor) {}
    fn get_camera_uniform_buffer(&self) -> Option<&Buffer> {
        None
    }
}

impl ShadowPassOld {
    pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub const SHADOW_SIZE: wgpu::Extent3d = wgpu::Extent3d {
        width: 512,
        height: 512,
        depth: 1,
    };

    pub fn new(
        device: &Device,
        _: &World,
        render_resources: &RenderResources,
        vertex_buffer_descriptor: VertexBufferDescriptor,
    ) -> ShadowPassOld {
        // Create pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0, // global
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &bind_group_layout,
                &render_resources.local_bind_group_layout,
            ],
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
                    range: 0..uniform_size,
                },
            }],
        });

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: Self::SHADOW_SIZE,
            array_layer_count: render_resources.max_lights as u32,
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
        let vs_bytes = shader::load_glsl(include_str!("shadow.vert"), shader::ShaderStage::Vertex);
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

        ShadowPassOld {
            pipeline,
            bind_group,
            uniform_buf,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            lights_are_dirty: true,
        }
    }
}
