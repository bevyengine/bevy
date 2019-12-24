use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use zerocopy::AsBytes;
use legion::prelude::*;

use std::mem;

use wgpu::{Surface, Device, Queue, SwapChain, SwapChainDescriptor};

use crate::{vertex::*, render::*, LocalToWorld, ApplicationStage, Time};

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
    pub render_resources: RenderResources,
    pub render_passes: Vec<Box<dyn Pass>>,
}

impl Application {
    fn add_default_passes(&mut self) {
        let vertex_size = mem::size_of::<Vertex>();
        let vertex_buffer_descriptor = wgpu::VertexBufferDescriptor {
            stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        };

        // let shadow_pass = ShadowPass::new(&mut self.device, &mut self.world, &self.render_resources, vertex_buffer_descriptor.clone());
        // let forward_shadow_pass = ForwardShadowPass::new(&mut self.device, &self.world, &self.render_resources, &shadow_pass, vertex_buffer_descriptor.clone(), &self.swap_chain_descriptor);
        let forward_pass = ForwardPass::new(&mut self.device, &self.world, &self.render_resources, vertex_buffer_descriptor, &self.swap_chain_descriptor);
        // self.render_passes.push(Box::new(shadow_pass));
        // self.render_passes.push(Box::new(forward_shadow_pass));
        self.render_passes.push(Box::new(forward_pass));
    }

    fn update(&mut self) {
        {
            let mut time = self.world.resources.get_mut::<Time>().unwrap();
            time.start();
        }
        self.scheduler.execute(&mut self.world);
        self.render();
        {
            let mut time = self.world.resources.get_mut::<Time>().unwrap();
            time.stop();
        }
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
            let camera_matrix: [[f32; 4]; 4] = (camera.view_matrix * local_to_world.0).to_cols_array_2d();
            let matrix_size = mem::size_of::<[[f32; 4]; 4]>() as u64;
            let temp_camera_buffer =
                self.device.create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::COPY_SRC);
            for pass in self.render_passes.iter() {
                if let Some(buffer) = pass.get_camera_uniform_buffer() {
                    encoder.copy_buffer_to_buffer(&temp_camera_buffer, 0, buffer, 0, matrix_size);
                }
            }
        }

        let command_buffer = encoder.finish();

        for pass in self.render_passes.iter_mut() {
            pass.resize(&mut self.device, &mut self.swap_chain_descriptor);
        }
        self.queue.submit(&[command_buffer]);
    }

    fn handle_event(&mut self, _: WindowEvent)
    {
    }

    fn render(&mut self)
    {
        let mut frame = self.swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let mut entities = <(Write<Material>, Read<LocalToWorld>)>::query();
        let entities_count = entities.iter(&mut self.world).count();
        let size = mem::size_of::<MaterialUniforms>();
        let temp_buf_data = self.device
            .create_buffer_mapped(entities_count * size, wgpu::BufferUsage::COPY_SRC);

        for ((material, transform), slot) in entities.iter(&mut self.world)
            .zip(temp_buf_data.data.chunks_exact_mut(size))
        {
            slot.copy_from_slice(
                MaterialUniforms {
                    model: transform.0.to_cols_array_2d(),
                    color: material.color.into(),
                }
                .as_bytes(),
            );
        }

        self.render_resources.update_lights(&self.device, &mut encoder, &mut self.world);

        for mut material in <Write<Material>>::query().iter(&mut self.world) {
            if let None = material.bind_group {
                let material_uniform_size = mem::size_of::<MaterialUniforms>() as wgpu::BufferAddress;
                let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                    size: material_uniform_size,
                    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                });

                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.render_resources.local_bind_group_layout,
                    bindings: &[wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0 .. material_uniform_size,
                        },
                    }],
                });

                material.bind_group = Some(bind_group);
                material.uniform_buf = Some(uniform_buf);
            }
        }

        let temp_buf = temp_buf_data.finish();
        
        for pass in self.render_passes.iter_mut() {
            pass.render(&mut self.device, &mut frame, &mut encoder, &mut self.world, &self.render_resources);
        }

        // TODO: this should happen before rendering
        for (i, (material, _)) in entities.iter(&mut self.world).enumerate() {
            encoder.copy_buffer_to_buffer(
                &temp_buf,
                (i * size) as wgpu::BufferAddress,
                material.uniform_buf.as_ref().unwrap(),
                0,
                size as wgpu::BufferAddress,
            );
        }

        let command_buffer = encoder.finish();
        self.queue.submit(&[command_buffer]);
    }

    #[allow(dead_code)]
    pub fn run(universe: Universe, mut world: World, system_scheduler: SystemScheduler<ApplicationStage>) {
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
    
        let (mut device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let (window, hidpi_factor, size, surface) = {
            let window = winit::window::Window::new(&event_loop).unwrap();
            window.set_title("bevy");
            window.set_inner_size((1280, 720).into());
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
        
        world.resources.insert(Time::new());

        let render_resources = RenderResources::new(&mut device, 10);

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
            render_resources,
            scheduler: system_scheduler,
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
                        app.handle_event(event);
                    }
                },
                event::Event::EventsCleared => {
                    app.update();
                }
                _ => (),
            }
        }); 
    }
}