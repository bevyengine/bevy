mod app_stage;
mod app_builder;

pub use app_stage::AppStage;
pub use app_builder::AppBuilder;

use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use legion::prelude::*;

use crate::{render::*, Time};

pub struct App
{
    pub world: World,
    pub window: Option<Window>,
    pub render_graph: RenderGraph,
    pub swap_chain: Option<wgpu::SwapChain>,
    pub scheduler: SystemScheduler<AppStage>,
}

impl App {
    pub fn new(world: World, scheduler: SystemScheduler<AppStage>) -> App {
        App {
            world: world,
            scheduler: scheduler,
            render_graph: RenderGraph::new(),
            swap_chain: None,
            window: None,
        }
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
        self.swap_chain = Some(self.render_graph.resize(width, height, &mut self.world));
    }

    fn handle_event(&mut self, _: WindowEvent)
    {
    }

    fn render(&mut self)
    {
        self.render_graph.render(&mut self.world, self.swap_chain.as_mut().unwrap());
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
            window.set_inner_size((1280, 720).into());
            let hidpi_factor = window.hidpi_factor();
            let size = window.inner_size().to_physical(hidpi_factor);
            let surface = wgpu::Surface::create(&window);
            (window, size, surface)
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
        self.render_graph.initialize(&mut self.world, device, swap_chain_descriptor, queue, surface);
        self.window = Some(window);
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
                    let hidpi_factor = self.window.as_ref().unwrap().hidpi_factor();
                    let physical = size.to_physical(hidpi_factor);
                    log::info!("Resizing to {:?}", physical);
                    let width = physical.width.round() as u32;
                    let height = physical.height.round() as u32;
                    self.resize(width, height);
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
                event::Event::EventsCleared => {
                    self.update();
                }
                _ => (),
            }
        }); 
    }
}