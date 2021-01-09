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
use bevy_ecs::{IntoSystem, Resources, World};
use bevy_math::Vec2;
use bevy_utils::tracing::{error, trace, warn};
use bevy_window::{
    CreateWindow, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, ReceivedCharacter,
    WindowBackendScaleFactorChanged, WindowCloseRequested, WindowCreated, WindowFocused,
    WindowResized, WindowScaleFactorChanged, Windows,
};
use winit::{
    event::{self, DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::unix::EventLoopExtUnix;

#[derive(Default)]
pub struct WinitPlugin;

impl Plugin for WinitPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<WinitWindows>()
            .set_runner(winit_runner)
            .add_system(change_window.system());
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
                bevy_window::WindowCommand::SetScaleFactor { scale_factor } => {
                    let mut window_dpi_changed_events = resources
                        .get_mut::<Events<WindowScaleFactorChanged>>()
                        .unwrap();
                    window_dpi_changed_events.send(WindowScaleFactorChanged { id, scale_factor });
                }
                bevy_window::WindowCommand::SetResolution {
                    logical_resolution: (width, height),
                    scale_factor,
                } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_inner_size(
                        winit::dpi::LogicalSize::new(width, height)
                            .to_physical::<f64>(scale_factor),
                    );
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
                    window
                        .set_cursor_grab(locked)
                        .unwrap_or_else(|e| error!("Unable to un/grab cursor: {}", e));
                }
                bevy_window::WindowCommand::SetCursorVisibility { visible } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_cursor_visible(visible);
                }
                bevy_window::WindowCommand::SetCursorPosition { position } => {
                    let window = winit_windows.get_window(id).unwrap();
                    let inner_size = window.inner_size().to_logical::<f32>(window.scale_factor());
                    window
                        .set_cursor_position(winit::dpi::LogicalPosition::new(
                            position.x,
                            inner_size.height - position.y,
                        ))
                        .unwrap_or_else(|e| error!("Unable to set cursor position: {}", e));
                }
                bevy_window::WindowCommand::SetMaximized { maximized } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_maximized(maximized)
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
    use winit::platform::run_return::EventLoopExtRunReturn;
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

pub fn winit_runner(app: App) {
    winit_runner_with(app, EventLoop::new());
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub fn winit_runner_any_thread(app: App) {
    winit_runner_with(app, EventLoop::new_any_thread());
}

pub fn winit_runner_with(mut app: App, mut event_loop: EventLoop<()>) {
    let mut create_window_event_reader = EventReader::<CreateWindow>::default();
    let mut app_exit_event_reader = EventReader::<AppExit>::default();

    app.resources.insert_thread_local(event_loop.create_proxy());

    trace!("Entering winit event loop");

    let should_return_from_run = app
        .resources
        .get::<WinitConfig>()
        .map_or(false, |config| config.return_from_run);

    let event_handler = move |event: Event<()>,
                              event_loop: &EventLoopWindowTarget<()>,
                              control_flow: &mut ControlFlow| {
        *control_flow = ControlFlow::Poll;

        if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
            if app_exit_event_reader.latest(&app_exit_events).is_some() {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            event::Event::WindowEvent {
                event,
                window_id: winit_window_id,
                ..
            } => {
                let winit_windows = app.resources.get_mut::<WinitWindows>().unwrap();
                let mut windows = app.resources.get_mut::<Windows>().unwrap();
                let window_id =
                    if let Some(window_id) = winit_windows.get_window_id(winit_window_id) {
                        window_id
                    } else {
                        warn!(
                            "Skipped event for unknown winit Window Id {:?}",
                            winit_window_id
                        );
                        return;
                    };

                let window = if let Some(window) = windows.get_mut(window_id) {
                    window
                } else {
                    warn!("Skipped event for unknown Window Id {:?}", winit_window_id);
                    return;
                };

                match event {
                    WindowEvent::Resized(size) => {
                        window.update_actual_size_from_backend(size.width, size.height);
                        let mut resize_events =
                            app.resources.get_mut::<Events<WindowResized>>().unwrap();
                        resize_events.send(WindowResized {
                            id: window_id,
                            width: window.width(),
                            height: window.height(),
                        });
                    }
                    WindowEvent::CloseRequested => {
                        let mut window_close_requested_events = app
                            .resources
                            .get_mut::<Events<WindowCloseRequested>>()
                            .unwrap();
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
                        let winit_window = winit_windows.get_window(window_id).unwrap();
                        let position = position.to_logical(winit_window.scale_factor());
                        let inner_size = winit_window
                            .inner_size()
                            .to_logical::<f32>(winit_window.scale_factor());

                        // move origin to bottom left
                        let y_position = inner_size.height - position.y;

                        let position = Vec2::new(position.x, y_position);
                        window.update_cursor_position_from_backend(Some(position));

                        cursor_moved_events.send(CursorMoved {
                            id: window_id,
                            position,
                        });
                    }
                    WindowEvent::CursorEntered { .. } => {
                        let mut cursor_entered_events =
                            app.resources.get_mut::<Events<CursorEntered>>().unwrap();
                        cursor_entered_events.send(CursorEntered { id: window_id });
                    }
                    WindowEvent::CursorLeft { .. } => {
                        let mut cursor_left_events =
                            app.resources.get_mut::<Events<CursorLeft>>().unwrap();
                        window.update_cursor_position_from_backend(None);
                        cursor_left_events.send(CursorLeft { id: window_id });
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
                    WindowEvent::Touch(touch) => {
                        let mut touch_input_events =
                            app.resources.get_mut::<Events<TouchInput>>().unwrap();

                        let winit_window = winit_windows.get_window(window_id).unwrap();
                        let mut location = touch.location.to_logical(winit_window.scale_factor());

                        // On a mobile window, the start is from the top while on PC/Linux/OSX from bottom
                        if cfg!(target_os = "android") || cfg!(target_os = "ios") {
                            let window_height = windows.get_primary().unwrap().height();
                            location.y = window_height - location.y;
                        }
                        touch_input_events.send(converters::convert_touch_input(touch, location));
                    }
                    WindowEvent::ReceivedCharacter(c) => {
                        let mut char_input_events = app
                            .resources
                            .get_mut::<Events<ReceivedCharacter>>()
                            .unwrap();

                        char_input_events.send(ReceivedCharacter {
                            id: window_id,
                            char: c,
                        })
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {
                        let mut backend_scale_factor_change_events = app
                            .resources
                            .get_mut::<Events<WindowBackendScaleFactorChanged>>()
                            .unwrap();
                        backend_scale_factor_change_events.send(WindowBackendScaleFactorChanged {
                            id: window_id,
                            scale_factor,
                        });
                        #[allow(clippy::float_cmp)]
                        if window.scale_factor() != scale_factor {
                            let mut scale_factor_change_events = app
                                .resources
                                .get_mut::<Events<WindowScaleFactorChanged>>()
                                .unwrap();

                            scale_factor_change_events.send(WindowScaleFactorChanged {
                                id: window_id,
                                scale_factor,
                            });
                        }

                        window.update_scale_factor_from_backend(scale_factor);

                        if window.physical_width() != new_inner_size.width
                            || window.physical_height() != new_inner_size.height
                        {
                            let mut resize_events =
                                app.resources.get_mut::<Events<WindowResized>>().unwrap();
                            resize_events.send(WindowResized {
                                id: window_id,
                                width: window.width(),
                                height: window.height(),
                            });
                        }
                        window.update_actual_size_from_backend(
                            new_inner_size.width,
                            new_inner_size.height,
                        );
                    }
                    WindowEvent::Focused(focused) => {
                        let mut focused_events =
                            app.resources.get_mut::<Events<WindowFocused>>().unwrap();
                        focused_events.send(WindowFocused {
                            id: window_id,
                            focused,
                        });
                    }
                    WindowEvent::DroppedFile(path_buf) => {
                        let mut events =
                            app.resources.get_mut::<Events<FileDragAndDrop>>().unwrap();
                        events.send(FileDragAndDrop::DroppedFile {
                            id: window_id,
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFile(path_buf) => {
                        let mut events =
                            app.resources.get_mut::<Events<FileDragAndDrop>>().unwrap();
                        events.send(FileDragAndDrop::HoveredFile {
                            id: window_id,
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFileCancelled => {
                        let mut events =
                            app.resources.get_mut::<Events<FileDragAndDrop>>().unwrap();
                        events.send(FileDragAndDrop::HoveredFileCancelled { id: window_id });
                    }
                    _ => {}
                }
            }
            event::Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                let mut mouse_motion_events =
                    app.resources.get_mut::<Events<MouseMotion>>().unwrap();
                mouse_motion_events.send(MouseMotion {
                    delta: Vec2::new(delta.0 as f32, delta.1 as f32),
                });
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
        let window = winit_windows.create_window(
            event_loop,
            create_window_event.id,
            &create_window_event.descriptor,
        );
        windows.add(window);
        window_created_events.send(WindowCreated {
            id: create_window_event.id,
        });
    }
}
