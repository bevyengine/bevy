use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use legion::prelude::*;

use crate::{core::Time, render::*, app::AppBuilder};

pub struct App {
    pub universe: Universe,
    pub world: World,
    pub legacy_render_graph: Option<RenderGraph>,
    pub schedule: Schedule,
}

impl App {
    pub fn new(
        universe: Universe,
        world: World,
        schedule: Schedule,
        legacy_render_graph: Option<RenderGraph>,
    ) -> App {
        App {
            universe,
            world,
            schedule: schedule,
            legacy_render_graph: legacy_render_graph,
        }
    }

    pub fn build() -> AppBuilder {
        AppBuilder::new()
    }

    fn update(&mut self) {
        if let Some(mut time) = self.world.resources.get_mut::<Time>() {
            time.start();
        }
        self.schedule.execute(&mut self.world);

        // TODO: remove me
        if let Some(ref mut render_graph) = self.legacy_render_graph {
            render_graph.render(&mut self.world);
        }

        if let Some(mut time) = self.world.resources.get_mut::<Time>() {
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

        self.world.resources.insert(window);

        log::info!("Initializing the example...");
        if let Some(ref mut render_graph) = self.legacy_render_graph {
            render_graph.initialize(&mut self.world);
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
                    if let Some(ref mut render_graph) = self.legacy_render_graph {
                        render_graph
                            .resize(size.width, size.height, &mut self.world);
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
