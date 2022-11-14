mod converters;
#[cfg(target_arch = "wasm32")]
mod web_resize;
mod winit_config;
mod winit_windows;

use converters::convert_cursor_grab_mode;
pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{App, AppExit, CoreStage, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::{
    event::{Events, ManualEventReader},
    world::World,
};
use bevy_input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy_math::{ivec2, DVec2, UVec2, Vec2};
use bevy_utils::{
    tracing::{error, info, trace, warn},
    Instant,
};
use bevy_window::{
    CreateWindow, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, ModifiesWindows,
    ReceivedCharacter, RequestRedraw, WindowBackendScaleFactorChanged, WindowCloseRequested,
    WindowClosed, WindowCreated, WindowFocused, WindowMoved, WindowResized,
    WindowScaleFactorChanged, Windows,
};

use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{self, DeviceEvent, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

#[derive(Default)]
pub struct WinitPlugin;

impl Plugin for WinitPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<WinitWindows>()
            .init_resource::<WinitSettings>()
            .set_runner(winit_runner)
            .add_system_to_stage(CoreStage::PostUpdate, change_window.label(ModifiesWindows));
        #[cfg(target_arch = "wasm32")]
        app.add_plugin(web_resize::CanvasParentResizePlugin);
        let event_loop = EventLoop::new();
        #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
        let mut create_window_reader = WinitCreateWindowReader::default();
        #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
        let create_window_reader = WinitCreateWindowReader::default();
        // Note that we create a window here "early" because WASM/WebGL requires the window to exist prior to initializing
        // the renderer.
        // And for ios and macos, we should not create window early, all ui related code should be executed inside
        // UIApplicationMain/NSApplicationMain.
        #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
        handle_create_window_events(&mut app.world, &event_loop, &mut create_window_reader.0);
        app.insert_resource(create_window_reader)
            .insert_non_send_resource(event_loop);
    }
}

fn change_window(
    mut winit_windows: NonSendMut<WinitWindows>,
    mut windows: ResMut<Windows>,
    mut window_dpi_changed_events: EventWriter<WindowScaleFactorChanged>,
    mut window_close_events: EventWriter<WindowClosed>,
) {
    let mut removed_windows = vec![];
    for bevy_window in windows.iter_mut() {
        let id = bevy_window.id();
        for command in bevy_window.drain_commands() {
            match command {
                bevy_window::WindowCommand::SetWindowMode {
                    mode,
                    resolution:
                        UVec2 {
                            x: width,
                            y: height,
                        },
                } => {
                    let window = winit_windows.get_window(id).unwrap();
                    match mode {
                        bevy_window::WindowMode::BorderlessFullscreen => {
                            window
                                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        }
                        bevy_window::WindowMode::Fullscreen => {
                            window.set_fullscreen(Some(winit::window::Fullscreen::Exclusive(
                                get_best_videomode(&window.current_monitor().unwrap()),
                            )));
                        }
                        bevy_window::WindowMode::SizedFullscreen => window.set_fullscreen(Some(
                            winit::window::Fullscreen::Exclusive(get_fitting_videomode(
                                &window.current_monitor().unwrap(),
                                width,
                                height,
                            )),
                        )),
                        bevy_window::WindowMode::Windowed => window.set_fullscreen(None),
                    }
                }
                bevy_window::WindowCommand::SetTitle { title } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_title(&title);
                }
                bevy_window::WindowCommand::SetScaleFactor { scale_factor } => {
                    window_dpi_changed_events.send(WindowScaleFactorChanged { id, scale_factor });
                }
                bevy_window::WindowCommand::SetResolution {
                    logical_resolution:
                        Vec2 {
                            x: width,
                            y: height,
                        },
                    scale_factor,
                } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_inner_size(
                        winit::dpi::LogicalSize::new(width, height)
                            .to_physical::<f64>(scale_factor),
                    );
                }
                bevy_window::WindowCommand::SetPresentMode { .. } => (),
                bevy_window::WindowCommand::SetResizable { resizable } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_resizable(resizable);
                }
                bevy_window::WindowCommand::SetDecorations { decorations } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_decorations(decorations);
                }
                bevy_window::WindowCommand::SetCursorIcon { icon } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_cursor_icon(converters::convert_cursor_icon(icon));
                }
                bevy_window::WindowCommand::SetCursorGrabMode { grab_mode } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window
                        .set_cursor_grab(convert_cursor_grab_mode(grab_mode))
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
                        .set_cursor_position(LogicalPosition::new(
                            position.x,
                            inner_size.height - position.y,
                        ))
                        .unwrap_or_else(|e| error!("Unable to set cursor position: {}", e));
                }
                bevy_window::WindowCommand::SetMaximized { maximized } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_maximized(maximized);
                }
                bevy_window::WindowCommand::SetMinimized { minimized } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_minimized(minimized);
                }
                bevy_window::WindowCommand::SetPosition {
                    monitor_selection,
                    position,
                } => {
                    let window = winit_windows.get_window(id).unwrap();

                    use bevy_window::MonitorSelection::*;
                    let maybe_monitor = match monitor_selection {
                        Current => window.current_monitor(),
                        Primary => window.primary_monitor(),
                        Index(i) => window.available_monitors().nth(i),
                    };
                    if let Some(monitor) = maybe_monitor {
                        let monitor_position = DVec2::from(<(_, _)>::from(monitor.position()));
                        let position = monitor_position + position.as_dvec2();

                        window.set_outer_position(LogicalPosition::new(position.x, position.y));
                    } else {
                        warn!("Couldn't get monitor selected with: {monitor_selection:?}");
                    }
                }
                bevy_window::WindowCommand::Center(monitor_selection) => {
                    let window = winit_windows.get_window(id).unwrap();

                    use bevy_window::MonitorSelection::*;
                    let maybe_monitor = match monitor_selection {
                        Current => window.current_monitor(),
                        Primary => window.primary_monitor(),
                        Index(i) => window.available_monitors().nth(i),
                    };

                    if let Some(monitor) = maybe_monitor {
                        let monitor_size = monitor.size();
                        let monitor_position = monitor.position().cast::<f64>();

                        let window_size = window.outer_size();

                        window.set_outer_position(PhysicalPosition {
                            x: monitor_size.width.saturating_sub(window_size.width) as f64 / 2.
                                + monitor_position.x,
                            y: monitor_size.height.saturating_sub(window_size.height) as f64 / 2.
                                + monitor_position.y,
                        });
                    } else {
                        warn!("Couldn't get monitor selected with: {monitor_selection:?}");
                    }
                }
                bevy_window::WindowCommand::SetResizeConstraints { resize_constraints } => {
                    let window = winit_windows.get_window(id).unwrap();
                    let constraints = resize_constraints.check_constraints();
                    let min_inner_size = LogicalSize {
                        width: constraints.min_width,
                        height: constraints.min_height,
                    };
                    let max_inner_size = LogicalSize {
                        width: constraints.max_width,
                        height: constraints.max_height,
                    };

                    window.set_min_inner_size(Some(min_inner_size));
                    if constraints.max_width.is_finite() && constraints.max_height.is_finite() {
                        window.set_max_inner_size(Some(max_inner_size));
                    }
                }
                bevy_window::WindowCommand::SetAlwaysOnTop { always_on_top } => {
                    let window = winit_windows.get_window(id).unwrap();
                    window.set_always_on_top(always_on_top);
                }
                bevy_window::WindowCommand::Close => {
                    // Since we have borrowed `windows` to iterate through them, we can't remove the window from it.
                    // Add the removal requests to a queue to solve this
                    removed_windows.push(id);
                    // No need to run any further commands - this drops the rest of the commands, although the `bevy_window::Window` will be dropped later anyway
                    break;
                }
            }
        }
    }
    if !removed_windows.is_empty() {
        for id in removed_windows {
            // Close the OS window. (The `Drop` impl actually closes the window)
            let _ = winit_windows.remove_window(id);
            // Clean up our own data structures
            windows.remove(id);
            window_close_events.send(WindowClosed { id });
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
    event_loop.run_return(event_handler);
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
    winit_runner_with(app);
}

// #[cfg(any(
//     target_os = "linux",
//     target_os = "dragonfly",
//     target_os = "freebsd",
//     target_os = "netbsd",
//     target_os = "openbsd"
// ))]
// pub fn winit_runner_any_thread(app: App) {
//     winit_runner_with(app, EventLoop::new_any_thread());
// }

/// Stores state that must persist between frames.
struct WinitPersistentState {
    /// Tracks whether or not the application is active or suspended.
    active: bool,
    /// Tracks whether or not an event has occurred this frame that would trigger an update in low
    /// power mode. Should be reset at the end of every frame.
    low_power_event: bool,
    /// Tracks whether the event loop was started this frame because of a redraw request.
    redraw_request_sent: bool,
    /// Tracks if the event loop was started this frame because of a `WaitUntil` timeout.
    timeout_reached: bool,
    last_update: Instant,
}
impl Default for WinitPersistentState {
    fn default() -> Self {
        Self {
            active: true,
            low_power_event: false,
            redraw_request_sent: false,
            timeout_reached: false,
            last_update: Instant::now(),
        }
    }
}

#[derive(Default, Resource)]
struct WinitCreateWindowReader(ManualEventReader<CreateWindow>);

pub fn winit_runner_with(mut app: App) {
    let mut event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<()>>()
        .unwrap();
    let mut create_window_event_reader = app
        .world
        .remove_resource::<WinitCreateWindowReader>()
        .unwrap()
        .0;
    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();
    let mut winit_state = WinitPersistentState::default();
    app.world
        .insert_non_send_resource(event_loop.create_proxy());

    let return_from_run = app.world.resource::<WinitSettings>().return_from_run;

    trace!("Entering winit event loop");

    let event_handler = move |event: Event<()>,
                              event_loop: &EventLoopWindowTarget<()>,
                              control_flow: &mut ControlFlow| {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();
        match event {
            event::Event::NewEvents(start) => {
                let winit_config = app.world.resource::<WinitSettings>();
                let windows = app.world.resource::<Windows>();
                let focused = windows.iter().any(|w| w.is_focused());
                // Check if either the `WaitUntil` timeout was triggered by winit, or that same
                // amount of time has elapsed since the last app update. This manual check is needed
                // because we don't know if the criteria for an app update were met until the end of
                // the frame.
                let auto_timeout_reached = matches!(start, StartCause::ResumeTimeReached { .. });
                let now = Instant::now();
                let manual_timeout_reached = match winit_config.update_mode(focused) {
                    UpdateMode::Continuous => false,
                    UpdateMode::Reactive { max_wait }
                    | UpdateMode::ReactiveLowPower { max_wait } => {
                        now.duration_since(winit_state.last_update) >= *max_wait
                    }
                };
                // The low_power_event state and timeout must be reset at the start of every frame.
                winit_state.low_power_event = false;
                winit_state.timeout_reached = auto_timeout_reached || manual_timeout_reached;
            }
            event::Event::WindowEvent {
                event,
                window_id: winit_window_id,
                ..
            } => {
                let world = app.world.cell();
                let winit_windows = world.non_send_resource_mut::<WinitWindows>();
                let mut windows = world.resource_mut::<Windows>();
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

                let Some(window) = windows.get_mut(window_id) else {
                    // If we're here, this window was previously opened
                    info!("Skipped event for closed window: {:?}", window_id);
                    return;
                };
                winit_state.low_power_event = true;

                match event {
                    WindowEvent::Resized(size) => {
                        window.update_actual_size_from_backend(size.width, size.height);
                        world.send_event(WindowResized {
                            id: window_id,
                            width: window.width(),
                            height: window.height(),
                        });
                    }
                    WindowEvent::CloseRequested => {
                        world.send_event(WindowCloseRequested { id: window_id });
                    }
                    WindowEvent::KeyboardInput { ref input, .. } => {
                        world.send_event(converters::convert_keyboard_input(input));
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let winit_window = winit_windows.get_window(window_id).unwrap();
                        let inner_size = winit_window.inner_size();

                        // move origin to bottom left
                        let y_position = inner_size.height as f64 - position.y;

                        let physical_position = DVec2::new(position.x, y_position);
                        window
                            .update_cursor_physical_position_from_backend(Some(physical_position));

                        world.send_event(CursorMoved {
                            id: window_id,
                            position: (physical_position / window.scale_factor()).as_vec2(),
                        });
                    }
                    WindowEvent::CursorEntered { .. } => {
                        world.send_event(CursorEntered { id: window_id });
                    }
                    WindowEvent::CursorLeft { .. } => {
                        window.update_cursor_physical_position_from_backend(None);
                        world.send_event(CursorLeft { id: window_id });
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        world.send_event(MouseButtonInput {
                            button: converters::convert_mouse_button(button),
                            state: converters::convert_element_state(state),
                        });
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        event::MouseScrollDelta::LineDelta(x, y) => {
                            world.send_event(MouseWheel {
                                unit: MouseScrollUnit::Line,
                                x,
                                y,
                            });
                        }
                        event::MouseScrollDelta::PixelDelta(p) => {
                            world.send_event(MouseWheel {
                                unit: MouseScrollUnit::Pixel,
                                x: p.x as f32,
                                y: p.y as f32,
                            });
                        }
                    },
                    WindowEvent::Touch(touch) => {
                        let mut location = touch.location.to_logical(window.scale_factor());

                        // On a mobile window, the start is from the top while on PC/Linux/OSX from
                        // bottom
                        if cfg!(target_os = "android") || cfg!(target_os = "ios") {
                            let window_height = windows.primary().height();
                            location.y = window_height - location.y;
                        }

                        world.send_event(converters::convert_touch_input(touch, location));
                    }
                    WindowEvent::ReceivedCharacter(c) => {
                        world.send_event(ReceivedCharacter {
                            id: window_id,
                            char: c,
                        });
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {
                        world.send_event(WindowBackendScaleFactorChanged {
                            id: window_id,
                            scale_factor,
                        });
                        let prior_factor = window.scale_factor();
                        window.update_scale_factor_from_backend(scale_factor);
                        let new_factor = window.scale_factor();
                        if let Some(forced_factor) = window.scale_factor_override() {
                            // If there is a scale factor override, then force that to be used
                            // Otherwise, use the OS suggested size
                            // We have already told the OS about our resize constraints, so
                            // the new_inner_size should take those into account
                            *new_inner_size = winit::dpi::LogicalSize::new(
                                window.requested_width(),
                                window.requested_height(),
                            )
                            .to_physical::<u32>(forced_factor);
                        } else if approx::relative_ne!(new_factor, prior_factor) {
                            world.send_event(WindowScaleFactorChanged {
                                id: window_id,
                                scale_factor,
                            });
                        }

                        let new_logical_width = new_inner_size.width as f64 / new_factor;
                        let new_logical_height = new_inner_size.height as f64 / new_factor;
                        if approx::relative_ne!(window.width() as f64, new_logical_width)
                            || approx::relative_ne!(window.height() as f64, new_logical_height)
                        {
                            world.send_event(WindowResized {
                                id: window_id,
                                width: new_logical_width as f32,
                                height: new_logical_height as f32,
                            });
                        }
                        window.update_actual_size_from_backend(
                            new_inner_size.width,
                            new_inner_size.height,
                        );
                    }
                    WindowEvent::Focused(focused) => {
                        window.update_focused_status_from_backend(focused);
                        world.send_event(WindowFocused {
                            id: window_id,
                            focused,
                        });
                    }
                    WindowEvent::DroppedFile(path_buf) => {
                        world.send_event(FileDragAndDrop::DroppedFile {
                            id: window_id,
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFile(path_buf) => {
                        world.send_event(FileDragAndDrop::HoveredFile {
                            id: window_id,
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFileCancelled => {
                        world.send_event(FileDragAndDrop::HoveredFileCancelled { id: window_id });
                    }
                    WindowEvent::Moved(position) => {
                        let position = ivec2(position.x, position.y);
                        window.update_actual_position_from_backend(position);
                        world.send_event(WindowMoved {
                            id: window_id,
                            position,
                        });
                    }
                    _ => {}
                }
            }
            event::Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (x, y) },
                ..
            } => {
                app.world.send_event(MouseMotion {
                    delta: DVec2 { x, y }.as_vec2(),
                });
            }
            event::Event::Suspended => {
                winit_state.active = false;
            }
            event::Event::Resumed => {
                winit_state.active = true;
            }
            event::Event::MainEventsCleared => {
                handle_create_window_events(
                    &mut app.world,
                    event_loop,
                    &mut create_window_event_reader,
                );
                let winit_config = app.world.resource::<WinitSettings>();
                let update = if winit_state.active {
                    let windows = app.world.resource::<Windows>();
                    let focused = windows.iter().any(|w| w.is_focused());
                    match winit_config.update_mode(focused) {
                        UpdateMode::Continuous | UpdateMode::Reactive { .. } => true,
                        UpdateMode::ReactiveLowPower { .. } => {
                            winit_state.low_power_event
                                || winit_state.redraw_request_sent
                                || winit_state.timeout_reached
                        }
                    }
                } else {
                    false
                };
                if update {
                    winit_state.last_update = Instant::now();
                    app.update();
                }
            }
            Event::RedrawEventsCleared => {
                {
                    let winit_config = app.world.resource::<WinitSettings>();
                    let windows = app.world.resource::<Windows>();
                    let focused = windows.iter().any(|w| w.is_focused());
                    let now = Instant::now();
                    use UpdateMode::*;
                    *control_flow = match winit_config.update_mode(focused) {
                        Continuous => ControlFlow::Poll,
                        Reactive { max_wait } | ReactiveLowPower { max_wait } => {
                            if let Some(instant) = now.checked_add(*max_wait) {
                                ControlFlow::WaitUntil(instant)
                            } else {
                                ControlFlow::Wait
                            }
                        }
                    };
                }
                // This block needs to run after `app.update()` in `MainEventsCleared`. Otherwise,
                // we won't be able to see redraw requests until the next event, defeating the
                // purpose of a redraw request!
                let mut redraw = false;
                if let Some(app_redraw_events) = app.world.get_resource::<Events<RequestRedraw>>() {
                    if redraw_event_reader.iter(app_redraw_events).last().is_some() {
                        *control_flow = ControlFlow::Poll;
                        redraw = true;
                    }
                }
                if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                    if app_exit_event_reader.iter(app_exit_events).last().is_some() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                winit_state.redraw_request_sent = redraw;
            }
            _ => (),
        }
    };

    if return_from_run {
        run_return(&mut event_loop, event_handler);
    } else {
        run(event_loop, event_handler);
    }
}

fn handle_create_window_events(
    world: &mut World,
    event_loop: &EventLoopWindowTarget<()>,
    create_window_event_reader: &mut ManualEventReader<CreateWindow>,
) {
    let world = world.cell();
    let mut winit_windows = world.non_send_resource_mut::<WinitWindows>();
    let mut windows = world.resource_mut::<Windows>();
    let create_window_events = world.resource::<Events<CreateWindow>>();
    for create_window_event in create_window_event_reader.iter(&create_window_events) {
        let window = winit_windows.create_window(
            event_loop,
            create_window_event.id,
            &create_window_event.descriptor,
        );
        // This event is already sent on windows, x11, and xwayland.
        // TODO: we aren't yet sure about native wayland, so we might be able to exclude it,
        // but sending a duplicate event isn't problematic, as windows already does this.
        #[cfg(not(any(target_os = "windows", target_feature = "x11")))]
        world.send_event(WindowResized {
            id: create_window_event.id,
            width: window.width(),
            height: window.height(),
        });
        windows.add(window);
        world.send_event(WindowCreated {
            id: create_window_event.id,
        });

        #[cfg(target_arch = "wasm32")]
        {
            let channel = world.resource_mut::<web_resize::CanvasParentResizeEventChannel>();
            if create_window_event.descriptor.fit_canvas_to_parent {
                let selector = if let Some(selector) = &create_window_event.descriptor.canvas {
                    selector
                } else {
                    web_resize::WINIT_CANVAS_SELECTOR
                };
                channel.listen_to_selector(create_window_event.id, selector);
            }
        }
    }
}
