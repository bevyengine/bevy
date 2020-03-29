use crate::{
    app::{App, AppBuilder},
    plugin::AppPlugin,
};

use super::Window;
use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

#[derive(Default)]
pub struct WinitPlugin;

impl AppPlugin for WinitPlugin {
    fn build(&self, mut app: AppBuilder) -> AppBuilder {
        {
            app.run = Some(get_winit_run());
        }

        app
    }

    fn name(&self) -> &'static str {
        "Winit"
    }
}

pub fn get_winit_run() -> Box<dyn Fn(App) + Send + Sync> {
    Box::new(|mut app: App| {
        env_logger::init();
        let event_loop = EventLoop::new();
        let winit_window = {
            let window = app.resources.get::<Window>().unwrap();
            let winit_window = winit::window::Window::new(&event_loop).unwrap();
            winit_window.set_title(&window.title);
            winit_window.set_inner_size(winit::dpi::PhysicalSize::new(window.width, window.height));
            winit_window
        };

        app.resources.insert(winit_window);

        log::debug!("Entering render loop");
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
                    if let Some(ref mut renderer) = app.renderer {
                        {
                            let mut window = app.resources.get_mut::<Window>().unwrap();
                            window.width = size.width;
                            window.height = size.height;
                        }

                        renderer.resize(&mut app.world, &mut app.resources);
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
                    _ => {}
                },
                event::Event::MainEventsCleared => {
                    app.update();
                }
                _ => (),
            }
        });
    })
}
