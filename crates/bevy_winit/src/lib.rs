mod converters;
mod winit_config;
mod winit_windows;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    touch::TouchInput,
};
pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{prelude::*, AppExit};
use bevy_ecs::{IntoThreadLocalSystem, Resources, World};
use bevy_math::Vec2;
use bevy_window::{
    CreateWindow, CursorMoved, ReceivedCharacter, Window, WindowCloseRequested, WindowCreated,
    WindowResized, Windows,
};
use winit::{
    event::{self, DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

#[derive(Default)]
pub struct WinitPlugin;

#[derive(Debug)]
pub struct EventLoopProxyPtr(pub usize);

impl Plugin for WinitPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            // TODO: It would be great to provide a raw winit WindowEvent here, but the lifetime on it is
            // stopping us. there are plans to remove the lifetime: https://github.com/rust-windowing/winit/pull/1456
            // .add_event::<winit::event::WindowEvent>()
            .init_resource::<WinitWindows>()
            .set_runner(winit_runner)
            .add_system(change_window.thread_local_system());
    }
}

fn change_window(_: &mut World, resources: &mut Resources) {
    let winit_windows = resources.get::<WinitWindows>().unwrap();
    let mut windows = resources.get_mut::<Windows>().unwrap();

    for bevy_window in windows.iter_mut() {
        let id = bevy_window.id();
        for command in bevy_window.drain_commands() {
            match command {
                bevy_window::WindowCommand::SetWindowMode {
                    mode,
                    resolution: (width, height),
                } => {
                    let window = winit_windows.get_window(id).unwrap();
                    match mode {
                        bevy_window::WindowMode::BorderlessFullscreen => {
                            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
                        }
                        bevy_window::WindowMode::Fullscreen { use_size } => window.set_fullscreen(
                            Some(winit::window::Fullscreen::Exclusive(match use_size {
                                true => get_fitting_videomode(
                                    &window.current_monitor().unwrap(),
                                    width,
                                    height,
                                ),
                                false => get_best_videomode(&window.current_monitor().unwrap()),
                            })),
                        ),
                        bevy_window::WindowMode::Windowed => window.set_fullscreen(None),
                    }
                }
                bevy_window::WindowCommand::SetTitle { title } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_title(&title);
                }
                bevy_window::WindowCommand::SetResolution { width, height } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_inner_size(winit::dpi::PhysicalSize::new(width, height));
                }
                bevy_window::WindowCommand::SetVsync { .. } => (),
                bevy_window::WindowCommand::SetResizable { resizable } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_resizable(resizable);
                }
                bevy_window::WindowCommand::SetDecorations { decorations } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_decorations(decorations);
                }
                bevy_window::WindowCommand::SetCursorLockMode { locked } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_cursor_grab(locked).unwrap();
                }
                bevy_window::WindowCommand::SetCursorVisibility { visible } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_cursor_visible(visible);
                }
            }
        }
    }
}

fn run<F>(event_loop: EventLoop<()>, event_handler: F) -> !
where
    F: 'static + FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
{
    event_loop.run(event_handler)
}

// TODO: It may be worth moving this cfg into a procedural macro so that it can be referenced by
// a single name instead of being copied around.
// https://gist.github.com/jakerr/231dee4a138f7a5f25148ea8f39b382e seems to work.
#[cfg(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn run_return<F>(event_loop: &mut EventLoop<()>, event_handler: F)
where
    F: FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
{
    use winit::platform::desktop::EventLoopExtDesktop;
    event_loop.run_return(event_handler)
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn run_return<F>(_event_loop: &mut EventLoop<()>, _event_handler: F)
where
    F: FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
{
    panic!("Run return is not supported on this platform!")
}

pub fn winit_runner(mut app: App) {
    let mut event_loop = EventLoop::new();
    let mut create_window_event_reader = EventReader::<CreateWindow>::default();
    let mut app_exit_event_reader = EventReader::<AppExit>::default();

    app.resources
        .insert_thread_local(EventLoopProxyPtr(
            Box::into_raw(Box::new(event_loop.create_proxy())) as usize,
        ));

    handle_create_window_events(
        &mut app.resources,
        &event_loop,
        &mut create_window_event_reader,
    );

    app.initialize();

    log::debug!("Entering winit event loop");

    let should_return_from_run = app
        .resources
        .get::<WinitConfig>()
        .map_or(false, |config| config.return_from_run);

    let event_handler = move |event: Event<()>,
                              event_loop: &EventLoopWindowTarget<()>,
                              control_flow: &mut ControlFlow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };

        if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
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
                let window = windows.get_mut(window_id).unwrap();
                window.update_resolution_from_backend(size.width, size.height);

                let mut resize_events = app.resources.get_mut::<Events<WindowResized>>().unwrap();
                resize_events.send(WindowResized {
                    id: window_id,
                    height: window.height() as usize,
                    width: window.width() as usize,
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
                    let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                    let window_id = winit_windows.get_window_id(winit_window_id).unwrap();
                    window_close_requested_events.send(WindowCloseRequested { id: window_id });
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
                    let window = winit_windows.get_window(window_id).unwrap();
                    let inner_size = window.inner_size();
                    // move origin to bottom left
                    let y_position = inner_size.height as f32 - position.y as f32;
                    cursor_moved_events.send(CursorMoved {
                        id: window_id,
                        position: Vec2::new(position.x as f32, y_position as f32),
                    });
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let mut mouse_button_input_events =
                        app.resources.get_mut::<Events<MouseButtonInput>>().unwrap();
                    mouse_button_input_events.send(MouseButtonInput {
                        button: converters::convert_mouse_button(button),
                        state: converters::convert_element_state(state),
                    });
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    event::MouseScrollDelta::LineDelta(x, y) => {
                        let mut mouse_wheel_input_events =
                            app.resources.get_mut::<Events<MouseWheel>>().unwrap();
                        mouse_wheel_input_events.send(MouseWheel {
                            unit: MouseScrollUnit::Line,
                            x,
                            y,
                        });
                    }
                    event::MouseScrollDelta::PixelDelta(p) => {
                        let mut mouse_wheel_input_events =
                            app.resources.get_mut::<Events<MouseWheel>>().unwrap();
                        mouse_wheel_input_events.send(MouseWheel {
                            unit: MouseScrollUnit::Pixel,
                            x: p.x as f32,
                            y: p.y as f32,
                        });
                    }
                },
                WindowEvent::Touch(mut touch) => {
                    let mut touch_input_events =
                        app.resources.get_mut::<Events<TouchInput>>().unwrap();
                    let windows = app.resources.get_mut::<Windows>().unwrap();
                    // FIXME?: On Android window start is top while on PC/Linux/OSX on bottom
                    if cfg!(target_os = "android") {
                        let window_height = windows.get_primary().unwrap().height();
                        touch.location.y = window_height as f64 - touch.location.y;
                    }
                    touch_input_events.send(converters::convert_touch_input(touch));
                }
                WindowEvent::ReceivedCharacter(c) => {
                    let mut char_input_events = app
                        .resources
                        .get_mut::<Events<ReceivedCharacter>>()
                        .unwrap();

                    let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                    let window_id = winit_windows.get_window_id(winit_window_id).unwrap();

                    char_input_events.send(ReceivedCharacter {
                        id: window_id,
                        char: c,
                    })
                }
                _ => {}
            },
            event::Event::DeviceEvent { ref event, .. } => {
                if let DeviceEvent::MouseMotion { delta } = event {
                    let mut mouse_motion_events =
                        app.resources.get_mut::<Events<MouseMotion>>().unwrap();
                    mouse_motion_events.send(MouseMotion {
                        delta: Vec2::new(delta.0 as f32, delta.1 as f32),
                    });
                }
            }
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
    };
    if should_return_from_run {
        run_return(&mut event_loop, event_handler);
    } else {
        run(event_loop, event_handler);
    }
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
        let window_id = window.id();
        windows.add(window);
        window_created_events.send(WindowCreated { id: window_id });
    }
}
