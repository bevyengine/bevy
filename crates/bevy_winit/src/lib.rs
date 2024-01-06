#![warn(missing_docs)]
//! `bevy_winit` provides utilities to handle window creation and the eventloop through [`winit`]
//!
//! Most commonly, the [`WinitPlugin`] is used as part of
//! [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html).
//! The app's [runner](bevy_app::App::runner) is set by `WinitPlugin` and handles the `winit` [`EventLoop`].
//! See `winit_runner` for details.

pub mod accessibility;
mod converters;
mod system;
mod winit_config;
mod winit_windows;

use bevy_a11y::AccessibilityRequested;
use bevy_utils::{Duration, Instant};
use system::{changed_windows, create_windows, despawn_windows, CachedWindow};
pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{App, AppExit, Last, Plugin, PluginsState};
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
use bevy_utils::tracing::{error, trace, warn};
use bevy_window::{
    exit_on_all_closed, ApplicationLifetime, CursorEntered, CursorLeft, CursorMoved,
    FileDragAndDrop, Ime, ReceivedCharacter, RequestRedraw, Window,
    WindowBackendScaleFactorChanged, WindowCloseRequested, WindowCreated, WindowDestroyed,
    WindowFocused, WindowMoved, WindowOccluded, WindowResized, WindowScaleFactorChanged,
    WindowThemeChanged,
};
#[cfg(target_os = "android")]
use bevy_window::{PrimaryWindow, RawHandleWrapper};

#[cfg(target_os = "android")]
pub use winit::platform::android::activity as android_activity;

use winit::{
    event::{self, DeviceEvent, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget},
};

use crate::accessibility::{AccessKitAdapters, AccessKitPlugin, WinitActionHandlers};

use crate::converters::convert_winit_theme;

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> =
    std::sync::OnceLock::new();

/// A [`Plugin`] that uses `winit` to create and manage windows, and receive window and input
/// events.
///
/// This plugin will add systems and resources that sync with the `winit` backend and also
/// replace the existing [`App`] runner with one that constructs an [event loop](EventLoop) to
/// receive window and input events from the OS.
#[derive(Default)]
pub struct WinitPlugin {
    /// Allows the window (and the event loop) to be created on any thread
    /// instead of only the main thread.
    ///
    /// See [`EventLoopBuilder::build`] for more information on this.
    ///
    /// # Supported platforms
    ///
    /// Only works on Linux (X11/Wayland) and Windows.
    /// This field is ignored on other platforms.
    pub run_on_any_thread: bool,
}

impl Plugin for WinitPlugin {
    fn build(&self, app: &mut App) {
        let mut event_loop_builder = EventLoopBuilder::<()>::with_user_event();

        // This is needed because the features checked in the inner
        // block might be enabled on other platforms than linux.
        #[cfg(target_os = "linux")]
        {
            #[cfg(feature = "x11")]
            {
                use winit::platform::x11::EventLoopBuilderExtX11;

                // This allows a Bevy app to be started and ran outside of the main thread.
                // A use case for this is to allow external applications to spawn a thread
                // which runs a Bevy app without requiring the Bevy app to need to reside on
                // the main thread, which can be problematic.
                event_loop_builder.with_any_thread(self.run_on_any_thread);
            }

            #[cfg(feature = "wayland")]
            {
                use winit::platform::wayland::EventLoopBuilderExtWayland;
                event_loop_builder.with_any_thread(self.run_on_any_thread);
            }
        }

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

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

        app.add_plugins(AccessKitPlugin);

        let event_loop = event_loop_builder
            .build()
            .expect("Failed to build event loop");

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
            );

            create_window_system_state.apply(&mut app.world);
        }

        // `winit`'s windows are bound to the event loop that created them, so the event loop must
        // be inserted as a resource here to pass it onto the runner.
        app.insert_non_send_resource(event_loop);
    }
}

#[derive(SystemParam)]
struct WindowAndInputEventWriters<'w> {
    // `winit` `WindowEvent`s
    window_resized: EventWriter<'w, WindowResized>,
    window_close_requested: EventWriter<'w, WindowCloseRequested>,
    window_scale_factor_changed: EventWriter<'w, WindowScaleFactorChanged>,
    window_backend_scale_factor_changed: EventWriter<'w, WindowBackendScaleFactorChanged>,
    window_focused: EventWriter<'w, WindowFocused>,
    window_occluded: EventWriter<'w, WindowOccluded>,
    window_moved: EventWriter<'w, WindowMoved>,
    window_theme_changed: EventWriter<'w, WindowThemeChanged>,
    window_destroyed: EventWriter<'w, WindowDestroyed>,
    lifetime: EventWriter<'w, ApplicationLifetime>,
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
    /// Current active state of the app.
    active: ActiveState,
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

#[derive(PartialEq, Eq)]
enum ActiveState {
    NotYetStarted,
    Active,
    Suspended,
    WillSuspend,
}

impl ActiveState {
    #[inline]
    fn should_run(&self) -> bool {
        match self {
            ActiveState::NotYetStarted | ActiveState::Suspended => false,
            ActiveState::Active | ActiveState::WillSuspend => true,
        }
    }
}

impl Default for WinitAppRunnerState {
    fn default() -> Self {
        Self {
            active: ActiveState::NotYetStarted,
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
    if app.plugins_state() == PluginsState::Ready {
        app.finish();
        app.cleanup();
    }

    let event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<()>>()
        .unwrap();

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
        NonSend<AccessKitAdapters>,
    )> = SystemState::new(&mut app.world);

    let mut create_window_system_state: SystemState<(
        Commands,
        Query<(Entity, &mut Window), Added<Window>>,
        EventWriter<WindowCreated>,
        NonSendMut<WinitWindows>,
        NonSendMut<AccessKitAdapters>,
        ResMut<WinitActionHandlers>,
        ResMut<AccessibilityRequested>,
    )> = SystemState::from_world(&mut app.world);

    // setup up the event loop
    let event_handler = move |event: Event<()>, event_loop: &EventLoopWindowTarget<()>| {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();

        if app.plugins_state() != PluginsState::Cleaned {
            if app.plugins_state() != PluginsState::Ready {
                #[cfg(not(target_arch = "wasm32"))]
                tick_global_task_pools_on_main_thread();
            } else {
                app.finish();
                app.cleanup();
            }

            if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                if app_exit_event_reader.read(app_exit_events).last().is_some() {
                    event_loop.exit();
                    return;
                }
            }
        }
        runner_state.redraw_requested = false;

        match event {
            Event::NewEvents(start_cause) => match start_cause {
                StartCause::Init => {
                    #[cfg(any(target_os = "ios", target_os = "macos"))]
                    {
                        let (
                            commands,
                            mut windows,
                            event_writer,
                            winit_windows,
                            adapters,
                            handlers,
                            accessibility_requested,
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
            Event::WindowEvent {
                event, window_id, ..
            } => {
                let (mut event_writers, winit_windows, mut windows, access_kit_adapters) =
                    event_writer_system_state.get_mut(&mut app.world);

                let Some(window_entity) = winit_windows.get_window_entity(window_id) else {
                    warn!(
                        "Skipped event {:?} for unknown winit Window Id {:?}",
                        event, window_id
                    );
                    return;
                };

                let Ok((mut window, _)) = windows.get_mut(window_entity) else {
                    warn!(
                        "Window {:?} is missing `Window` component, skipping event {:?}",
                        window_entity, event
                    );
                    return;
                };

                // Allow AccessKit to respond to `WindowEvent`s before they reach
                // the engine.
                if let Some(adapter) = access_kit_adapters.get(&window_entity) {
                    if let Some(window) = winit_windows.get_window(window_entity) {
                        adapter.process_event(window, &event);
                    }
                }

                runner_state.window_event_received = true;

                match event {
                    WindowEvent::Resized(size) => {
                        react_to_resize(&mut window, size, &mut event_writers, window_entity);
                    }
                    WindowEvent::CloseRequested => {
                        event_writers
                            .window_close_requested
                            .send(WindowCloseRequested {
                                window: window_entity,
                            });
                    }
                    WindowEvent::KeyboardInput { ref event, .. } => {
                        if event.state.is_pressed() {
                            if let Some(char) = &event.text {
                                event_writers.character_input.send(ReceivedCharacter {
                                    window: window_entity,
                                    char: char.clone(),
                                });
                            }
                        }
                        let keyboard_event =
                            converters::convert_keyboard_input(event, window_entity);
                        event_writers.keyboard_input.send(keyboard_event);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let physical_position = DVec2::new(position.x, position.y);
                        window.set_physical_cursor_position(Some(physical_position));
                        event_writers.cursor_moved.send(CursorMoved {
                            window: window_entity,
                            position: (physical_position / window.resolution.scale_factor() as f64)
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
                        let location = touch
                            .location
                            .to_logical(window.resolution.scale_factor() as f64);
                        event_writers
                            .touch_input
                            .send(converters::convert_touch_input(
                                touch,
                                location,
                                window_entity,
                            ));
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        mut inner_size_writer,
                    } => {
                        event_writers.window_backend_scale_factor_changed.send(
                            WindowBackendScaleFactorChanged {
                                window: window_entity,
                                scale_factor,
                            },
                        );

                        let prior_factor = window.resolution.scale_factor();
                        window.resolution.set_scale_factor(scale_factor as f32);
                        let new_factor = window.resolution.scale_factor();

                        let mut new_inner_size = winit::dpi::PhysicalSize::new(
                            window.physical_width(),
                            window.physical_height(),
                        );
                        if let Some(forced_factor) = window.resolution.scale_factor_override() {
                            // This window is overriding the OS-suggested DPI, so its physical size
                            // should be set based on the overriding value. Its logical size already
                            // incorporates any resize constraints.
                            let maybe_new_inner_size =
                                winit::dpi::LogicalSize::new(window.width(), window.height())
                                    .to_physical::<u32>(forced_factor as f64);
                            if let Err(err) = inner_size_writer.request_inner_size(new_inner_size) {
                                warn!("Winit Failed to resize the window: {err}");
                            } else {
                                new_inner_size = maybe_new_inner_size;
                            }
                        } else if approx::relative_ne!(new_factor, prior_factor) {
                            event_writers.window_scale_factor_changed.send(
                                WindowScaleFactorChanged {
                                    window: window_entity,
                                    scale_factor,
                                },
                            );
                        }
                        let new_logical_width = new_inner_size.width as f32 / new_factor;
                        let new_logical_height = new_inner_size.height as f32 / new_factor;
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
                    WindowEvent::Occluded(occluded) => {
                        event_writers.window_occluded.send(WindowOccluded {
                            window: window_entity,
                            occluded,
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
                        event::Ime::Commit(value) => {
                            event_writers.ime_input.send(Ime::Commit {
                                window: window_entity,
                                value,
                            });
                        }
                        event::Ime::Enabled => {
                            event_writers.ime_input.send(Ime::Enabled {
                                window: window_entity,
                            });
                        }
                        event::Ime::Disabled => {
                            event_writers.ime_input.send(Ime::Disabled {
                                window: window_entity,
                            });
                        }
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
                    WindowEvent::RedrawRequested => {
                        runner_state.redraw_requested = false;
                        run_app_update_if_should(
                            &mut runner_state,
                            &mut app,
                            &mut focused_windows_state,
                            event_loop,
                            &mut create_window_system_state,
                            &mut app_exit_event_reader,
                            &mut redraw_event_reader,
                        );
                    }
                    _ => {}
                }

                let mut windows = app.world.query::<(&mut Window, &mut CachedWindow)>();
                if let Ok((window, mut cache)) = windows.get_mut(&mut app.world, window_entity) {
                    if window.is_changed() {
                        cache.window = window.clone();
                    }
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (x, y) },
                ..
            } => {
                runner_state.redraw_requested = true;
                let (mut event_writers, ..) = event_writer_system_state.get_mut(&mut app.world);
                event_writers.mouse_motion.send(MouseMotion {
                    delta: Vec2::new(x as f32, y as f32),
                });
            }
            Event::Suspended => {
                let (mut event_writers, ..) = event_writer_system_state.get_mut(&mut app.world);
                event_writers.lifetime.send(ApplicationLifetime::Suspended);
                // Mark the state as `WillSuspend`. This will let the schedule run one last time
                // before actually suspending to let the application react
                runner_state.active = ActiveState::WillSuspend;
            }
            Event::Resumed => {
                let (mut event_writers, ..) = event_writer_system_state.get_mut(&mut app.world);
                match runner_state.active {
                    ActiveState::NotYetStarted => {
                        event_writers.lifetime.send(ApplicationLifetime::Started);
                    }
                    _ => {
                        event_writers.lifetime.send(ApplicationLifetime::Resumed);
                    }
                }
                runner_state.active = ActiveState::Active;
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
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
            _ => (),
        }
        if runner_state.redraw_requested {
            let (_, winit_windows, _, _) = event_writer_system_state.get_mut(&mut app.world);
            for window in winit_windows.windows.values() {
                window.request_redraw();
            }
        }
    };

    trace!("starting winit event loop");
    // TODO(clean): the winit docs mention using `spawn` instead of `run` on WASM.
    if let Err(err) = event_loop.run(event_handler) {
        error!("winit event loop returned an error: {err}");
    }
}

fn run_app_update_if_should(
    runner_state: &mut WinitAppRunnerState,
    app: &mut App,
    focused_windows_state: &mut SystemState<(Res<WinitSettings>, Query<&Window>)>,
    event_loop: &EventLoopWindowTarget<()>,
    create_window_system_state: &mut SystemState<(
        Commands,
        Query<(Entity, &mut Window), Added<Window>>,
        EventWriter<WindowCreated>,
        NonSendMut<WinitWindows>,
        NonSendMut<AccessKitAdapters>,
        ResMut<WinitActionHandlers>,
        ResMut<AccessibilityRequested>,
    )>,
    app_exit_event_reader: &mut ManualEventReader<AppExit>,
    redraw_event_reader: &mut ManualEventReader<RequestRedraw>,
) {
    if !runner_state.active.should_run() {
        return;
    }
    if runner_state.active == ActiveState::WillSuspend {
        runner_state.active = ActiveState::Suspended;
        #[cfg(target_os = "android")]
        {
            // Remove the `RawHandleWrapper` from the primary window.
            // This will trigger the surface destruction.
            let mut query = app.world.query_filtered::<Entity, With<PrimaryWindow>>();
            let entity = query.single(&app.world);
            app.world.entity_mut(entity).remove::<RawHandleWrapper>();
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
    let (config, windows) = focused_windows_state.get(&app.world);
    let focused = windows.iter().any(|window| window.focused);
    let should_update = match config.update_mode(focused) {
        // `Reactive`: In order for `event_handler` to have been called, either
        // we received a window or raw input event, the `wait` elapsed, or a
        // redraw was requested (by the app or the OS). There are no other
        // conditions, so we can just return `true` here.
        UpdateMode::Continuous | UpdateMode::Reactive { .. } => true,
        // TODO(bug): This is currently always true since we only run this function
        // if we received a `RequestRedraw` event.
        UpdateMode::ReactiveLowPower { .. } => {
            runner_state.wait_elapsed
                || runner_state.redraw_requested
                || runner_state.window_event_received
        }
    };

    if app.plugins_state() == PluginsState::Cleaned && should_update {
        // reset these on each update
        runner_state.wait_elapsed = false;
        runner_state.last_update = Instant::now();

        app.update();

        // decide when to run the next update
        let (config, windows) = focused_windows_state.get(&app.world);
        let focused = windows.iter().any(|window| window.focused);
        match config.update_mode(focused) {
            UpdateMode::Continuous => {
                runner_state.redraw_requested = true;
            }
            UpdateMode::Reactive { wait } | UpdateMode::ReactiveLowPower { wait } => {
                // TODO(bug): this is unexpected behavior.
                // When Reactive, user expects bevy to actually wait that amount of time,
                // and not potentially infinitely depending on plateform specifics (which this does)
                // Need to verify the plateform specifics (whether this can occur in
                // rare-but-possible cases) and replace this with a panic or a log warn!
                if let Some(next) = runner_state.last_update.checked_add(*wait) {
                    runner_state.scheduled_update = Some(next);
                    event_loop.set_control_flow(ControlFlow::WaitUntil(next));
                } else {
                    runner_state.scheduled_update = None;
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
        }

        if let Some(app_redraw_events) = app.world.get_resource::<Events<RequestRedraw>>() {
            if redraw_event_reader.read(app_redraw_events).last().is_some() {
                runner_state.redraw_requested = true;
            }
        }

        if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
            if app_exit_event_reader.read(app_exit_events).last().is_some() {
                event_loop.exit();
            }
        }
    }

    // create any new windows
    // (even if app did not update, some may have been created by plugin setup)
    let (
        commands,
        mut windows,
        event_writer,
        winit_windows,
        adapters,
        handlers,
        accessibility_requested,
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
    );

    create_window_system_state.apply(&mut app.world);
}

fn react_to_resize(
    window: &mut Mut<'_, Window>,
    size: winit::dpi::PhysicalSize<u32>,
    event_writers: &mut WindowAndInputEventWriters<'_>,
    window_entity: Entity,
) {
    window
        .resolution
        .set_physical_resolution(size.width, size.height);

    event_writers.window_resized.send(WindowResized {
        window: window_entity,
        width: window.width(),
        height: window.height(),
    });
}
