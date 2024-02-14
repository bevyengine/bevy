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

use approx::relative_eq;
use bevy_a11y::AccessibilityRequested;
use bevy_utils::{Duration, Instant};
use system::{changed_windows, create_windows, despawn_windows, CachedWindow};
use winit::dpi::{LogicalSize, PhysicalSize};
pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{App, AppExit, Last, Plugin, PluginsState};
use bevy_ecs::event::{Events, ManualEventReader};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_input::{
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
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
    event::{self, DeviceEvent, Event, WindowEvent},
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

        // linux check is needed because x11 might be enabled on other platforms.
        #[cfg(all(target_os = "linux", feature = "x11"))]
        {
            use winit::platform::x11::EventLoopBuilderExtX11;

            // This allows a Bevy app to be started and ran outside of the main thread.
            // A use case for this is to allow external applications to spawn a thread
            // which runs a Bevy app without requiring the Bevy app to need to reside on
            // the main thread, which can be problematic.
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

        // linux check is needed because wayland might be enabled on other platforms.
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        {
            use winit::platform::wayland::EventLoopBuilderExtWayland;
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

        #[cfg(target_os = "android")]
        {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            let msg = "Bevy must be setup with the #[bevy_main] macro on Android";
            event_loop_builder.with_android_app(ANDROID_APP.get().expect(msg).clone());
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
            let mut create_window = SystemState::<CreateWindowParams>::from_world(&mut app.world);
            create_windows(&event_loop, create_window.get_mut(&mut app.world));
            create_window.apply(&mut app.world);
        }

        // `winit`'s windows are bound to the event loop that created them, so the event loop must
        // be inserted as a resource here to pass it onto the runner.
        app.insert_non_send_resource(event_loop);
    }
}

trait AppSendEvent {
    fn send_event<E: bevy_ecs::event::Event>(&mut self, event: E);
}
impl AppSendEvent for App {
    fn send_event<E: bevy_ecs::event::Event>(&mut self, event: E) {
        self.world.send_event(event);
    }
}

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
struct WinitAppRunnerState {
    /// Current active state of the app.
    active: ActiveState,
    /// Is `true` if a new [`WindowEvent`] has been received since the last update.
    window_event_received: bool,
    /// Is `true` if a new [`DeviceEvent`] has been received since the last update.
    device_event_received: bool,
    /// Is `true` if the app has requested a redraw since the last update.
    redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update` to run another update.
    wait_elapsed: bool,
    /// The time the last update started.
    last_update: Instant,
    /// The time the next update is scheduled to start.
    scheduled_update: Option<Instant>,
    /// Number of "forced" updates to trigger on application start
    startup_forced_updates: u32,
}

impl WinitAppRunnerState {
    fn reset_on_update(&mut self) {
        self.redraw_requested = false;
        self.window_event_received = false;
        self.device_event_received = false;
        self.wait_elapsed = false;
    }
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
            device_event_received: false,
            redraw_requested: false,
            wait_elapsed: false,
            last_update: Instant::now(),
            scheduled_update: None,
            // 3 seems to be enough, 5 is a safe margin
            startup_forced_updates: 5,
        }
    }
}

type CreateWindowParams<'w, 's, F = ()> = (
    Commands<'w, 's>,
    Query<'w, 's, (Entity, &'static mut Window), F>,
    EventWriter<'w, WindowCreated>,
    NonSendMut<'w, WinitWindows>,
    NonSendMut<'w, AccessKitAdapters>,
    ResMut<'w, WinitActionHandlers>,
    Res<'w, AccessibilityRequested>,
);

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
        EventWriter<WindowResized>,
        NonSend<WinitWindows>,
        Query<(&mut Window, &mut CachedWindow)>,
        NonSend<AccessKitAdapters>,
    )> = SystemState::new(&mut app.world);

    let mut create_window =
        SystemState::<CreateWindowParams<Added<Window>>>::from_world(&mut app.world);
    // set up the event loop
    let event_handler = move |event, event_loop: &EventLoopWindowTarget<()>| {
        handle_winit_event(
            &mut app,
            &mut app_exit_event_reader,
            &mut runner_state,
            &mut create_window,
            &mut event_writer_system_state,
            &mut focused_windows_state,
            &mut redraw_event_reader,
            event,
            event_loop,
        );
    };

    trace!("starting winit event loop");
    // TODO(clean): the winit docs mention using `spawn` instead of `run` on WASM.
    if let Err(err) = event_loop.run(event_handler) {
        error!("winit event loop returned an error: {err}");
    }
}

#[allow(clippy::too_many_arguments /* TODO: probs can reduce # of args */)]
fn handle_winit_event(
    app: &mut App,
    app_exit_event_reader: &mut ManualEventReader<AppExit>,
    runner_state: &mut WinitAppRunnerState,
    create_window: &mut SystemState<CreateWindowParams<Added<Window>>>,
    event_writer_system_state: &mut SystemState<(
        EventWriter<WindowResized>,
        NonSend<WinitWindows>,
        Query<(&mut Window, &mut CachedWindow)>,
        NonSend<AccessKitAdapters>,
    )>,
    focused_windows_state: &mut SystemState<(Res<WinitSettings>, Query<&Window>)>,
    redraw_event_reader: &mut ManualEventReader<RequestRedraw>,
    event: Event<()>,
    event_loop: &EventLoopWindowTarget<()>,
) {
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
        runner_state.redraw_requested = true;

        if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
            if app_exit_event_reader.read(app_exit_events).last().is_some() {
                event_loop.exit();
                return;
            }
        }
    }

    match event {
        Event::AboutToWait => {
            let (config, windows) = focused_windows_state.get(&app.world);
            let focused = windows.iter().any(|window| window.focused);
            let mut should_update = match config.update_mode(focused) {
                UpdateMode::Continuous => {
                    runner_state.redraw_requested
                        || runner_state.window_event_received
                        || runner_state.device_event_received
                }
                UpdateMode::Reactive { .. } => {
                    runner_state.wait_elapsed
                        || runner_state.redraw_requested
                        || runner_state.window_event_received
                        || runner_state.device_event_received
                }
                UpdateMode::ReactiveLowPower { .. } => {
                    runner_state.wait_elapsed
                        || runner_state.redraw_requested
                        || runner_state.window_event_received
                }
            };

            // Ensure that an update is triggered on the first iterations for app initialization
            if runner_state.startup_forced_updates > 0 {
                runner_state.startup_forced_updates -= 1;
                should_update = true;
            }

            // Trigger one last update to enter suspended state
            if runner_state.active == ActiveState::WillSuspend {
                should_update = true;
            }

            if should_update {
                let visible = windows.iter().any(|window| window.visible);
                let (_, winit_windows, _, _) = event_writer_system_state.get_mut(&mut app.world);
                if visible && runner_state.active != ActiveState::WillSuspend {
                    for window in winit_windows.windows.values() {
                        window.request_redraw();
                    }
                } else {
                    // there are no windows, or they are not visible.
                    // Winit won't send events on some platforms, so trigger an update manually.
                    run_app_update_if_should(
                        runner_state,
                        app,
                        focused_windows_state,
                        event_loop,
                        create_window,
                        app_exit_event_reader,
                        redraw_event_reader,
                    );
                    if runner_state.active != ActiveState::Suspended {
                        event_loop.set_control_flow(ControlFlow::Poll);
                    }
                }
            }
        }
        Event::NewEvents(_) => {
            if let Some(t) = runner_state.scheduled_update {
                let now = Instant::now();
                let remaining = t.checked_duration_since(now).unwrap_or(Duration::ZERO);
                runner_state.wait_elapsed = remaining.is_zero();
            }
        }
        Event::WindowEvent {
            event, window_id, ..
        } => {
            let (mut window_resized, winit_windows, mut windows, access_kit_adapters) =
                event_writer_system_state.get_mut(&mut app.world);

            let Some(window) = winit_windows.get_window_entity(window_id) else {
                warn!("Skipped event {event:?} for unknown winit Window Id {window_id:?}");
                return;
            };

            let Ok((mut win, _)) = windows.get_mut(window) else {
                warn!("Window {window:?} is missing `Window` component, skipping event {event:?}");
                return;
            };

            // Allow AccessKit to respond to `WindowEvent`s before they reach
            // the engine.
            if let Some(adapter) = access_kit_adapters.get(&window) {
                if let Some(winit_window) = winit_windows.get_window(window) {
                    adapter.process_event(winit_window, &event);
                }
            }

            runner_state.window_event_received = true;

            match event {
                WindowEvent::Resized(size) => {
                    react_to_resize(&mut win, size, &mut window_resized, window);
                }
                WindowEvent::CloseRequested => app.send_event(WindowCloseRequested { window }),
                WindowEvent::KeyboardInput { ref event, .. } => {
                    if event.state.is_pressed() {
                        if let Some(char) = &event.text {
                            let char = char.clone();
                            app.send_event(ReceivedCharacter { window, char });
                        }
                    }
                    app.send_event(converters::convert_keyboard_input(event, window));
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let physical_position = DVec2::new(position.x, position.y);

                    let last_position = win.physical_cursor_position();
                    let delta = last_position.map(|last_pos| {
                        (physical_position.as_vec2() - last_pos) / win.resolution.scale_factor()
                    });

                    win.set_physical_cursor_position(Some(physical_position));
                    let position =
                        (physical_position / win.resolution.scale_factor() as f64).as_vec2();
                    app.send_event(CursorMoved {
                        window,
                        position,
                        delta,
                    });
                }
                WindowEvent::CursorEntered { .. } => {
                    app.send_event(CursorEntered { window });
                }
                WindowEvent::CursorLeft { .. } => {
                    win.set_physical_cursor_position(None);
                    app.send_event(CursorLeft { window });
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    app.send_event(MouseButtonInput {
                        button: converters::convert_mouse_button(button),
                        state: converters::convert_element_state(state),
                        window,
                    });
                }
                WindowEvent::TouchpadMagnify { delta, .. } => {
                    app.send_event(TouchpadMagnify(delta as f32));
                }
                WindowEvent::TouchpadRotate { delta, .. } => {
                    app.send_event(TouchpadRotate(delta));
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    event::MouseScrollDelta::LineDelta(x, y) => {
                        app.send_event(MouseWheel {
                            unit: MouseScrollUnit::Line,
                            x,
                            y,
                            window,
                        });
                    }
                    event::MouseScrollDelta::PixelDelta(p) => {
                        app.send_event(MouseWheel {
                            unit: MouseScrollUnit::Pixel,
                            x: p.x as f32,
                            y: p.y as f32,
                            window,
                        });
                    }
                },
                WindowEvent::Touch(touch) => {
                    let location = touch
                        .location
                        .to_logical(win.resolution.scale_factor() as f64);
                    app.send_event(converters::convert_touch_input(touch, location, window));
                }
                WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    mut inner_size_writer,
                } => {
                    let prior_factor = win.resolution.scale_factor();
                    win.resolution.set_scale_factor(scale_factor as f32);
                    // Note: this may be different from new_scale_factor if
                    // `scale_factor_override` is set to Some(thing)
                    let new_factor = win.resolution.scale_factor();

                    let mut new_inner_size =
                        PhysicalSize::new(win.physical_width(), win.physical_height());
                    let scale_factor_override = win.resolution.scale_factor_override();
                    if let Some(forced_factor) = scale_factor_override {
                        // This window is overriding the OS-suggested DPI, so its physical size
                        // should be set based on the overriding value. Its logical size already
                        // incorporates any resize constraints.
                        let maybe_new_inner_size = LogicalSize::new(win.width(), win.height())
                            .to_physical::<u32>(forced_factor as f64);
                        if let Err(err) = inner_size_writer.request_inner_size(new_inner_size) {
                            warn!("Winit Failed to resize the window: {err}");
                        } else {
                            new_inner_size = maybe_new_inner_size;
                        }
                    }
                    let new_logical_width = new_inner_size.width as f32 / new_factor;
                    let new_logical_height = new_inner_size.height as f32 / new_factor;

                    let width_equal = relative_eq!(win.width(), new_logical_width);
                    let height_equal = relative_eq!(win.height(), new_logical_height);
                    win.resolution
                        .set_physical_resolution(new_inner_size.width, new_inner_size.height);

                    app.send_event(WindowBackendScaleFactorChanged {
                        window,
                        scale_factor,
                    });
                    if scale_factor_override.is_none() && !relative_eq!(new_factor, prior_factor) {
                        app.send_event(WindowScaleFactorChanged {
                            window,
                            scale_factor,
                        });
                    }

                    if !width_equal || !height_equal {
                        app.send_event(WindowResized {
                            window,
                            width: new_logical_width,
                            height: new_logical_height,
                        });
                    }
                }
                WindowEvent::Focused(focused) => {
                    win.focused = focused;
                    app.send_event(WindowFocused { window, focused });
                }
                WindowEvent::Occluded(occluded) => {
                    app.send_event(WindowOccluded { window, occluded });
                }
                WindowEvent::DroppedFile(path_buf) => {
                    app.send_event(FileDragAndDrop::DroppedFile { window, path_buf });
                }
                WindowEvent::HoveredFile(path_buf) => {
                    app.send_event(FileDragAndDrop::HoveredFile { window, path_buf });
                }
                WindowEvent::HoveredFileCancelled => {
                    app.send_event(FileDragAndDrop::HoveredFileCanceled { window });
                }
                WindowEvent::Moved(position) => {
                    let position = ivec2(position.x, position.y);
                    win.position.set(position);
                    app.send_event(WindowMoved { window, position });
                }
                WindowEvent::Ime(event) => match event {
                    event::Ime::Preedit(value, cursor) => {
                        app.send_event(Ime::Preedit {
                            window,
                            value,
                            cursor,
                        });
                    }
                    event::Ime::Commit(value) => {
                        app.send_event(Ime::Commit { window, value });
                    }
                    event::Ime::Enabled => {
                        app.send_event(Ime::Enabled { window });
                    }
                    event::Ime::Disabled => {
                        app.send_event(Ime::Disabled { window });
                    }
                },
                WindowEvent::ThemeChanged(theme) => {
                    app.send_event(WindowThemeChanged {
                        window,
                        theme: convert_winit_theme(theme),
                    });
                }
                WindowEvent::Destroyed => {
                    app.send_event(WindowDestroyed { window });
                }
                WindowEvent::RedrawRequested => {
                    run_app_update_if_should(
                        runner_state,
                        app,
                        focused_windows_state,
                        event_loop,
                        create_window,
                        app_exit_event_reader,
                        redraw_event_reader,
                    );
                }
                _ => {}
            }

            let mut windows = app.world.query::<(&mut Window, &mut CachedWindow)>();
            if let Ok((window_component, mut cache)) = windows.get_mut(&mut app.world, window) {
                if window_component.is_changed() {
                    cache.window = window_component.clone();
                }
            }
        }
        Event::DeviceEvent { event, .. } => {
            runner_state.device_event_received = true;
            if let DeviceEvent::MouseMotion { delta: (x, y) } = event {
                let delta = Vec2::new(x as f32, y as f32);
                app.send_event(MouseMotion { delta });
            }
        }
        Event::Suspended => {
            app.send_event(ApplicationLifetime::Suspended);
            // Mark the state as `WillSuspend`. This will let the schedule run one last time
            // before actually suspending to let the application react
            runner_state.active = ActiveState::WillSuspend;
        }
        Event::Resumed => {
            #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
            {
                if runner_state.active == ActiveState::NotYetStarted {
                    create_windows(event_loop, create_window.get_mut(&mut app.world));
                    create_window.apply(&mut app.world);
                }
            }

            match runner_state.active {
                ActiveState::NotYetStarted => app.send_event(ApplicationLifetime::Started),
                _ => app.send_event(ApplicationLifetime::Resumed),
            }
            runner_state.active = ActiveState::Active;
            runner_state.redraw_requested = true;
            #[cfg(target_os = "android")]
            {
                // Get windows that are cached but without raw handles. Those window were already created, but got their
                // handle wrapper removed when the app was suspended.
                let mut query = app
                        .world
                        .query_filtered::<(Entity, &Window), (With<CachedWindow>, Without<bevy_window::RawHandleWrapper>)>();
                if let Ok((entity, window)) = query.get_single(&app.world) {
                    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
                    let window = window.clone();

                    let (
                        ..,
                        mut winit_windows,
                        mut adapters,
                        mut handlers,
                        accessibility_requested,
                    ) = create_window.get_mut(&mut app.world);

                    let winit_window = winit_windows.create_window(
                        event_loop,
                        entity,
                        &window,
                        &mut adapters,
                        &mut handlers,
                        &accessibility_requested,
                    );

                    let wrapper = RawHandleWrapper {
                        window_handle: winit_window.window_handle().unwrap().as_raw(),
                        display_handle: winit_window.display_handle().unwrap().as_raw(),
                    };

                    app.world.entity_mut(entity).insert(wrapper);
                }
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
        _ => (),
    }
}

fn run_app_update_if_should(
    runner_state: &mut WinitAppRunnerState,
    app: &mut App,
    focused_windows_state: &mut SystemState<(Res<WinitSettings>, Query<&Window>)>,
    event_loop: &EventLoopWindowTarget<()>,
    create_window: &mut SystemState<CreateWindowParams<Added<Window>>>,
    app_exit_event_reader: &mut ManualEventReader<AppExit>,
    redraw_event_reader: &mut ManualEventReader<RequestRedraw>,
) {
    runner_state.reset_on_update();

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

    if app.plugins_state() == PluginsState::Cleaned {
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
    create_windows(event_loop, create_window.get_mut(&mut app.world));
    create_window.apply(&mut app.world);
}

fn react_to_resize(
    win: &mut Mut<'_, Window>,
    size: winit::dpi::PhysicalSize<u32>,
    window_resized: &mut EventWriter<WindowResized>,
    window: Entity,
) {
    win.resolution
        .set_physical_resolution(size.width, size.height);

    window_resized.send(WindowResized {
        window,
        width: win.width(),
        height: win.height(),
    });
}
