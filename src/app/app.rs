use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use legion::prelude::*;

use crate::{app::AppBuilder, core::{Window, Time}, render::renderer::Renderer};

pub struct App {
    pub universe: Universe,
    pub world: World,
    pub resources: Resources,
    pub renderer: Option<Box<dyn Renderer>>,
    pub schedule: Schedule,
}

impl App {
    pub fn new(
        universe: Universe,
        world: World,
        schedule: Schedule,
        resources: Resources,
        renderer: Option<Box<dyn Renderer>>,
    ) -> App {
        App {
            universe,
            world,
            schedule,
            renderer,
            resources,
        }
    }

    pub fn build() -> AppBuilder {
        AppBuilder::new()
    }

    fn update(&mut self) {
        if let Some(mut time) = self.resources.get_mut::<Time>() {
            time.start();
        }
        self.schedule.execute(&mut self.world, &mut self.resources);

        if let Some(ref mut renderer) = self.renderer {
            renderer.update(&mut self.world, &mut self.resources);
        }

        if let Some(mut time) = self.resources.get_mut::<Time>() {
            time.stop();
        }
    }

    fn handle_event(&mut self, _: WindowEvent) {}

    pub fn run(mut self) {
        env_logger::init();
        let event_loop = EventLoop::new();
        log::info!("Initializing the window...");

        let window = Window {
            width: 1280,
            height: 720,
        };

        let winit_window = winit::window::Window::new(&event_loop).unwrap();
        winit_window.set_title("bevy");
        winit_window.set_inner_size(winit::dpi::PhysicalSize::new(window.width, window.height));

        self.resources.insert(window);
        self.resources.insert(winit_window);

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
                    if let Some(ref mut renderer) = self.renderer {
                        {
                            let mut window = self.resources.get_mut::<Window>().unwrap();
                            window.width = size.width;
                            window.height = size.height;
                        }

                        renderer.resize(&mut self.world, &mut self.resources, size.width, size.height);
                    } else {
                        println!("no renderer {} {}", size.width, size.height);
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
