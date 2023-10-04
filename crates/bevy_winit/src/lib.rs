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
use system::{changed_windows, create_windows, despawn_windows, CachedWindow};
pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{App, AppExit, Last, Plugin};
use bevy_ecs::event::{Events, ManualEventReader};
use bevy_ecs::prelude::*;
use bevy_ecs::system::{SystemParam, SystemState};
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    touch::TouchInput,
    touchpad::{TouchpadMagnify, TouchpadRotate},
};
use bevy_math::{ivec2, DVec2, Vec2};
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;
use bevy_utils::{
    tracing::{trace, warn},
    Duration, Instant,
};
use bevy_window::{
    exit_on_all_closed, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime,
    ReceivedCharacter, RequestRedraw, Window, WindowBackendScaleFactorChanged,
    WindowCloseRequested, WindowCreated, WindowDestroyed, WindowFocused, WindowMoved,
    WindowResized, WindowScaleFactorChanged, WindowThemeChanged,
};
#[cfg(target_os = "android")]
use bevy_window::{PrimaryWindow, RawHandleWrapper};

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

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();

/// A [`Plugin`] that uses `winit` to create and manage windows, and receive window and input
/// events.
///
/// This plugin will add systems and resources that sync with the `winit` backend and also
/// replace the existing [`App`] runner with one that constructs an [event loop](EventLoop) to
/// receive window and input events from the OS.
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

        app.init_non_send_resource::<WinitWindows>()
            .init_resource::<WinitSettings>()
            .set_runner(winit_runner)
            .add_systems(
                Last,
                (
                    // `exit_on_all_closed` only checks if windows exist but doesn't access data,
                    // so we don't need to care about its ordering relative to `changed_windows`
                    changed_windows.ambiguous_with(exit_on_all_closed),
                    despawn_windows,
                )
                    .chain(),
            );

        app.add_plugins(AccessibilityPlugin);

        #[cfg(target_arch = "wasm32")]
        app.add_plugins(CanvasParentResizePlugin);

        let event_loop = event_loop_builder.build();

        // iOS, macOS, and Android don't like it if you create windows before the event loop is
        // initialized.
        //
        // See:
        // - https://github.com/rust-windowing/winit/blob/master/README.md#macos
        // - https://github.com/rust-windowing/winit/blob/master/README.md#ios
        #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
        {
            // Otherwise, we want to create a window before `bevy_render` initializes the renderer
            // so that we have a surface to use as a hint. This improves compatibility with `wgpu`
            // backends, especially WASM/WebGL2.
            #[cfg(not(target_arch = "wasm32"))]
            let mut create_window_system_state: SystemState<(
                Commands,
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
                Query<(Entity, &mut Window)>,
                EventWriter<WindowCreated>,
                NonSendMut<WinitWindows>,
                NonSendMut<AccessKitAdapters>,
                ResMut<WinitActionHandlers>,
                ResMut<AccessibilityRequested>,
                ResMut<CanvasParentResizeEventChannel>,
            )> = SystemState::from_world(&mut app.world);

            #[cfg(not(target_arch = "wasm32"))]
            let (
                commands,
                mut windows,
                event_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
            ) = create_window_system_state.get_mut(&mut app.world);

            #[cfg(target_arch = "wasm32")]
            let (
                commands,
                mut windows,
                event_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
                event_channel,
            ) = create_window_system_state.get_mut(&mut app.world);

            create_windows(
                &event_loop,
                commands,
                windows.iter_mut(),
                event_writer,
                winit_windows,
                adapters,
                handlers,
                accessibility_requested,
                #[cfg(target_arch = "wasm32")]
                event_channel,
            );

            create_window_system_state.apply(&mut app.world);
        }

        // `winit`'s windows are bound to the event loop that created them, so the event loop must
        // be inserted as a resource here to pass it onto the runner.
        app.insert_non_send_resource(event_loop);
    }
}

fn run<F, T>(event_loop: EventLoop<T>, event_handler: F) -> !
where
    F: 'static + FnMut(Event<'_, T>, &EventLoopWindowTarget<T>, &mut ControlFlow),
{
    event_loop.run(event_handler)
}

#[cfg(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn run_return<F, T>(event_loop: &mut EventLoop<T>, event_handler: F)
where
    F: FnMut(Event<'_, T>, &EventLoopWindowTarget<T>, &mut ControlFlow),
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
fn run_return<F, T>(_event_loop: &mut EventLoop<T>, _event_handler: F)
where
    F: FnMut(Event<'_, T>, &EventLoopWindowTarget<T>, &mut ControlFlow),
{
    panic!("Run return is not supported on this platform!")
}

#[derive(SystemParam)]
struct WindowAndInputEventWriters<'w> {
    // `winit` `WindowEvent`s
    window_resized: EventWriter<'w, WindowResized>,
    window_close_requested: EventWriter<'w, WindowCloseRequested>,
    window_scale_factor_changed: EventWriter<'w, WindowScaleFactorChanged>,
    window_backend_scale_factor_changed: EventWriter<'w, WindowBackendScaleFactorChanged>,
    window_focused: EventWriter<'w, WindowFocused>,
    window_moved: EventWriter<'w, WindowMoved>,
    window_theme_changed: EventWriter<'w, WindowThemeChanged>,
    window_destroyed: EventWriter<'w, WindowDestroyed>,
    keyboard_input: EventWriter<'w, KeyboardInput>,
    character_input: EventWriter<'w, ReceivedCharacter>,
    mouse_button_input: EventWriter<'w, MouseButtonInput>,
    touchpad_magnify_input: EventWriter<'w, TouchpadMagnify>,
    touchpad_rotate_input: EventWriter<'w, TouchpadRotate>,
    mouse_wheel_input: EventWriter<'w, MouseWheel>,
    touch_input: EventWriter<'w, TouchInput>,
    ime_input: EventWriter<'w, Ime>,
    file_drag_and_drop: EventWriter<'w, FileDragAndDrop>,
    cursor_moved: EventWriter<'w, CursorMoved>,
    cursor_entered: EventWriter<'w, CursorEntered>,
    cursor_left: EventWriter<'w, CursorLeft>,
    // `winit` `DeviceEvent`s
    mouse_motion: EventWriter<'w, MouseMotion>,
}

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
struct WinitAppRunnerState {
    /// Is `true` if the app is running and not suspended.
    is_active: bool,
    /// Is `true` if a new [`WindowEvent`] has been received since the last update.
    window_event_received: bool,
    /// Is `true` if the app has requested a redraw since the last update.
    redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update` to run another update.
    wait_elapsed: bool,
    /// The time the last update started.
    last_update: Instant,
    /// The time the next update is scheduled to start.
    scheduled_update: Option<Instant>,
}

impl Default for WinitAppRunnerState {
    fn default() -> Self {
        Self {
            is_active: false,
            window_event_received: false,
            redraw_requested: false,
            wait_elapsed: false,
            last_update: Instant::now(),
            scheduled_update: None,
        }
    }
}

/// The default [`App::runner`] for the [`WinitPlugin`] plugin.
///
/// Overriding the app's [runner](bevy_app::App::runner) while using `WinitPlugin` will bypass the
/// `EventLoop`.
pub fn winit_runner(mut app: App) {
    let mut event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<()>>()
        .unwrap();

    let return_from_run = app.world.resource::<WinitSettings>().return_from_run;

    app.world
        .insert_non_send_resource(event_loop.create_proxy());

    let mut runner_state = WinitAppRunnerState::default();

    // prepare structures to access data in the world
    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();

    let mut focused_windows_state: SystemState<(Res<WinitSettings>, Query<&Window>)> =
        SystemState::new(&mut app.world);

    let mut event_writer_system_state: SystemState<(
        WindowAndInputEventWriters,
        NonSend<WinitWindows>,
        Query<(&mut Window, &mut CachedWindow)>,
    )> = SystemState::new(&mut app.world);

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

    // setup up the event loop
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

            if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                if app_exit_event_reader.read(app_exit_events).last().is_some() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
        }

        match event {
            event::Event::NewEvents(start_cause) => match start_cause {
                StartCause::Init => {
                    #[cfg(any(target_os = "ios", target_os = "macos"))]
                    {
                        #[cfg(not(target_arch = "wasm32"))]
                        let (
                            commands,
                            mut windows,
                            event_writer,
                            winit_windows,
                            adapters,
                            handlers,
                            accessibility_requested,
                        ) = create_window_system_state.get_mut(&mut app.world);

                        #[cfg(target_arch = "wasm32")]
                        let (
                            commands,
                            mut windows,
                            event_writer,
                            winit_windows,
                            adapters,
                            handlers,
                            accessibility_requested,
                            event_channel,
                        ) = create_window_system_state.get_mut(&mut app.world);

                        create_windows(
                            event_loop,
                            commands,
                            windows.iter_mut(),
                            event_writer,
                            winit_windows,
                            adapters,
                            handlers,
                            accessibility_requested,
                            #[cfg(target_arch = "wasm32")]
                            event_channel,
                        );

                        create_window_system_state.apply(&mut app.world);
                    }
                }
                _ => {
                    if let Some(t) = runner_state.scheduled_update {
                        let now = Instant::now();
                        let remaining = t.checked_duration_since(now).unwrap_or(Duration::ZERO);
                        runner_state.wait_elapsed = remaining.is_zero();
                    }
                }
            },
            event::Event::WindowEvent {
                event, window_id, ..
            } => {
                let (mut event_writers, winit_windows, mut windows) =
                    event_writer_system_state.get_mut(&mut app.world);

                let Some(window_entity) = winit_windows.get_window_entity(window_id) else {
                    warn!(
                        "Skipped event {:?} for unknown winit Window Id {:?}",
                        event, window_id
                    );
                    return;
                };

                let Ok((mut window, mut cache)) = windows.get_mut(window_entity) else {
                    warn!(
                        "Window {:?} is missing `Window` component, skipping event {:?}",
                        window_entity, event
                    );
                    return;
                };

                runner_state.window_event_received = true;

                match event {
                    WindowEvent::Resized(size) => {
                        window
                            .resolution
                            .set_physical_resolution(size.width, size.height);

                        event_writers.window_resized.send(WindowResized {
                            window: window_entity,
                            width: window.width(),
                            height: window.height(),
                        });
                    }
                    WindowEvent::CloseRequested => {
                        event_writers
                            .window_close_requested
                            .send(WindowCloseRequested {
                                window: window_entity,
                            });
                    }
                    WindowEvent::KeyboardInput { ref input, .. } => {
                        event_writers
                            .keyboard_input
                            .send(converters::convert_keyboard_input(input, window_entity));
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let physical_position = DVec2::new(position.x, position.y);
                        window.set_physical_cursor_position(Some(physical_position));
                        event_writers.cursor_moved.send(CursorMoved {
                            window: window_entity,
                            position: (physical_position / window.resolution.scale_factor())
                                .as_vec2(),
                        });
                    }
                    WindowEvent::CursorEntered { .. } => {
                        event_writers.cursor_entered.send(CursorEntered {
                            window: window_entity,
                        });
                    }
                    WindowEvent::CursorLeft { .. } => {
                        window.set_physical_cursor_position(None);
                        event_writers.cursor_left.send(CursorLeft {
                            window: window_entity,
                        });
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        event_writers.mouse_button_input.send(MouseButtonInput {
                            button: converters::convert_mouse_button(button),
                            state: converters::convert_element_state(state),
                            window: window_entity,
                        });
                    }
                    WindowEvent::TouchpadMagnify { delta, .. } => {
                        event_writers
                            .touchpad_magnify_input
                            .send(TouchpadMagnify(delta as f32));
                    }
                    WindowEvent::TouchpadRotate { delta, .. } => {
                        event_writers
                            .touchpad_rotate_input
                            .send(TouchpadRotate(delta));
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        event::MouseScrollDelta::LineDelta(x, y) => {
                            event_writers.mouse_wheel_input.send(MouseWheel {
                                unit: MouseScrollUnit::Line,
                                x,
                                y,
                                window: window_entity,
                            });
                        }
                        event::MouseScrollDelta::PixelDelta(p) => {
                            event_writers.mouse_wheel_input.send(MouseWheel {
                                unit: MouseScrollUnit::Pixel,
                                x: p.x as f32,
                                y: p.y as f32,
                                window: window_entity,
                            });
                        }
                    },
                    WindowEvent::Touch(touch) => {
                        let location = touch.location.to_logical(window.resolution.scale_factor());
                        event_writers
                            .touch_input
                            .send(converters::convert_touch_input(touch, location));
                    }
                    WindowEvent::ReceivedCharacter(char) => {
                        event_writers.character_input.send(ReceivedCharacter {
                            window: window_entity,
                            char,
                        });
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {
                        event_writers.window_backend_scale_factor_changed.send(
                            WindowBackendScaleFactorChanged {
                                window: window_entity,
                                scale_factor,
                            },
                        );

                        let prior_factor = window.resolution.scale_factor();
                        window.resolution.set_scale_factor(scale_factor);
                        let new_factor = window.resolution.scale_factor();

                        if let Some(forced_factor) = window.resolution.scale_factor_override() {
                            // This window is overriding the OS-suggested DPI, so its physical size
                            // should be set based on the overriding value. Its logical size already
                            // incorporates any resize constraints.
                            *new_inner_size =
                                winit::dpi::LogicalSize::new(window.width(), window.height())
                                    .to_physical::<u32>(forced_factor);
                        } else if approx::relative_ne!(new_factor, prior_factor) {
                            event_writers.window_scale_factor_changed.send(
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
                            event_writers.window_resized.send(WindowResized {
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
                        window.focused = focused;
                        event_writers.window_focused.send(WindowFocused {
                            window: window_entity,
                            focused,
                        });
                    }
                    WindowEvent::DroppedFile(path_buf) => {
                        event_writers
                            .file_drag_and_drop
                            .send(FileDragAndDrop::DroppedFile {
                                window: window_entity,
                                path_buf,
                            });
                    }
                    WindowEvent::HoveredFile(path_buf) => {
                        event_writers
                            .file_drag_and_drop
                            .send(FileDragAndDrop::HoveredFile {
                                window: window_entity,
                                path_buf,
                            });
                    }
                    WindowEvent::HoveredFileCancelled => {
                        event_writers.file_drag_and_drop.send(
                            FileDragAndDrop::HoveredFileCanceled {
                                window: window_entity,
                            },
                        );
                    }
                    WindowEvent::Moved(position) => {
                        let position = ivec2(position.x, position.y);
                        window.position.set(position);
                        event_writers.window_moved.send(WindowMoved {
                            entity: window_entity,
                            position,
                        });
                    }
                    WindowEvent::Ime(event) => match event {
                        event::Ime::Preedit(value, cursor) => {
                            event_writers.ime_input.send(Ime::Preedit {
                                window: window_entity,
                                value,
                                cursor,
                            });
                        }
                        event::Ime::Commit(value) => event_writers.ime_input.send(Ime::Commit {
                            window: window_entity,
                            value,
                        }),
                        event::Ime::Enabled => event_writers.ime_input.send(Ime::Enabled {
                            window: window_entity,
                        }),
                        event::Ime::Disabled => event_writers.ime_input.send(Ime::Disabled {
                            window: window_entity,
                        }),
                    },
                    WindowEvent::ThemeChanged(theme) => {
                        event_writers.window_theme_changed.send(WindowThemeChanged {
                            window: window_entity,
                            theme: convert_winit_theme(theme),
                        });
                    }
                    WindowEvent::Destroyed => {
                        event_writers.window_destroyed.send(WindowDestroyed {
                            window: window_entity,
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
                let (mut event_writers, _, _) = event_writer_system_state.get_mut(&mut app.world);
                event_writers.mouse_motion.send(MouseMotion {
                    delta: Vec2::new(x as f32, y as f32),
                });
            }
            event::Event::Suspended => {
                runner_state.is_active = false;
                #[cfg(target_os = "android")]
                {
                    // Remove the `RawHandleWrapper` from the primary window.
                    // This will trigger the surface destruction.
                    let mut query = app.world.query_filtered::<Entity, With<PrimaryWindow>>();
                    let entity = query.single(&app.world);
                    app.world.entity_mut(entity).remove::<RawHandleWrapper>();
                    *control_flow = ControlFlow::Wait;
                }
            }
            event::Event::Resumed => {
                runner_state.is_active = true;
                #[cfg(target_os = "android")]
                {
                    // Get windows that are cached but without raw handles. Those window were already created, but got their
                    // handle wrapper removed when the app was suspended.
                    let mut query = app
                        .world
                        .query_filtered::<(Entity, &Window), (With<CachedWindow>, Without<bevy_window::RawHandleWrapper>)>();
                    if let Ok((entity, window)) = query.get_single(&app.world) {
                        use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
                        let window = window.clone();

                        let (
                            _,
                            _,
                            _,
                            mut winit_windows,
                            mut adapters,
                            mut handlers,
                            accessibility_requested,
                        ) = create_window_system_state.get_mut(&mut app.world);

                        let winit_window = winit_windows.create_window(
                            event_loop,
                            entity,
                            &window,
                            &mut adapters,
                            &mut handlers,
                            &accessibility_requested,
                        );

                        let wrapper = RawHandleWrapper {
                            window_handle: winit_window.raw_window_handle(),
                            display_handle: winit_window.raw_display_handle(),
                        };

                        app.world.entity_mut(entity).insert(wrapper);
                    }
                    *control_flow = ControlFlow::Poll;
                }
            }
            event::Event::MainEventsCleared => {
                if runner_state.is_active {
                    let (config, windows) = focused_windows_state.get(&app.world);
                    let focused = windows.iter().any(|window| window.focused);
                    let should_update = match config.update_mode(focused) {
                        UpdateMode::Continuous | UpdateMode::Reactive { .. } => {
                            // `Reactive`: In order for `event_handler` to have been called, either
                            // we received a window or raw input event, the `wait` elapsed, or a
                            // redraw was requested (by the app or the OS). There are no other
                            // conditions, so we can just return `true` here.
                            true
                        }
                        UpdateMode::ReactiveLowPower { .. } => {
                            runner_state.wait_elapsed
                                || runner_state.redraw_requested
                                || runner_state.window_event_received
                        }
                    };

                    if finished_and_setup_done && should_update {
                        // reset these on each update
                        runner_state.wait_elapsed = false;
                        runner_state.window_event_received = false;
                        runner_state.redraw_requested = false;
                        runner_state.last_update = Instant::now();

                        app.update();

                        // decide when to run the next update
                        let (config, windows) = focused_windows_state.get(&app.world);
                        let focused = windows.iter().any(|window| window.focused);
                        match config.update_mode(focused) {
                            UpdateMode::Continuous => *control_flow = ControlFlow::Poll,
                            UpdateMode::Reactive { wait }
                            | UpdateMode::ReactiveLowPower { wait } => {
                                if let Some(next) = runner_state.last_update.checked_add(*wait) {
                                    runner_state.scheduled_update = Some(next);
                                    *control_flow = ControlFlow::WaitUntil(next);
                                } else {
                                    runner_state.scheduled_update = None;
                                    *control_flow = ControlFlow::Wait;
                                }
                            }
                        }

                        if let Some(app_redraw_events) =
                            app.world.get_resource::<Events<RequestRedraw>>()
                        {
                            if redraw_event_reader.read(app_redraw_events).last().is_some() {
                                runner_state.redraw_requested = true;
                                *control_flow = ControlFlow::Poll;
                            }
                        }

                        if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                            if app_exit_event_reader.read(app_exit_events).last().is_some() {
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                    }

                    // create any new windows
                    // (even if app did not update, some may have been created by plugin setup)
                    #[cfg(not(target_arch = "wasm32"))]
                    let (
                        commands,
                        mut windows,
                        event_writer,
                        winit_windows,
                        adapters,
                        handlers,
                        accessibility_requested,
                    ) = create_window_system_state.get_mut(&mut app.world);

                    #[cfg(target_arch = "wasm32")]
                    let (
                        commands,
                        mut windows,
                        event_writer,
                        winit_windows,
                        adapters,
                        handlers,
                        accessibility_requested,
                        event_channel,
                    ) = create_window_system_state.get_mut(&mut app.world);

                    create_windows(
                        event_loop,
                        commands,
                        windows.iter_mut(),
                        event_writer,
                        winit_windows,
                        adapters,
                        handlers,
                        accessibility_requested,
                        #[cfg(target_arch = "wasm32")]
                        event_channel,
                    );

                    create_window_system_state.apply(&mut app.world);
                }
            }
            _ => (),
        }
    };

    trace!("starting winit event loop");
    if return_from_run {
        run_return(&mut event_loop, event_handler);
    } else {
        run(event_loop, event_handler);
    }
}
