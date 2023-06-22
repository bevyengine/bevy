#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
//! `bevy_winit` provides utilities to handle window creation and the eventloop through [`winit`]
//!
//! Most commonly, the [`WinitPlugin`] is used as part of
//! [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html).
//! The app's [runner](bevy_app::App::runner) is set by `WinitPlugin` and handles the `winit` [`EventLoop`](winit::event_loop::EventLoop).
//! See `winit_runner` for details.

pub mod accessibility;
mod converters;
mod system;
#[cfg(target_arch = "wasm32")]
mod web_resize;
mod winit_config;
mod winit_windows;

use bevy_a11y::AccessibilityRequested;
use bevy_ecs::system::{SystemParam, SystemState};
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;
use bevy_utils::Instant;
use system::{changed_window, create_window, despawn_window, CachedWindow};

pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{App, AppExit, Last, Plugin};
use bevy_ecs::event::{Events, ManualEventReader};
use bevy_ecs::prelude::*;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    touch::TouchInput,
    touchpad::{TouchpadMagnify, TouchpadRotate},
};
use bevy_math::{ivec2, DVec2, Vec2};
use bevy_utils::tracing::{trace, warn};
use bevy_window::{
    exit_on_all_closed, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime,
    ReceivedCharacter, RequestRedraw, Window, WindowBackendScaleFactorChanged,
    WindowCloseRequested, WindowCreated, WindowFocused, WindowMoved, WindowResized,
    WindowScaleFactorChanged, WindowThemeChanged,
};

#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

use winit::{
    event::{self, DeviceEvent, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget},
};

use crate::accessibility::{AccessKitAdapters, AccessibilityPlugin, WinitActionHandlers};

use crate::converters::convert_winit_theme;
#[cfg(target_arch = "wasm32")]
use crate::web_resize::{CanvasParentResizeEventChannel, CanvasParentResizePlugin};

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events (for example lifecycle and input events)
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();

/// A [`Plugin`] that utilizes [`winit`] for window creation and event loop management.
#[derive(Default)]
pub struct WinitPlugin;

impl Plugin for WinitPlugin {
    fn build(&self, app: &mut App) {
        let mut event_loop_builder = EventLoopBuilder::<()>::with_user_event();

        #[cfg(target_os = "android")]
        {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            event_loop_builder.with_android_app(
                ANDROID_APP
                    .get()
                    .expect("Bevy must be setup with the #[bevy_main] macro on Android")
                    .clone(),
            );
        }

        let event_loop = event_loop_builder.build();
        app.insert_non_send_resource(event_loop);

        app.init_non_send_resource::<WinitWindows>()
            .init_resource::<WinitSettings>()
            .set_runner(winit_runner)
            // exit_on_all_closed only uses the query to determine if the query is empty,
            // and so doesn't care about ordering relative to changed_window
            .add_systems(
                Last,
                (
                    changed_window.ambiguous_with(exit_on_all_closed),
                    // Update the state of the window before attempting to despawn to ensure consistent event ordering
                    despawn_window.after(changed_window),
                ),
            );

        app.add_plugins(AccessibilityPlugin);

        #[cfg(target_arch = "wasm32")]
        app.add_plugins(CanvasParentResizePlugin);

        #[cfg(not(target_arch = "wasm32"))]
        let mut create_window_system_state: SystemState<(
            Commands,
            NonSendMut<EventLoop<()>>,
            Query<(Entity, &mut Window)>,
            EventWriter<WindowCreated>,
            NonSendMut<WinitWindows>,
            NonSendMut<AccessKitAdapters>,
            ResMut<WinitActionHandlers>,
            ResMut<AccessibilityRequested>,
        )> = SystemState::from_world(&mut app.world);

        #[cfg(target_arch = "wasm32")]
        let mut create_window_system_state: SystemState<(
            Commands,
            NonSendMut<EventLoop<()>>,
            Query<(Entity, &mut Window)>,
            EventWriter<WindowCreated>,
            NonSendMut<WinitWindows>,
            NonSendMut<AccessKitAdapters>,
            ResMut<WinitActionHandlers>,
            ResMut<AccessibilityRequested>,
            ResMut<CanvasParentResizeEventChannel>,
        )> = SystemState::from_world(&mut app.world);

        // And for ios and macos, we should not create window early, all ui related code should be executed inside
        // UIApplicationMain/NSApplicationMain.
        #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
        {
            #[cfg(not(target_arch = "wasm32"))]
            let (
                commands,
                event_loop,
                mut new_windows,
                event_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
            ) = create_window_system_state.get_mut(&mut app.world);

            #[cfg(target_arch = "wasm32")]
            let (
                commands,
                event_loop,
                mut new_windows,
                event_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
                event_channel,
            ) = create_window_system_state.get_mut(&mut app.world);

            // Here we need to create a winit-window and give it a WindowHandle which the renderer can use.
            // It needs to be spawned before the start of the startup schedule, so we cannot use a regular system.
            // Instead we need to create the window and spawn it using direct world access
            create_window(
                commands,
                &event_loop,
                new_windows.iter_mut(),
                event_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
                #[cfg(target_arch = "wasm32")]
                event_channel,
            );
        }

        create_window_system_state.apply(&mut app.world);
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

#[derive(SystemParam)]
struct WindowEvents<'w> {
    window_resized: EventWriter<'w, WindowResized>,
    window_close_requested: EventWriter<'w, WindowCloseRequested>,
    window_scale_factor_changed: EventWriter<'w, WindowScaleFactorChanged>,
    window_backend_scale_factor_changed: EventWriter<'w, WindowBackendScaleFactorChanged>,
    window_focused: EventWriter<'w, WindowFocused>,
    window_moved: EventWriter<'w, WindowMoved>,
    window_theme_changed: EventWriter<'w, WindowThemeChanged>,
}

#[derive(SystemParam)]
struct InputEvents<'w> {
    keyboard_input: EventWriter<'w, KeyboardInput>,
    character_input: EventWriter<'w, ReceivedCharacter>,
    mouse_button_input: EventWriter<'w, MouseButtonInput>,
    touchpad_magnify_input: EventWriter<'w, TouchpadMagnify>,
    touchpad_rotate_input: EventWriter<'w, TouchpadRotate>,
    mouse_wheel_input: EventWriter<'w, MouseWheel>,
    touch_input: EventWriter<'w, TouchInput>,
    ime_input: EventWriter<'w, Ime>,
}

#[derive(SystemParam)]
struct CursorEvents<'w> {
    cursor_moved: EventWriter<'w, CursorMoved>,
    cursor_entered: EventWriter<'w, CursorEntered>,
    cursor_left: EventWriter<'w, CursorLeft>,
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
    /// Tracks if the event loop was started this frame because of a [`ControlFlow::WaitUntil`]
    /// timeout.
    timeout_reached: bool,
    last_update: Instant,
}
impl Default for WinitPersistentState {
    fn default() -> Self {
        Self {
            active: false,
            low_power_event: false,
            redraw_request_sent: false,
            timeout_reached: false,
            last_update: Instant::now(),
        }
    }
}

/// The default [`App::runner`] for the [`WinitPlugin`] plugin.
///
/// Overriding the app's [runner](bevy_app::App::runner) while using `WinitPlugin` will bypass the `EventLoop`.
pub fn winit_runner(mut app: App) {
    // We remove this so that we have ownership over it.
    let mut event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<()>>()
        .unwrap();

    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();
    let mut winit_state = WinitPersistentState::default();
    app.world
        .insert_non_send_resource(event_loop.create_proxy());

    let return_from_run = app.world.resource::<WinitSettings>().return_from_run;

    trace!("Entering winit event loop");

    let mut focused_window_state: SystemState<(Res<WinitSettings>, Query<&Window>)> =
        SystemState::from_world(&mut app.world);

    #[cfg(not(target_arch = "wasm32"))]
    let mut create_window_system_state: SystemState<(
        Commands,
        Query<(Entity, &mut Window), Added<Window>>,
        EventWriter<WindowCreated>,
        NonSendMut<WinitWindows>,
        NonSendMut<AccessKitAdapters>,
        ResMut<WinitActionHandlers>,
        ResMut<AccessibilityRequested>,
    )> = SystemState::from_world(&mut app.world);

    #[cfg(target_arch = "wasm32")]
    let mut create_window_system_state: SystemState<(
        Commands,
        Query<(Entity, &mut Window), Added<Window>>,
        EventWriter<WindowCreated>,
        NonSendMut<WinitWindows>,
        NonSendMut<AccessKitAdapters>,
        ResMut<WinitActionHandlers>,
        ResMut<AccessibilityRequested>,
        ResMut<CanvasParentResizeEventChannel>,
    )> = SystemState::from_world(&mut app.world);

    let mut finished_and_setup_done = false;

    let event_handler = move |event: Event<()>,
                              event_loop: &EventLoopWindowTarget<()>,
                              control_flow: &mut ControlFlow| {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();

        if !finished_and_setup_done {
            if !app.ready() {
                #[cfg(not(target_arch = "wasm32"))]
                tick_global_task_pools_on_main_thread();
            } else {
                app.finish();
                app.cleanup();
                finished_and_setup_done = true;
            }
        }

        if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
            if app_exit_event_reader.iter(app_exit_events).last().is_some() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        match event {
            event::Event::NewEvents(start) => {
                let (winit_config, window_focused_query) = focused_window_state.get(&app.world);

                let app_focused = window_focused_query.iter().any(|window| window.focused);

                // Check if either the `WaitUntil` timeout was triggered by winit, or that same
                // amount of time has elapsed since the last app update. This manual check is needed
                // because we don't know if the criteria for an app update were met until the end of
                // the frame.
                let auto_timeout_reached = matches!(start, StartCause::ResumeTimeReached { .. });
                let now = Instant::now();
                let manual_timeout_reached = match winit_config.update_mode(app_focused) {
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
                // Fetch and prepare details from the world
                let mut system_state: SystemState<(
                    NonSend<WinitWindows>,
                    Query<(&mut Window, &mut CachedWindow)>,
                    WindowEvents,
                    InputEvents,
                    CursorEvents,
                    EventWriter<FileDragAndDrop>,
                )> = SystemState::new(&mut app.world);
                let (
                    winit_windows,
                    mut window_query,
                    mut window_events,
                    mut input_events,
                    mut cursor_events,
                    mut file_drag_and_drop_events,
                ) = system_state.get_mut(&mut app.world);

                // Entity of this window
                let window_entity =
                    if let Some(entity) = winit_windows.get_window_entity(winit_window_id) {
                        entity
                    } else {
                        warn!(
                            "Skipped event {:?} for unknown winit Window Id {:?}",
                            event, winit_window_id
                        );
                        return;
                    };

                let (mut window, mut cache) =
                    if let Ok((window, info)) = window_query.get_mut(window_entity) {
                        (window, info)
                    } else {
                        warn!(
                            "Window {:?} is missing `Window` component, skipping event {:?}",
                            window_entity, event
                        );
                        return;
                    };

                winit_state.low_power_event = true;

                match event {
                    WindowEvent::Resized(size) => {
                        window
                            .resolution
                            .set_physical_resolution(size.width, size.height);

                        window_events.window_resized.send(WindowResized {
                            window: window_entity,
                            width: window.width(),
                            height: window.height(),
                        });
                    }
                    WindowEvent::CloseRequested => {
                        window_events
                            .window_close_requested
                            .send(WindowCloseRequested {
                                window: window_entity,
                            });
                    }
                    WindowEvent::KeyboardInput { ref event, .. } => {
                        let keyboard_event =
                            converters::convert_keyboard_input(event, window_entity);
                        if let bevy_input::keyboard::Key::Character(c) = &keyboard_event.logical_key
                        {
                            if let Some(first_char) = c.chars().next() {
                                input_events.character_input.send(ReceivedCharacter {
                                    window: window_entity,
                                    char: first_char,
                                });
                            }
                        }
                        input_events.keyboard_input.send(keyboard_event);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let physical_position = DVec2::new(position.x, position.y);

                        window.set_physical_cursor_position(Some(physical_position));

                        cursor_events.cursor_moved.send(CursorMoved {
                            window: window_entity,
                            position: (physical_position / window.resolution.scale_factor())
                                .as_vec2(),
                        });
                    }
                    WindowEvent::CursorEntered { .. } => {
                        cursor_events.cursor_entered.send(CursorEntered {
                            window: window_entity,
                        });
                    }
                    WindowEvent::CursorLeft { .. } => {
                        window.set_physical_cursor_position(None);

                        cursor_events.cursor_left.send(CursorLeft {
                            window: window_entity,
                        });
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        input_events.mouse_button_input.send(MouseButtonInput {
                            button: converters::convert_mouse_button(button),
                            state: converters::convert_element_state(state),
                            window: window_entity,
                        });
                    }
                    WindowEvent::TouchpadMagnify { delta, .. } => {
                        input_events
                            .touchpad_magnify_input
                            .send(TouchpadMagnify(delta as f32));
                    }
                    WindowEvent::TouchpadRotate { delta, .. } => {
                        input_events
                            .touchpad_rotate_input
                            .send(TouchpadRotate(delta));
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        event::MouseScrollDelta::LineDelta(x, y) => {
                            input_events.mouse_wheel_input.send(MouseWheel {
                                unit: MouseScrollUnit::Line,
                                x,
                                y,
                                window: window_entity,
                            });
                        }
                        event::MouseScrollDelta::PixelDelta(p) => {
                            input_events.mouse_wheel_input.send(MouseWheel {
                                unit: MouseScrollUnit::Pixel,
                                x: p.x as f32,
                                y: p.y as f32,
                                window: window_entity,
                            });
                        }
                    },
                    WindowEvent::Touch(touch) => {
                        let location = touch.location.to_logical(window.resolution.scale_factor());

                        // Event
                        input_events
                            .touch_input
                            .send(converters::convert_touch_input(touch, location));
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {
                        window_events.window_backend_scale_factor_changed.send(
                            WindowBackendScaleFactorChanged {
                                window: window_entity,
                                scale_factor,
                            },
                        );

                        let prior_factor = window.resolution.scale_factor();
                        window.resolution.set_scale_factor(scale_factor);
                        let new_factor = window.resolution.scale_factor();

                        if let Some(forced_factor) = window.resolution.scale_factor_override() {
                            // If there is a scale factor override, then force that to be used
                            // Otherwise, use the OS suggested size
                            // We have already told the OS about our resize constraints, so
                            // the new_inner_size should take those into account
                            *new_inner_size =
                                winit::dpi::LogicalSize::new(window.width(), window.height())
                                    .to_physical::<u32>(forced_factor);
                            // TODO: Should this not trigger a WindowsScaleFactorChanged?
                        } else if approx::relative_ne!(new_factor, prior_factor) {
                            // Trigger a change event if they are approximately different
                            window_events.window_scale_factor_changed.send(
                                WindowScaleFactorChanged {
                                    window: window_entity,
                                    scale_factor,
                                },
                            );
                        }

                        let new_logical_width = (new_inner_size.width as f64 / new_factor) as f32;
                        let new_logical_height = (new_inner_size.height as f64 / new_factor) as f32;
                        if approx::relative_ne!(window.width(), new_logical_width)
                            || approx::relative_ne!(window.height(), new_logical_height)
                        {
                            window_events.window_resized.send(WindowResized {
                                window: window_entity,
                                width: new_logical_width,
                                height: new_logical_height,
                            });
                        }
                        window
                            .resolution
                            .set_physical_resolution(new_inner_size.width, new_inner_size.height);
                    }
                    WindowEvent::Focused(focused) => {
                        // Component
                        window.focused = focused;

                        window_events.window_focused.send(WindowFocused {
                            window: window_entity,
                            focused,
                        });
                    }
                    WindowEvent::DroppedFile(path_buf) => {
                        file_drag_and_drop_events.send(FileDragAndDrop::DroppedFile {
                            window: window_entity,
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFile(path_buf) => {
                        file_drag_and_drop_events.send(FileDragAndDrop::HoveredFile {
                            window: window_entity,
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFileCancelled => {
                        file_drag_and_drop_events.send(FileDragAndDrop::HoveredFileCanceled {
                            window: window_entity,
                        });
                    }
                    WindowEvent::Moved(position) => {
                        let position = ivec2(position.x, position.y);

                        window.position.set(position);

                        window_events.window_moved.send(WindowMoved {
                            entity: window_entity,
                            position,
                        });
                    }
                    WindowEvent::Ime(event) => match event {
                        event::Ime::Preedit(value, cursor) => {
                            input_events.ime_input.send(Ime::Preedit {
                                window: window_entity,
                                value,
                                cursor,
                            });
                        }
                        event::Ime::Commit(value) => input_events.ime_input.send(Ime::Commit {
                            window: window_entity,
                            value,
                        }),
                        event::Ime::Enabled => input_events.ime_input.send(Ime::Enabled {
                            window: window_entity,
                        }),
                        event::Ime::Disabled => input_events.ime_input.send(Ime::Disabled {
                            window: window_entity,
                        }),
                    },
                    WindowEvent::ThemeChanged(theme) => {
                        window_events.window_theme_changed.send(WindowThemeChanged {
                            window: window_entity,
                            theme: convert_winit_theme(theme),
                        });
                    }
                    _ => {}
                }

                if window.is_changed() {
                    cache.window = window.clone();
                }
            }
            event::Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (x, y) },
                ..
            } => {
                let mut system_state: SystemState<EventWriter<MouseMotion>> =
                    SystemState::new(&mut app.world);
                let mut mouse_motion = system_state.get_mut(&mut app.world);

                mouse_motion.send(MouseMotion {
                    delta: Vec2::new(x as f32, y as f32),
                });
            }
            event::Event::Suspended => {
                winit_state.active = false;
                #[cfg(target_os = "android")]
                {
                    // Bevy doesn't support suspend/resume so we just exit
                    // and Android will restart the application on resume
                    // TODO: Save save some state and load on resume
                    *control_flow = ControlFlow::Exit;
                }
            }
            event::Event::Resumed => {
                winit_state.active = true;
            }
            event::Event::MainEventsCleared => {
                let (winit_config, window_focused_query) = focused_window_state.get(&app.world);

                let update = if winit_state.active {
                    // True if _any_ windows are currently being focused
                    let app_focused = window_focused_query.iter().any(|window| window.focused);
                    match winit_config.update_mode(app_focused) {
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

                if update && finished_and_setup_done {
                    winit_state.last_update = Instant::now();
                    app.update();
                }
            }
            Event::RedrawEventsCleared => {
                {
                    // Fetch from world
                    let (winit_config, window_focused_query) = focused_window_state.get(&app.world);

                    // True if _any_ windows are currently being focused
                    let app_focused = window_focused_query.iter().any(|window| window.focused);

                    let now = Instant::now();
                    use UpdateMode::*;
                    *control_flow = match winit_config.update_mode(app_focused) {
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

                winit_state.redraw_request_sent = redraw;
            }

            _ => (),
        }

        if winit_state.active {
            #[cfg(not(target_arch = "wasm32"))]
            let (
                commands,
                mut new_windows,
                created_window_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
            ) = create_window_system_state.get_mut(&mut app.world);

            #[cfg(target_arch = "wasm32")]
            let (
                commands,
                mut new_windows,
                created_window_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
                canvas_parent_resize_channel,
            ) = create_window_system_state.get_mut(&mut app.world);

            // Responsible for creating new windows
            create_window(
                commands,
                event_loop,
                new_windows.iter_mut(),
                created_window_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
                #[cfg(target_arch = "wasm32")]
                canvas_parent_resize_channel,
            );

            create_window_system_state.apply(&mut app.world);
        }
    };

    // If true, returns control from Winit back to the main Bevy loop
    if return_from_run {
        run_return(&mut event_loop, event_handler);
    } else {
        run(event_loop, event_handler);
    }
}
