mod winit_windows;
pub use winit_windows::*;

use crate::prelude::*;

use super::{CreateWindow, Window, WindowCreated, WindowResized, Windows};
use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

#[derive(Default)]
pub struct WinitPlugin;

impl AppPlugin for WinitPlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        app.add_resource(WinitWindows::default())
            .set_runner(winit_runner)
    }

    fn name(&self) -> &'static str {
        "Winit"
    }
}

pub fn winit_runner(mut app: App) {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut create_window_event_reader = app.resources.get_event_reader::<CreateWindow>();

    handle_create_window_events(
        &mut app.resources,
        &event_loop,
        &mut create_window_event_reader,
    );

    log::debug!("Entering render loop");
    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id: winit_window_id,
                ..
            } => {
                let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                let mut windows = app.resources.get_mut::<Windows>().unwrap();
                let window_id = winit_windows.get_window_id(winit_window_id).unwrap();
                let is_primary = windows
                    .get_primary()
                    .map(|primary_window| primary_window.id == window_id)
                    .unwrap_or(false);
                let mut window = windows.get_mut(window_id).unwrap();
                window.width = size.width;
                window.height = size.height;

                let mut resize_event = app.resources.get_mut::<Events<WindowResized>>().unwrap();
                resize_event.send(WindowResized {
                    id: window_id,
                    height: window.height,
                    width: window.width,
                    is_primary,
                });
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
                handle_create_window_events(
                    &mut app.resources,
                    event_loop,
                    &mut create_window_event_reader,
                );
                app.update();
            }
            _ => (),
        }
    });
}

fn handle_create_window_events(
    resources: &mut Resources,
    event_loop: &EventLoopWindowTarget<()>,
    create_window_event_reader: &mut EventReader<CreateWindow>,
) {
    let mut winit_windows = resources.get_mut::<WinitWindows>().unwrap();
    let mut windows = resources.get_mut::<Windows>().unwrap();
    let create_window_events = resources.get::<Events<CreateWindow>>().unwrap();
    let mut window_created_events = resources.get_mut::<Events<WindowCreated>>().unwrap();
    for create_window_event in create_window_events.iter(create_window_event_reader) {
        let window = Window::new(&create_window_event.descriptor);
        winit_windows.create_window(event_loop, &window);
        let window_id = window.id;
        windows.add(window);
        let is_primary = windows.get_primary().map(|primary| primary.id == window_id).unwrap_or(false);
        window_created_events.send(WindowCreated { id: window_id, is_primary });
    }
}
