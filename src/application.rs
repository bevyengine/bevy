use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use zerocopy::AsBytes;
use legion::prelude::*;

use std::sync::Arc;
use std::mem;

use wgpu::{Surface, Device, Queue, SwapChain, SwapChainDescriptor};

use crate::{vertex::*, render::*, math, LocalToWorld, ApplicationStage};

pub struct Application
{
    pub universe: Universe,
    pub world: World,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub window: Window,
    pub swap_chain: SwapChain,
    pub swap_chain_descriptor: SwapChainDescriptor,
    pub scheduler: SystemScheduler<ApplicationStage>,
    pub render_passes: Vec<Box<dyn Pass>>,
}

impl Application {
    pub const MAX_LIGHTS: usize = 10;

    fn add_default_passes(&mut self) {
        let light_uniform_size =
        (Self::MAX_LIGHTS * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;

        let local_bind_group_layout =
            self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            });

        let light_uniform_buffer = Arc::new(UniformBuffer {
            buffer: self.device.create_buffer(&wgpu::BufferDescriptor {
                size: light_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::COPY_SRC
                    | wgpu::BufferUsage::COPY_DST,
            }),
            size: light_uniform_size,
        });

        let mut materials = <Write<Material>>::query();
        for mut material in materials.iter(&mut self.world) {
            let entity_uniform_size = mem::size_of::<MaterialUniforms>() as wgpu::BufferAddress;
            let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                size: entity_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &local_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &uniform_buf,
                        range: 0 .. entity_uniform_size,
                    },
                }],
            });

            material.bind_group = Some(bind_group);
            material.uniform_buf = Some(uniform_buf);
        }

        let light_count = <Read<Light>>::query().iter(&mut self.world).count();
        let forward_uniforms = ForwardUniforms {
            proj: math::Mat4::identity().into(),
            num_lights: [light_count as u32, 0, 0, 0],
        };

        let vertex_size = mem::size_of::<Vertex>();

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

        let shadow_pass = ShadowPass::new(&mut self.device, &mut self.world, light_uniform_buffer.clone(), vb_desc.clone(), &local_bind_group_layout, Self::MAX_LIGHTS as u32);
        let forward_pass = ForwardPass::new(&mut self.device, forward_uniforms, light_uniform_buffer.clone(), &shadow_pass, vb_desc, &local_bind_group_layout, &self.swap_chain_descriptor);
        self.render_passes.push(Box::new(shadow_pass));
        self.render_passes.push(Box::new(forward_pass));
    }

    fn resize(&mut self, width: u32, height: u32)
    {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        for (mut camera, local_to_world) in <(Write<Camera>, Read<LocalToWorld>)>::query().iter(&mut self.world) {
            camera.update(self.swap_chain_descriptor.width, self.swap_chain_descriptor.height);
            let camera_matrix: [[f32; 4]; 4] = (camera.view_matrix * local_to_world.0).into();
            let temp_buf =
                self.device.create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::COPY_SRC);
            for pass in self.render_passes.iter() {
                if let Some(buffer) = pass.get_camera_uniform_buffer() {
                    encoder.copy_buffer_to_buffer(&temp_buf, 0, buffer, 0, 64);
                }
            }
        }

        let command_buffer = encoder.finish();

        for pass in self.render_passes.iter_mut() {
            pass.resize(&mut self.device, &mut self.swap_chain_descriptor);
        }
        self.queue.submit(&[command_buffer]);
    }

    fn update(&mut self, _: WindowEvent)
    {
    }

    fn render(&mut self)
    {
        let mut frame = self.swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let mut entities = <(Read<Material>, Read<LocalToWorld>)>::query();
        let entities_count = entities.iter(&mut self.world).count();
        let size = mem::size_of::<MaterialUniforms>();
        let temp_buf_data = self.device
            .create_buffer_mapped(entities_count * size, wgpu::BufferUsage::COPY_SRC);

        for ((entity, transform), slot) in entities.iter(&mut self.world)
            .zip(temp_buf_data.data.chunks_exact_mut(size))
        {
            slot.copy_from_slice(
                MaterialUniforms {
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

        for pass in self.render_passes.iter_mut() {
            pass.render(&mut self.device, &mut frame, &mut encoder, &mut self.world);
        }

        let command_buffer = encoder.finish();
        self.queue.submit(&[command_buffer]);
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
    
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let (window, hidpi_factor, size, surface) = {
            let window = winit::window::Window::new(&event_loop).unwrap();
            window.set_title("bevy");
            let hidpi_factor = window.hidpi_factor();
            let size = window.inner_size().to_physical(hidpi_factor);
            let surface = wgpu::Surface::create(&window);
            (window, hidpi_factor, size, surface)
        };
    
        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);
    
        log::info!("Initializing the example...");
        let mut app = Application {
            universe,
            world,
            device,
            surface,
            window,
            queue,
            swap_chain,
            swap_chain_descriptor,
            scheduler: SystemScheduler::new(),
            render_passes: Vec::new(),
        };

        app.add_default_passes();
    
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
                    let width = physical.width.round() as u32;
                    let height = physical.height.round() as u32;
                    app.resize(width, height);
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
                        app.update(event);
                    }
                },
                event::Event::EventsCleared => {
                    app.scheduler.execute(&mut app.world);
                    app.render();
                }
                _ => (),
            }
        }); 
    }
}