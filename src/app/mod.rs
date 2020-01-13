mod app_builder;

pub use app_builder::AppBuilder;

use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use legion::prelude::*;

use crate::{render::*, Time};

pub struct App {
    pub world: World,
    pub render_graph: RenderGraph,
    pub swap_chain: Option<wgpu::SwapChain>,
    pub schedule: Schedule,
}

impl App {
    pub fn new(world: World, schedule: Schedule, render_graph: RenderGraph) -> App {
        App {
            world,
            schedule: schedule,
            render_graph,
            swap_chain: None,
        }
    }

    fn update(&mut self) {
        {
            let mut time = self.world.resources.get_mut::<Time>().unwrap();
            time.start();
        }
        self.schedule.execute(&mut self.world);
        self.render();
        {
            let mut time = self.world.resources.get_mut::<Time>().unwrap();
            time.stop();
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain = Some(self.render_graph.resize(width, height, &mut self.world));
    }

    fn handle_event(&mut self, _: WindowEvent) {}

    fn render(&mut self) {
        self.render_graph
            .render(&mut self.world, self.swap_chain.as_mut().unwrap());
    }

    pub fn run(mut self) {
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

        let (window, size, surface) = {
            let window = winit::window::Window::new(&event_loop).unwrap();
            window.set_title("bevy");
            window.set_inner_size(winit::dpi::LogicalSize::new(1280, 720));
            let size = window.inner_size();
            let surface = wgpu::Surface::create(&window);
            (window, size, surface)
        };

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        log::info!("Initializing the example...");
        self.render_graph.initialize(
            &mut self.world,
            device,
            swap_chain_descriptor,
            queue,
            surface,
        );

        self.world.resources.insert(window);
        self.swap_chain = Some(swap_chain);

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
                    self.resize(size.width, size.height);
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
                        self.handle_event(event);
                    }
                },
                event::Event::MainEventsCleared => {
                    self.update();
                }
                _ => (),
            }
        });
    }
}
