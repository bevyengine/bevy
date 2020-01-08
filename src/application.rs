use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use legion::prelude::*;

use crate::{render::*, render::passes::*, ApplicationStage, Time};

pub struct Application
{
    pub universe: Universe,
    pub world: World,
    pub window: Window,
    pub render_graph: RenderGraph,
    pub scheduler: SystemScheduler<ApplicationStage>,
}

impl Application {
    fn add_default_passes(&mut self) {
        let local_bind_group_layout =
        self.render_graph.data.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });

        self.render_graph.add_render_resource_manager(Box::new(render_resources::MaterialResourceManager));
        self.render_graph.add_render_resource_manager(Box::new(render_resources::LightResourceManager::new(10)));
        self.render_graph.add_render_resource_manager(Box::new(render_resources::CameraResourceManager));

        self.render_graph.data.set_bind_group_layout("local", local_bind_group_layout);

        let depth_format = wgpu::TextureFormat::Depth32Float;
        self.render_graph.set_pass("forward", Box::new(ForwardPass::new(depth_format)));
        self.render_graph.set_pipeline("forward", "forward", Box::new(ForwardPipeline::new()));
        self.render_graph.set_pipeline("forward", "forward_instanced", Box::new(ForwardInstancedPipeline::new(depth_format)));
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
        self.render_graph.resize(width, height, &mut self.world);
    }

    fn handle_event(&mut self, _: WindowEvent)
    {
    }

    fn render(&mut self)
    {
        self.render_graph.render(&mut self.world);
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
    
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
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

        log::info!("Initializing the example...");
        let render_graph = RenderGraph::new(device, swap_chain_descriptor, swap_chain, queue, surface);
        let mut app = Application {
            universe,
            world,
            window,
            render_graph,
            scheduler: system_scheduler,
        };

        app.add_default_passes();
        app.render_graph.initialize(&mut app.world);
    
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