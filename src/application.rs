use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use zerocopy::{AsBytes, FromBytes};

use std::rc::Rc;
use std::mem;

use crate::temp::*;
use crate::vertex::*;

#[cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::mem::size_of;
    use std::slice::from_raw_parts;

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

pub struct Application
{
    entities: Vec<Entity>,
    lights: Vec<Light>,
    lights_are_dirty: bool,
    shadow_pass: Pass,
    forward_pass: Pass,
    forward_depth: wgpu::TextureView,
    light_uniform_buf: wgpu::Buffer,
}

impl Application {
    const MAX_LIGHTS: usize = 10;
    const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    const SHADOW_SIZE: wgpu::Extent3d = wgpu::Extent3d {
        width: 512,
        height: 512,
        depth: 1,
    };
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
    ) -> (Self, Option<wgpu::CommandBuffer>)
    {
        let vertex_size = mem::size_of::<Vertex>();
        let (cube_vertex_data, cube_index_data) = create_cube();
        let cube_vertex_buf = Rc::new(
            device.create_buffer_with_data(cube_vertex_data.as_bytes(), wgpu::BufferUsage::VERTEX),
        );

        let cube_index_buf = Rc::new(
            device.create_buffer_with_data(cube_index_data.as_bytes(), wgpu::BufferUsage::INDEX),
        );

        let (plane_vertex_data, plane_index_data) = create_plane(7);
        let plane_vertex_buf =
            device.create_buffer_with_data(plane_vertex_data.as_bytes(), wgpu::BufferUsage::VERTEX);

        let plane_index_buf =
            device.create_buffer_with_data(plane_index_data.as_bytes(), wgpu::BufferUsage::INDEX);

        let entity_uniform_size = mem::size_of::<EntityUniforms>() as wgpu::BufferAddress;
        let plane_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            size: entity_uniform_size,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let local_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            });

        let mut entities = vec![{
            use cgmath::SquareMatrix;

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &local_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &plane_uniform_buf,
                        range: 0 .. entity_uniform_size,
                    },
                }],
            });
            Entity {
                mx_world: cgmath::Matrix4::identity(),
                rotation_speed: 0.0,
                color: wgpu::Color::WHITE,
                vertex_buf: Rc::new(plane_vertex_buf),
                index_buf: Rc::new(plane_index_buf),
                index_count: plane_index_data.len(),
                bind_group,
                uniform_buf: plane_uniform_buf,
            }
        }];

        struct CubeDesc {
            offset: cgmath::Vector3<f32>,
            angle: f32,
            scale: f32,
            rotation: f32,
        }
        let cube_descs = [
            CubeDesc {
                offset: cgmath::vec3(-2.0, -2.0, 2.0),
                angle: 10.0,
                scale: 0.7,
                rotation: 0.1,
            },
            CubeDesc {
                offset: cgmath::vec3(2.0, -2.0, 2.0),
                angle: 50.0,
                scale: 1.3,
                rotation: 0.2,
            },
            CubeDesc {
                offset: cgmath::vec3(-2.0, 2.0, 2.0),
                angle: 140.0,
                scale: 1.1,
                rotation: 0.3,
            },
            CubeDesc {
                offset: cgmath::vec3(2.0, 2.0, 2.0),
                angle: 210.0,
                scale: 0.9,
                rotation: 0.4,
            },
        ];

        for cube in &cube_descs {
            use cgmath::{Decomposed, Deg, InnerSpace, Quaternion, Rotation3};

            let transform = Decomposed {
                disp: cube.offset.clone(),
                rot: Quaternion::from_axis_angle(cube.offset.normalize(), Deg(cube.angle)),
                scale: cube.scale,
            };
            let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
                size: entity_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });
            entities.push(Entity {
                mx_world: cgmath::Matrix4::from(transform),
                rotation_speed: cube.rotation,
                color: wgpu::Color::GREEN,
                vertex_buf: Rc::clone(&cube_vertex_buf),
                index_buf: Rc::clone(&cube_index_buf),
                index_count: cube_index_data.len(),
                bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &local_bind_group_layout,
                    bindings: &[wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0 .. entity_uniform_size,
                        },
                    }],
                }),
                uniform_buf,
            });
        }

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

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: Self::SHADOW_SIZE,
            array_layer_count: Self::MAX_LIGHTS as u32,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::SHADOW_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });
        let shadow_view = shadow_texture.create_default_view();

        let mut shadow_target_views = (0 .. 2)
            .map(|i| {
                Some(shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Self::SHADOW_FORMAT,
                    dimension: wgpu::TextureViewDimension::D2,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: i as u32,
                    array_layer_count: 1,
                }))
            })
            .collect::<Vec<_>>();
        let lights = vec![
            Light {
                pos: cgmath::Point3::new(7.0, -5.0, 10.0),
                color: wgpu::Color {
                    r: 0.5,
                    g: 1.0,
                    b: 0.5,
                    a: 1.0,
                },
                fov: 60.0,
                depth: 1.0 .. 20.0,
                target_view: shadow_target_views[0].take().unwrap(),
            },
            Light {
                pos: cgmath::Point3::new(-5.0, 7.0, 10.0),
                color: wgpu::Color {
                    r: 1.0,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                },
                fov: 45.0,
                depth: 1.0 .. 20.0,
                target_view: shadow_target_views[1].take().unwrap(),
            },
        ];
        let light_uniform_size =
            (Self::MAX_LIGHTS * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;
        let light_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            size: light_uniform_size,
            usage: wgpu::BufferUsage::UNIFORM
                | wgpu::BufferUsage::COPY_SRC
                | wgpu::BufferUsage::COPY_DST,
        });

        let vb_desc = wgpu::VertexBufferDescriptor {
            stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Char4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Char4,
                    offset: 4 * 1,
                    shader_location: 1,
                },
            ],
        };

        let shadow_pass = {
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

            // Create the render pipeline
            let vs_bytes =
                load_glsl(include_str!("render/bake/bake.vert"), ShaderStage::Vertex);
            let fs_bytes =
                load_glsl(include_str!("render/bake/bake.frag"), ShaderStage::Fragment);
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
                vertex_buffers: &[vb_desc.clone()],
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

            Pass {
                pipeline,
                bind_group,
                uniform_buf,
            }
        };

        let forward_pass = {
            // Create pipeline layout
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
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&bind_group_layout, &local_bind_group_layout],
            });

            let mx_total = generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
            let forward_uniforms = ForwardUniforms {
                proj: *mx_total.as_ref(),
                num_lights: [lights.len() as u32, 0, 0, 0],
            };
            let uniform_size = mem::size_of::<ForwardUniforms>() as wgpu::BufferAddress;
            let uniform_buf = device.create_buffer_with_data(
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
                            buffer: &uniform_buf,
                            range: 0 .. uniform_size,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &light_uniform_buf,
                            range: 0 .. light_uniform_size,
                        },
                    },
                    wgpu::Binding {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&shadow_view),
                    },
                    wgpu::Binding {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                    },
                ],
            });

            // Create the render pipeline
            let vs_bytes =
                load_glsl(include_str!("render/forward/forward.vert"), ShaderStage::Vertex);
            let fs_bytes =
                load_glsl(include_str!("render/forward/forward.frag"), ShaderStage::Fragment);

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
                color_states: &[wgpu::ColorStateDescriptor {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
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
                vertex_buffers: &[vb_desc],
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

            Pass {
                pipeline,
                bind_group,
                uniform_buf,
            }
        };

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: sc_desc.width,
                height: sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        let this = Application {
            entities,
            lights,
            lights_are_dirty: true,
            shadow_pass,
            forward_pass,
            forward_depth: depth_texture.create_default_view(),
            light_uniform_buf,
        };
        (this, None)
    }

    fn resize(
        &mut self,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
    ) -> Option<wgpu::CommandBuffer>
    {
        let command_buf = {
            let mx_total = generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
            let mx_ref: &[f32; 16] = mx_total.as_ref();
            let temp_buf =
                device.create_buffer_with_data(mx_ref.as_bytes(), wgpu::BufferUsage::COPY_SRC);

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
            encoder.copy_buffer_to_buffer(&temp_buf, 0, &self.forward_pass.uniform_buf, 0, 64);
            encoder.finish()
        };

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: sc_desc.width,
                height: sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });
        self.forward_depth = depth_texture.create_default_view();

        Some(command_buf)
    }

    fn update(&mut self, event: WindowEvent)
    {
    }

    fn render(
        &mut self,
        frame: &wgpu::SwapChainOutput,
        device: &wgpu::Device,
    ) -> wgpu::CommandBuffer
    {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        {
            let size = mem::size_of::<EntityUniforms>();
            let temp_buf_data = device
                .create_buffer_mapped(self.entities.len() * size, wgpu::BufferUsage::COPY_SRC);

            // FIXME: Align and use `LayoutVerified`
            for (entity, slot) in self
                .entities
                .iter_mut()
                .zip(temp_buf_data.data.chunks_exact_mut(size))
            {
                if entity.rotation_speed != 0.0 {
                    let rotation =
                        cgmath::Matrix4::from_angle_x(cgmath::Deg(entity.rotation_speed));
                    entity.mx_world = entity.mx_world * rotation;
                }
                slot.copy_from_slice(
                    EntityUniforms {
                        model: entity.mx_world.into(),
                        color: [
                            entity.color.r as f32,
                            entity.color.g as f32,
                            entity.color.b as f32,
                            entity.color.a as f32,
                        ],
                    }
                    .as_bytes(),
                );
            }

            let temp_buf = temp_buf_data.finish();

            for (i, entity) in self.entities.iter().enumerate() {
                encoder.copy_buffer_to_buffer(
                    &temp_buf,
                    (i * size) as wgpu::BufferAddress,
                    &entity.uniform_buf,
                    0,
                    size as wgpu::BufferAddress,
                );
            }
        }

        if self.lights_are_dirty {
            self.lights_are_dirty = false;
            let size = mem::size_of::<LightRaw>();
            let total_size = size * self.lights.len();
            let temp_buf_data =
                device.create_buffer_mapped(total_size, wgpu::BufferUsage::COPY_SRC);
            // FIXME: Align and use `LayoutVerified`
            for (light, slot) in self
                .lights
                .iter()
                .zip(temp_buf_data.data.chunks_exact_mut(size))
            {
                slot.copy_from_slice(light.to_raw().as_bytes());
            }
            encoder.copy_buffer_to_buffer(
                &temp_buf_data.finish(),
                0,
                &self.light_uniform_buf,
                0,
                total_size as wgpu::BufferAddress,
            );
        }

        for (i, light) in self.lights.iter().enumerate() {
            // The light uniform buffer already has the projection,
            // let's just copy it over to the shadow uniform buffer.
            encoder.copy_buffer_to_buffer(
                &self.light_uniform_buf,
                (i * mem::size_of::<LightRaw>()) as wgpu::BufferAddress,
                &self.shadow_pass.uniform_buf,
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
            pass.set_pipeline(&self.shadow_pass.pipeline);
            pass.set_bind_group(0, &self.shadow_pass.bind_group, &[]);

            for entity in &self.entities {
                pass.set_bind_group(1, &entity.bind_group, &[]);
                pass.set_index_buffer(&entity.index_buf, 0);
                pass.set_vertex_buffers(0, &[(&entity.vertex_buf, 0)]);
                pass.draw_indexed(0 .. entity.index_count as u32, 0, 0 .. 1);
            }
        }

        // forward pass
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.forward_depth,
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
            });
            pass.set_pipeline(&self.forward_pass.pipeline);
            pass.set_bind_group(0, &self.forward_pass.bind_group, &[]);

            for entity in &self.entities {
                pass.set_bind_group(1, &entity.bind_group, &[]);
                pass.set_index_buffer(&entity.index_buf, 0);
                pass.set_vertex_buffers(0, &[(&entity.vertex_buf, 0)]);
                pass.draw_indexed(0 .. entity.index_count as u32, 0, 0 .. 1);
            }
        }

        encoder.finish()
    }

    #[allow(dead_code)]
    pub fn run() {
        env_logger::init();
        let event_loop = EventLoop::new();
        log::info!("Initializing the window...");
    
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
            },
            wgpu::BackendBit::PRIMARY,
        )
        .unwrap();
    
        let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let (_window, hidpi_factor, size, surface) = {
            let window = winit::window::Window::new(&event_loop).unwrap();
            window.set_title("bevy");
            let hidpi_factor = window.hidpi_factor();
            let size = window.inner_size().to_physical(hidpi_factor);
            let surface = wgpu::Surface::create(&window);
            (window, hidpi_factor, size, surface)
        };
    
        let mut sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);
    
        log::info!("Initializing the example...");
        let (mut example, init_command_buf) = Application::init(&sc_desc, &device);
        if let Some(command_buf) = init_command_buf {
            queue.submit(&[command_buf]);
        }
    
        log::info!("Entering render loop...");
        event_loop.run(move |event, _, control_flow| {
            *control_flow = if cfg!(feature = "metal-auto-capture") {
                ControlFlow::Exit
            } else {
                ControlFlow::Poll
            };
            match event {
                event::Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let physical = size.to_physical(hidpi_factor);
                    log::info!("Resizing to {:?}", physical);
                    sc_desc.width = physical.width.round() as u32;
                    sc_desc.height = physical.height.round() as u32;
                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
                    let command_buf = example.resize(&sc_desc, &device);
                    if let Some(command_buf) = command_buf {
                        queue.submit(&[command_buf]);
                    }
                }
                event::Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(event::VirtualKeyCode::Escape),
                                state: event::ElementState::Pressed,
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {
                        example.update(event);
                    }
                },
                event::Event::EventsCleared => {
                    let frame = swap_chain
                        .get_next_texture()
                        .expect("Timeout when acquiring next swap chain texture");
                    let command_buf = example.render(&frame, &device);
                    queue.submit(&[command_buf]);
                }
                _ => (),
            }
        }); 
    }
}