use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use legion::prelude::*;

use crate::{app::AppBuilder, core::Time, render::renderer::Renderer};

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

        let window = winit::window::Window::new(&event_loop).unwrap();
        window.set_title("bevy");
        window.set_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        self.resources.insert(window);

        log::info!("Entering render loop...");
        event_loop.run(move |event, _, control_flow| {
            *control_flow = if cfg!(feature = "metal-auto-capture") {
                ControlFlow::Exit
            } else {
                ControlFlow::Poll
            };
            match event {
                event::Event::WindowEvent {
                    event: WindowEvent::Resized(_size),
                    ..
                } => {
                    if let Some(ref mut renderer) = self.renderer {
                        renderer.resize(&mut self.world, &mut self.resources);
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
