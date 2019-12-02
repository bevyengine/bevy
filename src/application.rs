use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use zerocopy::AsBytes;
use legion::prelude::*;

use std::sync::Arc;
use std::mem;

use crate::{temp::*, vertex::*, render::*, math, LocalToWorld, Translation, ApplicationStage};

pub struct Application
{
    pub universe: Universe,
    pub world: World,
    pub scheduler: SystemScheduler<ApplicationStage>,
    pub shadow_pass: ShadowPass,
    pub forward_pass: ForwardPass,
    camera_position: math::Vec3,
    camera_fov: f32,
}

impl Application {
    pub const MAX_LIGHTS: usize = 10;

    fn init(
        universe: Universe,
        mut world: World,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
    ) -> (Self, Option<wgpu::CommandBuffer>)
    {
        let vertex_size = mem::size_of::<Vertex>();

        let local_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            });


        let mut entities = <Write<CubeEnt>>::query();
        for mut entity in entities.iter(&mut world) {
            let entity_uniform_size = mem::size_of::<EntityUniforms>() as wgpu::BufferAddress;
            let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
                size: entity_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &local_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &uniform_buf,
                        range: 0 .. entity_uniform_size,
                    },
                }],
            });

            entity.bind_group = Some(bind_group);
            entity.uniform_buf = Some(uniform_buf);
        }

        let camera_position = math::vec3(3.0f32, -10.0, 6.0);
        let camera_fov = math::quarter_pi();

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

        let light_uniform_size =
        (Self::MAX_LIGHTS * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;

        let light_uniform_buffer = Arc::new(UniformBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                size: light_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::COPY_SRC
                    | wgpu::BufferUsage::COPY_DST,
            }),
            size: light_uniform_size,
        });

        let shadow_pass = ShadowPass::new(device, light_uniform_buffer.clone(), vb_desc.clone(), &local_bind_group_layout, Self::MAX_LIGHTS as u32);
        
        let mut shadow_target_views = (0 .. 2)
        .map(|i| {
            Some(shadow_pass.shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                format: ShadowPass::SHADOW_FORMAT,
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
            (Light {
                pos: math::vec3(7.0, -5.0, 10.0),
                color: wgpu::Color {
                    r: 0.5,
                    g: 1.0,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(60.0),
                depth: 1.0 .. 20.0,
                target_view: shadow_target_views[0].take().unwrap(),
            },),
            (Light {
                pos: math::vec3(-5.0, 7.0, 10.0),
                color: wgpu::Color {
                    r: 1.0,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(45.0),
                depth: 1.0 .. 20.0,
                target_view: shadow_target_views[1].take().unwrap(),
            },),
        ];

        let light_count = lights.len();
        world.insert((), lights);
        
        let matrix = camera::get_projection_view_matrix(&camera_position, camera_fov, sc_desc.width as f32 / sc_desc.height as f32, 1.0, 20.0);
        let forward_uniforms = ForwardUniforms {
            proj: *matrix.as_ref(),
            num_lights: [light_count as u32, 0, 0, 0],
        };

        let forward_pass = ForwardPass::new(device, forward_uniforms, light_uniform_buffer.clone(), &shadow_pass, vb_desc, &local_bind_group_layout, sc_desc);

        let this = Application {
            universe,
            world,
            scheduler: SystemScheduler::new(),
            shadow_pass,
            forward_pass,
            camera_position,
            camera_fov
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
            let mx_total = camera::get_projection_view_matrix(&self.camera_position, self.camera_fov, sc_desc.width as f32 / sc_desc.height as f32, 1.0, 20.0);
            let mx_ref: [[f32; 4]; 4] = mx_total.into();
            let temp_buf =
                device.create_buffer_with_data(mx_ref.as_bytes(), wgpu::BufferUsage::COPY_SRC);

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
            encoder.copy_buffer_to_buffer(&temp_buf, 0, &self.forward_pass.forward_uniform_buffer, 0, 64);
            encoder.finish()
        };

        self.forward_pass.update_swap_chain_descriptor(device, sc_desc);

        Some(command_buf)
    }

    fn update(&mut self, _: WindowEvent)
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
            let mut entities = <(Read<CubeEnt>, Read<LocalToWorld>)>::query();
            let entities_count = entities.iter(&mut self.world).count();
            let size = mem::size_of::<EntityUniforms>();
            let temp_buf_data = device
                .create_buffer_mapped(entities_count * size, wgpu::BufferUsage::COPY_SRC);

            for ((entity, transform), slot) in entities.iter(&mut self.world)
                .zip(temp_buf_data.data.chunks_exact_mut(size))
            {
                slot.copy_from_slice(
                    EntityUniforms {
                        model: transform.0.into(),
                        color: [
                            entity.color.x as f32,
                            entity.color.y as f32,
                            entity.color.z as f32,
                            entity.color.w as f32,
                        ],
                    }
                    .as_bytes(),
                );
            }

            let temp_buf = temp_buf_data.finish();

            for (i, (entity, _)) in entities.iter(&mut self.world).enumerate() {
                encoder.copy_buffer_to_buffer(
                    &temp_buf,
                    (i * size) as wgpu::BufferAddress,
                    entity.uniform_buf.as_ref().unwrap(),
                    0,
                    size as wgpu::BufferAddress,
                );
            }
        }

        self.shadow_pass.render(device, frame, &mut encoder, &mut self.world);
        self.forward_pass.render(device, frame, &mut encoder, &mut self.world);

        encoder.finish()
    }

    #[allow(dead_code)]
    pub fn run(universe: Universe, world: World) {
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
        let (mut example, init_command_buf) = Application::init(universe, world, &sc_desc, &device);
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
                    example.scheduler.execute(&mut example.world);
                    let command_buf = example.render(&frame, &device);
                    queue.submit(&[command_buf]);
                }
                _ => (),
            }
        }); 
    }
}