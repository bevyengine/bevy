mod converters;
mod winit_windows;
pub use winit_windows::*;

use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion},
};

use bevy_app::{App, AppBuilder, AppExit, AppPlugin, EventReader, Events};
use bevy_ecs::Resources;
use bevy_math::Vec2;
use bevy_window::{
    CreateWindow, CursorMoved, Window, WindowCloseRequested, WindowCreated, WindowResized, Windows,
};
use winit::{
    event,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

#[derive(Default)]
pub struct WinitPlugin;

impl AppPlugin for WinitPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            // TODO: It would be great to provide a raw winit WindowEvent here, but the lifetime on it is
            // stopping us. there are plans to remove the lifetime: https://github.com/rust-windowing/winit/pull/1456
            // .add_event::<winit::event::WindowEvent>()
            .init_resource::<WinitWindows>()
            .set_runner(winit_runner);
    }
}

pub fn winit_runner(mut app: App) {
    let event_loop = EventLoop::new();
    let mut create_window_event_reader = EventReader::<CreateWindow>::default();
    let mut app_exit_event_reader = EventReader::<AppExit>::default();

    handle_create_window_events(
        &mut app.resources,
        &event_loop,
        &mut create_window_event_reader,
    );

    log::debug!("Entering winit event loop");
    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };

        if let Ok(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
            if app_exit_event_reader.latest(&app_exit_events).is_some() {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id: winit_window_id,
                ..
            } => {
                let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                let mut windows = app.resources.get_mut::<Windows>().unwrap();
                let window_id = winit_windows.get_window_id(winit_window_id).unwrap();
                let mut window = windows.get_mut(window_id).unwrap();
                window.width = size.width;
                window.height = size.height;

                let mut resize_events = app.resources.get_mut::<Events<WindowResized>>().unwrap();
                resize_events.send(WindowResized {
                    id: window_id,
                    height: window.height as usize,
                    width: window.width as usize,
                    is_primary: windows.is_primary(window_id),
                });
            }
            event::Event::WindowEvent {
                event,
                window_id: winit_window_id,
                ..
            } => match event {
                WindowEvent::CloseRequested => {
                    let mut window_close_requested_events = app
                        .resources
                        .get_mut::<Events<WindowCloseRequested>>()
                        .unwrap();
                    let windows = app.resources.get_mut::<Windows>().unwrap();
                    let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                    let window_id = winit_windows.get_window_id(winit_window_id).unwrap();
                    window_close_requested_events.send(WindowCloseRequested {
                        id: window_id,
                        is_primary: windows.is_primary(window_id),
                    });
                }
                WindowEvent::KeyboardInput { ref input, .. } => {
                    let mut keyboard_input_events =
                        app.resources.get_mut::<Events<KeyboardInput>>().unwrap();
                    keyboard_input_events.send(converters::convert_keyboard_input(input));
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let mut cursor_moved_events =
                        app.resources.get_mut::<Events<CursorMoved>>().unwrap();
                    let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                    let window_id = winit_windows.get_window_id(winit_window_id).unwrap();
                    cursor_moved_events.send(CursorMoved {
                        id: window_id,
                        position: Vec2::new(position.x as f32, position.y as f32),
                    });
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let mut mouse_button_input_events =
                        app.resources.get_mut::<Events<MouseButtonInput>>().unwrap();
                    mouse_button_input_events.send(MouseButtonInput {
                        button: converters::convert_mouse_button(button.into()),
                        state: converters::convert_element_state(state),
                    });
                }
                _ => {}
            },
            event::Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    let mut mouse_motion_events =
                        app.resources.get_mut::<Events<MouseMotion>>().unwrap();
                    mouse_motion_events.send(MouseMotion {
                        delta: Vec2::new(delta.0 as f32, delta.1 as f32),
                    });
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
    for create_window_event in create_window_event_reader.iter(&create_window_events) {
        let window = Window::new(create_window_event.id, &create_window_event.descriptor);
        winit_windows.create_window(event_loop, &window);
        let window_id = window.id;
        windows.add(window);
        window_created_events.send(WindowCreated {
            id: window_id,
            is_primary: windows.is_primary(window_id),
        });
    }
}
