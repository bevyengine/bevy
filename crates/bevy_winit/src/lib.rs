#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
//! `bevy_winit` provides utilities to create and manage windows through [`winit`]
//!
//! The [`WinitPlugin`] is one of the [`DefaultPlugins`]. It registers an [`App`](bevy_app::App)
//! runner that manages the [`App`](bevy_app::App) using an [`EventLoop`](winit::event_loop::EventLoop).
//!
//! [`DefaultPlugins`]: https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html

pub mod accessibility;
mod converters;
mod system;
#[cfg(target_arch = "wasm32")]
mod web_resize;
mod winit_config;
mod winit_windows;

use accessibility::AccessibilityPlugin;
pub use runner::*;
use system::{changed_windows, create_windows, despawn_windows};
#[cfg(target_arch = "wasm32")]
use web_resize::{CanvasParentResizeEventChannel, CanvasParentResizePlugin};
pub use winit_config::*;
pub use winit_windows::*;

use winit::event_loop::EventLoopBuilder;
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

#[cfg(not(target_arch = "wasm32"))]
use bevy_app::AppEvent;
use bevy_app::{App, First, Last, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::storage::{ThreadLocalTask, ThreadLocalTaskSendError, ThreadLocalTaskSender};
use bevy_ecs::system::SystemParam;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    touch::TouchInput,
    touchpad::{TouchpadMagnify, TouchpadRotate},
};
use bevy_utils::{synccell::SyncCell, tracing::warn, Instant};
use bevy_window::{
    exit_on_all_closed, ApplicationLifetime, CursorEntered, CursorLeft, CursorMoved,
    FileDragAndDrop, Ime, ReceivedCharacter, WindowBackendScaleFactorChanged, WindowCloseRequested,
    WindowDestroyed, WindowFocused, WindowMoved, WindowResized, WindowScaleFactorChanged,
    WindowThemeChanged,
};

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();

/// A [`Plugin`] that uses `winit` to create and manage windows, and receive window and input
/// events.
///
/// This plugin will add systems and resources that sync with the `winit` backend and also replace
/// the exising [`App`] runner with one that constructs an [event loop] to receive window and input
/// events from the OS.
///
/// [event loop]: winit::event_loop::EventLoop
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
        // setup event loop
        let mut event_loop_builder = EventLoopBuilder::<AppEvent>::with_user_event();

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

        let event_loop = crate::EventLoop::new(event_loop_builder.build());

        #[cfg(not(target_arch = "wasm32"))]
        app.init_resource::<WinitWindowEntityMap>();

        // setup app
        app.init_non_send_resource::<WinitWindows>()
            .init_resource::<WinitSettings>()
            .set_runner(winit_runner)
            .add_systems(
                Last,
                (
                    // `exit_on_all_closed` seemingly conflicts with `changed_windows`
                    // but does not actually access any data that would alias (only metadata)
                    changed_windows.ambiguous_with(exit_on_all_closed),
                    despawn_windows,
                    create_windows::<AppEvent>,
                )
                    // apply all changes before despawning windows for consistent event ordering
                    .chain(),
            );

        // TODO: schedule after TimeSystem
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(First, flush_winit_events::<AppEvent>);

        app.add_plugins(AccessibilityPlugin);

        #[cfg(target_arch = "wasm32")]
        app.add_plugins(CanvasParentResizePlugin);

        // iOS, macOS, and Android don't like it if you create windows before the
        // event loop is initialized.
        //
        // See:
        // - https://github.com/rust-windowing/winit/blob/master/README.md#macos
        // - https://github.com/rust-windowing/winit/blob/master/README.md#ios
        #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
        {
            // TODO: rework app setup
            let sender = crate::EventLoopProxy::new(event_loop.create_proxy());
            let world = app.sub_apps.main.world_mut();

            app.tls.insert_channel(world, sender);

            app.tls
                .lock()
                .insert_resource(crate::EventLoopWindowTarget::new(&event_loop));

            // Otherwise, create a window before `bevy_render` initializes
            // the renderer, so that we have a surface to use as a hint.
            // This improves compatibility with wgpu backends, especially WASM/WebGL2.
            let mut create_windows = IntoSystem::into_system(create_windows::<AppEvent>);
            create_windows.initialize(world);
            create_windows.run((), world);
            create_windows.apply_deferred(world);

            app.tls
                .lock()
                .remove_resource::<crate::EventLoopWindowTarget<AppEvent>>();

            app.tls.remove_channel(world);
        }

        app.insert_non_send_resource(event_loop);
    }
}

pub(crate) fn run<F, T>(event_loop: winit::event_loop::EventLoop<T>, event_handler: F) -> !
where
    F: 'static
        + FnMut(
            winit::event::Event<'_, T>,
            &winit::event_loop::EventLoopWindowTarget<T>,
            &mut winit::event_loop::ControlFlow,
        ),
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
pub(crate) fn run_return<F, T>(event_loop: &mut winit::event_loop::EventLoop<T>, event_handler: F)
where
    F: FnMut(
        winit::event::Event<'_, T>,
        &winit::event_loop::EventLoopWindowTarget<T>,
        &mut winit::event_loop::ControlFlow,
    ),
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
pub(crate) fn run_return<F, T>(_event_loop: &mut winit::event_loop::EventLoop<T>, _event_handler: F)
where
    F: FnMut(
        winit::event::Event<'_, T>,
        &winit::event_loop::EventLoopWindowTarget<T>,
        &mut winit::event_loop::ControlFlow,
    ),
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

/// Persistent state that is used to run the [`App`] according to the current [`UpdateMode`].
struct WinitAppRunnerState {
    /// Current active state of the app.
    active: ActiveState,
    /// Is `true` if active state just went from `NotYetStarted` to `Active`.
    just_started: bool,
    /// Is `true` if a new [`WindowEvent`](winit::event::WindowEvent) has been received since the
    /// last update.
    window_event_received: bool,
    /// Is `true` if a new [`DeviceEvent`](winit::event::DeviceEvent) has been received.
    device_event_received: bool,
    /// Is `true` if the app has requested a redraw.
    redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update`.
    wait_elapsed: bool,
    /// The time the most recent update started.
    last_update: Instant,
    /// The time the next update is scheduled to start.
    scheduled_update: Option<Instant>,
}

#[derive(PartialEq, Eq)]
pub(crate) enum ActiveState {
    NotYetStarted,
    Active,
    Suspended,
    WillSuspend,
}

impl ActiveState {
    #[inline]
    pub(crate) fn should_run(&self) -> bool {
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
            just_started: false,
            window_event_received: false,
            device_event_received: false,
            redraw_requested: false,
            wait_elapsed: false,
            last_update: Instant::now(),
            scheduled_update: None,
        }
    }
}

#[derive(ThreadLocalResource, Deref, DerefMut)]
pub(crate) struct EventLoop<T: 'static>(winit::event_loop::EventLoop<T>);

impl<T> EventLoop<T> {
    pub fn new(value: winit::event_loop::EventLoop<T>) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> winit::event_loop::EventLoop<T> {
        self.0
    }
}

/// [`EventLoopWindowTarget`] that systems can access through [`ThreadLocal`].
///
/// [`EventLoopWindowTarget`]: winit::event_loop::EventLoopWindowTarget
//
// SAFETY: This type cannot be made `pub`. If it was `pub`, user code could overwrite one whose
// pointer is valid with one whose pointer is invalid, which would break safety invariants.
#[derive(ThreadLocalResource, Deref)]
pub(crate) struct EventLoopWindowTarget<T: 'static> {
    ptr: *const winit::event_loop::EventLoopWindowTarget<T>,
}

impl<T: 'static> EventLoopWindowTarget<T> {
    pub(crate) fn new(target: &winit::event_loop::EventLoopWindowTarget<T>) -> Self {
        Self {
            ptr: target as *const _,
        }
    }

    /// Returns a reference to the [`EventLoopWindowTarget`].
    ///
    /// # Safety
    ///
    /// The target pointer must be valid. That means the [`EventLoop`] used to create the target
    /// must still exist and must not have moved.
    ///
    /// [`EventLoopWindowTarget`]: winit::event_loop::EventLoopWindowTarget
    /// [`EventLoop`]: winit::event_loop::EventLoop
    pub unsafe fn get(&self) -> &'_ winit::event_loop::EventLoopWindowTarget<T> {
        &*self.ptr
    }
}

/// [`EventLoopProxy`](winit::event_loop::EventLoopProxy) wrapped in a [`SyncCell`].
/// Allows systems to wake the [`winit`] event loop from any thread.
#[derive(Resource, Deref, DerefMut)]
pub struct EventLoopProxy<T: 'static>(pub SyncCell<winit::event_loop::EventLoopProxy<T>>);

impl<T> EventLoopProxy<T> {
    pub(crate) fn new(value: winit::event_loop::EventLoopProxy<T>) -> Self {
        Self(SyncCell::new(value))
    }
}

impl ThreadLocalTaskSender for crate::EventLoopProxy<AppEvent> {
    fn send_task(
        &mut self,
        task: ThreadLocalTask,
    ) -> Result<(), ThreadLocalTaskSendError<ThreadLocalTask>> {
        self.0
            .get()
            .send_event(AppEvent::Task(task))
            .map_err(|error| {
                let AppEvent::Task(task) = error.0 else {
                    unreachable!()
                };
                ThreadLocalTaskSendError(task)
            })
    }
}

#[cfg(target_arch = "wasm32")]
mod runner {
    use crate::{
        accessibility::{AccessKitAdapters, WinitActionHandlers},
        converters, run, run_return,
        system::CachedWindow,
        ActiveState, UpdateMode, WindowAndInputEventWriters, WinitAppRunnerState, WinitSettings,
        WinitWindows,
    };

    use bevy_a11y::AccessibilityRequested;
    use bevy_app::{App, AppEvent, AppExit, PluginsState};
    use bevy_ecs::{event::ManualEventReader, prelude::*, system::SystemState};
    use bevy_input::{
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
        touchpad::{TouchpadMagnify, TouchpadRotate},
    };
    use bevy_math::{ivec2, DVec2, Vec2};
    use bevy_utils::{
        tracing::{trace, warn},
        Duration, Instant,
    };
    use bevy_window::{
        ApplicationLifetime, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime,
        ReceivedCharacter, RequestRedraw, Window, WindowBackendScaleFactorChanged,
        WindowCloseRequested, WindowDestroyed, WindowFocused, WindowMoved, WindowResized,
        WindowScaleFactorChanged, WindowThemeChanged,
    };
    #[cfg(target_os = "android")]
    use bevy_window::{PrimaryWindow, RawHandleWrapper};

    use winit::{event::StartCause, event_loop::ControlFlow};

    /// The default [`App::runner`] for the [`WinitPlugin`](super::WinitPlugin).
    pub(crate) fn winit_runner(mut app: App) {
        let return_from_run = app.world().resource::<WinitSettings>().return_from_run;
        let mut event_loop = app
            .tls
            .lock()
            .remove_resource::<crate::EventLoop<AppEvent>>()
            .unwrap()
            .into_inner();
        let event_loop_proxy = event_loop.create_proxy();

        // insert app -> winit channel
        //
        // This is done here because it's the only chance to insert the TLS channel resource to the
        // rendering sub-app before it's moved to another thread.
        // TODO: rework app setup
        app.sub_apps.iter_mut().for_each(|sub_app| {
            let sender = crate::EventLoopProxy::new(event_loop_proxy.clone());
            app.tls.insert_channel(sub_app.world_mut(), sender);
        });

        let mut runner_state = WinitAppRunnerState::default();

        // prepare structures to access data in the world
        let mut event_writer_system_state: SystemState<(
            WindowAndInputEventWriters,
            Query<(&mut Window, &mut CachedWindow)>,
        )> = SystemState::new(app.world_mut());

        #[cfg(target_os = "android")]
        let mut create_window_system_state: SystemState<(
            ResMut<WinitActionHandlers>,
            ResMut<AccessibilityRequested>,
        )> = SystemState::from_world(app.world_mut());

        let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
        let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();

        let mut focused_windows_state: SystemState<(Res<WinitSettings>, Query<&Window>)> =
            SystemState::from_world(app.world_mut());

        let event_cb = move |event: winit::event::Event<AppEvent>,
                             event_loop: &winit::event_loop::EventLoopWindowTarget<AppEvent>,
                             control_flow: &mut ControlFlow| {
            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();

            let plugins_state = app.plugins_state();
            if plugins_state != PluginsState::Cleaned {
                if plugins_state != PluginsState::Ready {
                    #[cfg(not(target_arch = "wasm32"))]
                    tick_global_task_pools_on_main_thread();
                } else {
                    app.finish();
                    app.cleanup();
                }

                if let Some(app_exit_events) = app.world().get_resource::<Events<AppExit>>() {
                    if app_exit_event_reader.read(app_exit_events).last().is_some() {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }

                *control_flow = ControlFlow::Poll;
            }

            app.tls
                .lock()
                .insert_resource(crate::EventLoopWindowTarget::new(event_loop));

            match event {
                winit::event::Event::NewEvents(start_cause) => match start_cause {
                    StartCause::Init => {
                        #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
                        {
                            let mut create_windows =
                                IntoSystem::into_system(create_windows::<AppEvent>);
                            create_windows.initialize(app.world_mut());
                            create_windows.run((), app.world_mut());
                            create_windows.apply_deferred(app.world_mut());
                        }
                    }
                    _ => {
                        if let Some(next) = runner_state.scheduled_update {
                            let now = Instant::now();
                            let remaining =
                                next.checked_duration_since(now).unwrap_or(Duration::ZERO);
                            runner_state.wait_elapsed = remaining.is_zero();
                        }
                    }
                },
                winit::event::Event::WindowEvent {
                    window_id, event, ..
                } => 'window_event: {
                    let tls_guard = app.tls.lock();
                    let winit_windows = tls_guard.resource::<WinitWindows>();
                    let (mut event_writers, mut windows) =
                        event_writer_system_state.get_mut(app.sub_apps.main.world_mut());

                    let Some(window_entity) = winit_windows.get_window_entity(window_id) else {
                        warn!(
                            "Skipped event {:?} for unknown winit Window Id {:?}",
                            event, window_id
                        );
                        break 'window_event;
                    };

                    let Ok((mut window, mut cache)) = windows.get_mut(window_entity) else {
                        warn!(
                            "Window {:?} is missing `Window` component, skipping event {:?}",
                            window_entity, event
                        );
                        break 'window_event;
                    };

                    let access_kit_adapters = tls_guard.resource::<AccessKitAdapters>();

                    // Allow AccessKit to respond to `WindowEvent`s before they reach the engine.
                    if let Some(adapter) = access_kit_adapters.get(&window_entity) {
                        if let Some(window) = winit_windows.get_window(window_entity) {
                            // Somewhat surprisingly, this call has meaningful side effects
                            // See https://github.com/AccessKit/accesskit/issues/300
                            // AccessKit might later need to filter events based on this, but we currently do not.
                            // See https://github.com/bevyengine/bevy/pull/10239#issuecomment-1775572176
                            let _ = adapter.on_event(window, &event);
                        }
                    }

                    runner_state.window_event_received = true;

                    match event {
                        winit::event::WindowEvent::Resized(size) => {
                            window
                                .resolution
                                .set_physical_resolution(size.width, size.height);

                            event_writers.window_resized.send(WindowResized {
                                window: window_entity,
                                width: window.width(),
                                height: window.height(),
                            });
                        }
                        winit::event::WindowEvent::CloseRequested => {
                            event_writers
                                .window_close_requested
                                .send(WindowCloseRequested {
                                    window: window_entity,
                                });
                        }
                        winit::event::WindowEvent::KeyboardInput { ref input, .. } => {
                            event_writers
                                .keyboard_input
                                .send(converters::convert_keyboard_input(input, window_entity));
                        }
                        winit::event::WindowEvent::CursorMoved { position, .. } => {
                            let physical_position = DVec2::new(position.x, position.y);
                            window.set_physical_cursor_position(Some(physical_position));
                            event_writers.cursor_moved.send(CursorMoved {
                                window: window_entity,
                                position: (physical_position / window.resolution.scale_factor())
                                    .as_vec2(),
                            });
                        }
                        winit::event::WindowEvent::CursorEntered { .. } => {
                            event_writers.cursor_entered.send(CursorEntered {
                                window: window_entity,
                            });
                        }
                        winit::event::WindowEvent::CursorLeft { .. } => {
                            window.set_physical_cursor_position(None);
                            event_writers.cursor_left.send(CursorLeft {
                                window: window_entity,
                            });
                        }
                        winit::event::WindowEvent::MouseInput { state, button, .. } => {
                            event_writers.mouse_button_input.send(MouseButtonInput {
                                button: converters::convert_mouse_button(button),
                                state: converters::convert_element_state(state),
                                window: window_entity,
                            });
                        }
                        winit::event::WindowEvent::TouchpadMagnify { delta, .. } => {
                            event_writers
                                .touchpad_magnify_input
                                .send(TouchpadMagnify(delta as f32));
                        }
                        winit::event::WindowEvent::TouchpadRotate { delta, .. } => {
                            event_writers
                                .touchpad_rotate_input
                                .send(TouchpadRotate(delta));
                        }
                        winit::event::WindowEvent::MouseWheel { delta, .. } => match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                event_writers.mouse_wheel_input.send(MouseWheel {
                                    unit: MouseScrollUnit::Line,
                                    x,
                                    y,
                                    window: window_entity,
                                });
                            }
                            winit::event::MouseScrollDelta::PixelDelta(p) => {
                                event_writers.mouse_wheel_input.send(MouseWheel {
                                    unit: MouseScrollUnit::Pixel,
                                    x: p.x as f32,
                                    y: p.y as f32,
                                    window: window_entity,
                                });
                            }
                        },
                        winit::event::WindowEvent::Touch(touch) => {
                            let location =
                                touch.location.to_logical(window.resolution.scale_factor());
                            event_writers
                                .touch_input
                                .send(converters::convert_touch_input(touch, location));
                        }
                        winit::event::WindowEvent::ReceivedCharacter(char) => {
                            event_writers.character_input.send(ReceivedCharacter {
                                window: window_entity,
                                char,
                            });
                        }
                        winit::event::WindowEvent::ScaleFactorChanged {
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
                                // This window is overriding the OS-suggested DPI, so its physical
                                // size should be set based on the overriding value. Its logical
                                // size already incorporates any resize constraints.
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

                            let new_logical_width =
                                (new_inner_size.width as f64 / new_factor) as f32;
                            let new_logical_height =
                                (new_inner_size.height as f64 / new_factor) as f32;
                            if approx::relative_ne!(window.width(), new_logical_width)
                                || approx::relative_ne!(window.height(), new_logical_height)
                            {
                                event_writers.window_resized.send(WindowResized {
                                    window: window_entity,
                                    width: new_logical_width,
                                    height: new_logical_height,
                                });
                            }
                            window.resolution.set_physical_resolution(
                                new_inner_size.width,
                                new_inner_size.height,
                            );
                        }
                        winit::event::WindowEvent::Focused(focused) => {
                            window.focused = focused;
                            event_writers.window_focused.send(WindowFocused {
                                window: window_entity,
                                focused,
                            });
                        }
                        winit::event::WindowEvent::DroppedFile(path_buf) => {
                            event_writers
                                .file_drag_and_drop
                                .send(FileDragAndDrop::DroppedFile {
                                    window: window_entity,
                                    path_buf,
                                });
                        }
                        winit::event::WindowEvent::HoveredFile(path_buf) => {
                            event_writers
                                .file_drag_and_drop
                                .send(FileDragAndDrop::HoveredFile {
                                    window: window_entity,
                                    path_buf,
                                });
                        }
                        winit::event::WindowEvent::HoveredFileCancelled => {
                            event_writers.file_drag_and_drop.send(
                                FileDragAndDrop::HoveredFileCanceled {
                                    window: window_entity,
                                },
                            );
                        }
                        winit::event::WindowEvent::Moved(position) => {
                            let position = ivec2(position.x, position.y);
                            window.position.set(position);
                            event_writers.window_moved.send(WindowMoved {
                                entity: window_entity,
                                position,
                            });
                        }
                        winit::event::WindowEvent::Ime(event) => match event {
                            winit::event::Ime::Preedit(value, cursor) => {
                                event_writers.ime_input.send(Ime::Preedit {
                                    window: window_entity,
                                    value,
                                    cursor,
                                });
                            }
                            winit::event::Ime::Commit(value) => {
                                event_writers.ime_input.send(Ime::Commit {
                                    window: window_entity,
                                    value,
                                })
                            }
                            winit::event::Ime::Enabled => {
                                event_writers.ime_input.send(Ime::Enabled {
                                    window: window_entity,
                                })
                            }
                            winit::event::Ime::Disabled => {
                                event_writers.ime_input.send(Ime::Disabled {
                                    window: window_entity,
                                })
                            }
                        },
                        winit::event::WindowEvent::ThemeChanged(theme) => {
                            event_writers.window_theme_changed.send(WindowThemeChanged {
                                window: window_entity,
                                theme: converters::convert_winit_theme(theme),
                            });
                        }
                        winit::event::WindowEvent::Destroyed => {
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
                winit::event::Event::DeviceEvent {
                    event: winit::event::DeviceEvent::MouseMotion { delta: (x, y) },
                    ..
                } => {
                    let (mut event_writers, ..) =
                        event_writer_system_state.get_mut(app.world_mut());
                    event_writers.mouse_motion.send(MouseMotion {
                        delta: Vec2::new(x as f32, y as f32),
                    });
                }
                winit::event::Event::Suspended => {
                    let (mut event_writers, ..) =
                        event_writer_system_state.get_mut(app.world_mut());
                    event_writers.lifetime.send(ApplicationLifetime::Suspended);
                    // Mark the state as `WillSuspend` so the application can react to the suspend
                    // before actually suspending.
                    runner_state.active = ActiveState::WillSuspend;
                }
                winit::event::Event::Resumed => {
                    let (mut event_writers, ..) =
                        event_writer_system_state.get_mut(app.world_mut());
                    if runner_state.active == ActiveState::NotYetStarted {
                        event_writers.lifetime.send(ApplicationLifetime::Started);
                    } else {
                        event_writers.lifetime.send(ApplicationLifetime::Resumed);
                    }

                    runner_state.active = ActiveState::Active;
                    #[cfg(target_os = "android")]
                    {
                        let mut query = app
                            .world_mut()
                            .query_filtered::<(Entity, &Window), (With<CachedWindow>, Without<RawHandleWrapper>)>();

                        if let Ok((entity, window)) = query.get_single(app.world()) {
                            // Re-create the window on the backend and link it to its entity.
                            use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
                            let window = window.clone();

                            let (mut handlers, accessibility_requested) =
                                create_window_system_state.get_mut(app.world_mut());

                            let tls = app.tls.lock();

                            let raw_handle_wrapper =
                                tls.resource_scope(|tls, mut winit_windows: Mut<WinitWindows>| {
                                    tls.resource_scope(
                                        |tls, mut adapters: Mut<AccessKitAdapters>| {
                                            let winit_window = winit_windows.create_window(
                                                event_loop,
                                                entity,
                                                &window,
                                                &mut adapters,
                                                &mut handlers,
                                                &accessibility_requested,
                                            );

                                            RawHandleWrapper {
                                                window_handle: winit_window.raw_window_handle(),
                                                display_handle: winit_window.raw_display_handle(),
                                            }
                                        },
                                    )
                                });

                            app.world_mut()
                                .entity_mut(entity)
                                .insert(raw_handle_wrapper);
                        }

                        // Set the control flow to run the handler again immediately.
                        *control_flow = ControlFlow::Poll;
                    }
                }
                winit::event::Event::MainEventsCleared => {
                    if runner_state.active.should_run() {
                        if runner_state.active == ActiveState::WillSuspend {
                            runner_state.active = ActiveState::Suspended;
                            #[cfg(target_os = "android")]
                            {
                                // Android sending this event invalidates the existing
                                // surfaces/windows.
                                //
                                // Remove and drop the `RawHandleWrapper` from the entity so
                                // Android will destroy the surface/window.
                                let mut query = app
                                    .world_mut()
                                    .query_filtered::<Entity, With<PrimaryWindow>>();
                                let entity = query.single(app.world());
                                app.world_mut()
                                    .entity_mut(entity)
                                    .remove::<RawHandleWrapper>();
                            }
                        }

                        let (config, windows) = focused_windows_state.get(app.world());
                        let focused = windows.iter().any(|window| window.focused);
                        let should_update = match config.update_mode(focused) {
                            UpdateMode::Continuous | UpdateMode::Reactive { .. } => {
                                // `Reactive`: In order for `event_handler` to have been called,
                                // either we received a window or raw input event, the `wait`
                                // elapsed, or a redraw was requested. There are no other
                                // conditions, so we can just return `true` here.
                                true
                            }
                            UpdateMode::ReactiveLowPower { .. } => {
                                runner_state.wait_elapsed
                                    || runner_state.redraw_requested
                                    || runner_state.window_event_received
                            }
                        };

                        if should_update {
                            // reset these on each update
                            runner_state.wait_elapsed = false;
                            runner_state.window_event_received = false;
                            runner_state.redraw_requested = false;
                            runner_state.last_update = Instant::now();

                            app.update();

                            // decide when to run the next update
                            let (config, windows) = focused_windows_state.get(app.world());
                            let focused = windows.iter().any(|window| window.focused);
                            match config.update_mode(focused) {
                                UpdateMode::Continuous => *control_flow = ControlFlow::Poll,
                                UpdateMode::Reactive { wait }
                                | UpdateMode::ReactiveLowPower { wait } => {
                                    if let Some(next) = runner_state.last_update.checked_add(wait) {
                                        runner_state.scheduled_update = Some(next);
                                        *control_flow = ControlFlow::WaitUntil(next);
                                    } else {
                                        runner_state.scheduled_update = None;
                                        *control_flow = ControlFlow::Wait;
                                    }
                                }
                            }

                            if let Some(redraw_events) =
                                app.world().get_resource::<Events<RequestRedraw>>()
                            {
                                if redraw_event_reader.read(redraw_events).last().is_some() {
                                    runner_state.redraw_requested = true;
                                    *control_flow = ControlFlow::Poll;
                                }
                            }

                            if runner_state.active = ActiveState::Suspended {
                                // Wait for a `Resume` event.
                                *control_flow = ControlFlow::Wait;
                            }

                            if let Some(exit_events) = app.world().get_resource::<Events<AppExit>>()
                            {
                                if app_exit_event_reader.read(exit_events).last().is_some() {
                                    trace!("exiting app");
                                    *control_flow = ControlFlow::Exit;
                                }
                            }
                        }
                    }
                }
                _ => (),
            }

            app.tls
                .lock()
                .remove_resource::<crate::EventLoopWindowTarget<AppEvent>>();
        };

        trace!("starting winit event loop");
        if return_from_run {
            run_return(&mut event_loop, event_cb);
        } else {
            run(event_loop, event_cb);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod runner {
    use std::collections::VecDeque;
    use std::mem;
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
    use std::sync::mpsc::{
        channel, Receiver, RecvError, RecvTimeoutError, SendError, Sender, TryRecvError,
    };

    use bevy_a11y::AccessibilityRequested;
    use bevy_app::{App, AppEvent, AppExit, PluginsState, SubApps};
    #[cfg(target_os = "android")]
    use bevy_ecs::system::SystemParam;
    use bevy_ecs::{event::ManualEventReader, prelude::*, system::SystemState};
    use bevy_input::{
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
        touchpad::{TouchpadMagnify, TouchpadRotate},
    };
    use bevy_math::{ivec2, DVec2, Vec2};
    use bevy_tasks::tick_global_task_pools_on_main_thread;
    use bevy_utils::{
        default,
        synccell::SyncCell,
        tracing::{trace, warn},
        Duration, Instant,
    };
    use bevy_window::{
        ApplicationLifetime, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime,
        ReceivedCharacter, RequestRedraw, Window, WindowBackendScaleFactorChanged,
        WindowCloseRequested, WindowDestroyed, WindowFocused, WindowMoved, WindowResized,
        WindowScaleFactorChanged, WindowThemeChanged,
    };
    #[cfg(target_os = "android")]
    use bevy_window::{PrimaryWindow, RawHandleWrapper};

    use winit::event_loop::ControlFlow;

    use crate::{
        accessibility::{AccessKitAdapters, WinitActionHandlers},
        converters::{self, convert_event, convert_window_event},
        run, run_return,
        system::CachedWindow,
        ActiveState, UpdateMode, WindowAndInputEventWriters, WinitAppRunnerState, WinitSettings,
        WinitWindowEntityMap, WinitWindows,
    };

    /// Sending half of an [`Event`] channel.
    pub struct WinitEventSender<T: Send + 'static> {
        pub(crate) event_send: Sender<crate::converters::Event<T>>,
        pub(crate) clear_send: Sender<u64>,
        pub(crate) last_event_sent: u64,
    }

    /// Receiving half of an [`Event`] channel.
    #[derive(Resource)]
    pub struct WinitEventReceiver<T: Send + 'static> {
        pub(crate) event_recv: SyncCell<Receiver<crate::converters::Event<T>>>,
        pub(crate) clear_recv: SyncCell<Receiver<u64>>,
        pub(crate) processed: SyncCell<VecDeque<crate::converters::Event<T>>>,
        pub(crate) last_event_processed: u64,
        pub(crate) state: WinitAppRunnerState,
    }

    /// Constructs a new [`WinitEventSender`] and [`WinitEventReceiver`] channel pair.
    pub fn winit_channel<T: Send + 'static>() -> (WinitEventSender<T>, WinitEventReceiver<T>) {
        let (clear_send, clear_recv) = channel();
        let (event_send, event_recv) = channel();
        let processed = VecDeque::new();

        let sender = WinitEventSender {
            clear_send,
            event_send,
            last_event_sent: 0,
        };

        let receiver = WinitEventReceiver {
            event_recv: SyncCell::new(event_recv),
            clear_recv: SyncCell::new(clear_recv),
            processed: SyncCell::new(processed),
            last_event_processed: 0,
            state: default(),
        };

        (sender, receiver)
    }

    impl<T> WinitEventSender<T>
    where
        T: Send + 'static,
    {
        pub(crate) fn send(
            &mut self,
            event: converters::Event<T>,
        ) -> Result<(), SendError<converters::Event<T>>> {
            self.last_event_sent = self.last_event_sent.checked_add(1).unwrap();
            self.event_send.send(event)
        }

        /// Informs the receiver that there is a new batch of events to be read.
        pub(crate) fn send_clear(
            &mut self,
            event: converters::Event<T>,
        ) -> Result<(), SendError<converters::Event<T>>> {
            assert!(matches!(event, converters::Event::MainEventsCleared));
            self.send(event)?;
            self.clear_send.send(self.last_event_sent).unwrap();
            Ok(())
        }
    }

    impl<T> WinitEventReceiver<T>
    where
        T: Send + 'static,
    {
        fn process_event(&mut self, event: converters::Event<T>) {
            match &event {
                converters::Event::WindowEvent { .. } => {
                    self.state.window_event_received = true;
                }
                converters::Event::DeviceEvent { .. } => {
                    self.state.device_event_received = true;
                }
                converters::Event::Suspended => {
                    self.state.active = ActiveState::WillSuspend;
                }
                converters::Event::Resumed => {
                    if self.state.active == ActiveState::NotYetStarted {
                        self.state.just_started = true;
                    } else {
                        self.state.just_started = false;
                    }

                    self.state.active = ActiveState::Active;
                }
                converters::Event::RedrawRequested(_) => {
                    self.state.redraw_requested = true;
                }
                _ => (),
            }
            self.last_event_processed = self.last_event_processed.checked_add(1).unwrap();
            self.processed.get().push_back(event);
        }

        fn process_events_until(&mut self, clear_event: u64) {
            while self.last_event_processed < clear_event {
                let event = self.event_recv.get().try_recv().unwrap();
                self.process_event(event);
            }
        }

        pub(crate) fn recv(&mut self) -> Result<(), RecvError> {
            let rx = self.clear_recv.get();
            let mut event = rx.recv()?;
            while let Ok(n) = rx.try_recv() {
                assert!(n > event);
                event = n;
            }
            self.process_events_until(event);
            Ok(())
        }

        pub(crate) fn try_recv(&mut self) -> Result<(), TryRecvError> {
            let rx = self.clear_recv.get();
            let mut event = rx.try_recv()?;
            while let Ok(n) = rx.try_recv() {
                assert!(n > event);
                event = n;
            }
            self.process_events_until(event);
            Ok(())
        }

        pub(crate) fn recv_timeout(&mut self, timeout: Duration) -> Result<(), RecvTimeoutError> {
            let rx = self.clear_recv.get();
            let mut event = rx.recv_timeout(timeout)?;
            while let Ok(n) = rx.try_recv() {
                assert!(n > event);
                event = n;
            }
            self.process_events_until(event);
            Ok(())
        }
    }

    #[cfg(target_os = "android")]
    #[derive(SystemParam)]
    struct ExtraAndroidParams<'w, 's> {
        resume:
            Query<'w, 's, Entity, (With<Window>, With<CachedWindow>, Without<RawHandleWrapper>)>,
        suspend: Query<'w, 's, Entity, With<PrimaryWindow>>,
        handlers: ResMut<'w, WinitActionHandlers>,
        accessibility_requested: Res<'w, AccessibilityRequested>,
        main_thread: ThreadLocal<'w, 's>,
        commands: Commands<'w, 's>,
    }

    pub(crate) fn flush_winit_events<T: Send + 'static>(
        mut queue: ResMut<WinitEventReceiver<T>>,
        mut windows: Query<(&mut Window, &mut CachedWindow)>,
        mut event_writers: WindowAndInputEventWriters,
        #[cfg(not(target_os = "android"))] map: Res<WinitWindowEntityMap>,
        #[cfg(target_os = "android")] mut map: ResMut<WinitWindowEntityMap>,
        #[cfg(target_os = "android")] mut extra: ExtraAndroidParams,
    ) {
        // TODO: Use system local instead?
        let just_started = queue.state.just_started;

        for event in queue.processed.get().drain(..) {
            match event {
                crate::converters::Event::WindowEvent {
                    window_id, event, ..
                } => {
                    let Some(window_entity) = map.get_window_entity(window_id) else {
                        warn!(
                            "Skipped event {:?} for unknown winit Window Id {:?}",
                            event, window_id
                        );
                        continue;
                    };

                    let Ok((mut window, mut cache)) = windows.get_mut(window_entity) else {
                        warn!(
                            "Window {:?} is missing `Window` component, skipping event {:?}",
                            window_entity, event
                        );
                        continue;
                    };

                    match event {
                        converters::WindowEvent::Resized(size) => {
                            window
                                .resolution
                                .set_physical_resolution(size.width, size.height);

                            event_writers.window_resized.send(WindowResized {
                                window: window_entity,
                                width: window.width(),
                                height: window.height(),
                            });
                        }
                        converters::WindowEvent::CloseRequested => {
                            event_writers
                                .window_close_requested
                                .send(WindowCloseRequested {
                                    window: window_entity,
                                });
                        }
                        converters::WindowEvent::KeyboardInput { ref input, .. } => {
                            event_writers
                                .keyboard_input
                                .send(converters::convert_keyboard_input(input, window_entity));
                        }
                        converters::WindowEvent::CursorMoved { position, .. } => {
                            let physical_position = DVec2::new(position.x, position.y);
                            window.set_physical_cursor_position(Some(physical_position));
                            event_writers.cursor_moved.send(CursorMoved {
                                window: window_entity,
                                position: (physical_position / window.resolution.scale_factor())
                                    .as_vec2(),
                            });
                        }
                        converters::WindowEvent::CursorEntered { .. } => {
                            event_writers.cursor_entered.send(CursorEntered {
                                window: window_entity,
                            });
                        }
                        converters::WindowEvent::CursorLeft { .. } => {
                            window.set_physical_cursor_position(None);
                            event_writers.cursor_left.send(CursorLeft {
                                window: window_entity,
                            });
                        }
                        converters::WindowEvent::MouseInput { state, button, .. } => {
                            event_writers.mouse_button_input.send(MouseButtonInput {
                                button: converters::convert_mouse_button(button),
                                state: converters::convert_element_state(state),
                                window: window_entity,
                            });
                        }
                        converters::WindowEvent::TouchpadMagnify { delta, .. } => {
                            event_writers
                                .touchpad_magnify_input
                                .send(TouchpadMagnify(delta as f32));
                        }
                        converters::WindowEvent::TouchpadRotate { delta, .. } => {
                            event_writers
                                .touchpad_rotate_input
                                .send(TouchpadRotate(delta));
                        }
                        converters::WindowEvent::MouseWheel { delta, .. } => match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                event_writers.mouse_wheel_input.send(MouseWheel {
                                    unit: MouseScrollUnit::Line,
                                    x,
                                    y,
                                    window: window_entity,
                                });
                            }
                            winit::event::MouseScrollDelta::PixelDelta(p) => {
                                event_writers.mouse_wheel_input.send(MouseWheel {
                                    unit: MouseScrollUnit::Pixel,
                                    x: p.x as f32,
                                    y: p.y as f32,
                                    window: window_entity,
                                });
                            }
                        },
                        converters::WindowEvent::Touch(touch) => {
                            let location =
                                touch.location.to_logical(window.resolution.scale_factor());
                            event_writers
                                .touch_input
                                .send(converters::convert_touch_input(touch, location));
                        }
                        converters::WindowEvent::ReceivedCharacter(char) => {
                            event_writers.character_input.send(ReceivedCharacter {
                                window: window_entity,
                                char,
                            });
                        }
                        converters::WindowEvent::ScaleFactorChanged {
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

                            if window.resolution.scale_factor_override().is_none()
                                && approx::relative_ne!(scale_factor, prior_factor)
                            {
                                event_writers.window_scale_factor_changed.send(
                                    WindowScaleFactorChanged {
                                        window: window_entity,
                                        scale_factor,
                                    },
                                );
                            }

                            let new_factor = window.resolution.scale_factor();
                            let new_logical_width =
                                (new_inner_size.width as f64 / new_factor) as f32;
                            let new_logical_height =
                                (new_inner_size.height as f64 / new_factor) as f32;
                            if approx::relative_ne!(window.width(), new_logical_width)
                                || approx::relative_ne!(window.height(), new_logical_height)
                            {
                                event_writers.window_resized.send(WindowResized {
                                    window: window_entity,
                                    width: new_logical_width,
                                    height: new_logical_height,
                                });
                            }
                            window.resolution.set_physical_resolution(
                                new_inner_size.width,
                                new_inner_size.height,
                            );
                        }
                        converters::WindowEvent::Focused(focused) => {
                            window.focused = focused;
                            event_writers.window_focused.send(WindowFocused {
                                window: window_entity,
                                focused,
                            });
                        }
                        converters::WindowEvent::DroppedFile(path_buf) => {
                            event_writers
                                .file_drag_and_drop
                                .send(FileDragAndDrop::DroppedFile {
                                    window: window_entity,
                                    path_buf,
                                });
                        }
                        converters::WindowEvent::HoveredFile(path_buf) => {
                            event_writers
                                .file_drag_and_drop
                                .send(FileDragAndDrop::HoveredFile {
                                    window: window_entity,
                                    path_buf,
                                });
                        }
                        converters::WindowEvent::HoveredFileCancelled => {
                            event_writers.file_drag_and_drop.send(
                                FileDragAndDrop::HoveredFileCanceled {
                                    window: window_entity,
                                },
                            );
                        }
                        converters::WindowEvent::Moved(position) => {
                            let position = ivec2(position.x, position.y);
                            window.position.set(position);
                            event_writers.window_moved.send(WindowMoved {
                                entity: window_entity,
                                position,
                            });
                        }
                        converters::WindowEvent::Ime(event) => match event {
                            winit::event::Ime::Preedit(value, cursor) => {
                                event_writers.ime_input.send(Ime::Preedit {
                                    window: window_entity,
                                    value,
                                    cursor,
                                });
                            }
                            winit::event::Ime::Commit(value) => {
                                event_writers.ime_input.send(Ime::Commit {
                                    window: window_entity,
                                    value,
                                });
                            }
                            winit::event::Ime::Enabled => {
                                event_writers.ime_input.send(Ime::Enabled {
                                    window: window_entity,
                                });
                            }
                            winit::event::Ime::Disabled => {
                                event_writers.ime_input.send(Ime::Disabled {
                                    window: window_entity,
                                });
                            }
                        },
                        converters::WindowEvent::ThemeChanged(theme) => {
                            event_writers.window_theme_changed.send(WindowThemeChanged {
                                window: window_entity,
                                theme: converters::convert_winit_theme(theme),
                            });
                        }
                        converters::WindowEvent::Destroyed => {
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
                crate::converters::Event::DeviceEvent {
                    event: winit::event::DeviceEvent::MouseMotion { delta: (x, y) },
                    ..
                } => {
                    event_writers.mouse_motion.send(MouseMotion {
                        delta: Vec2::new(x as f32, y as f32),
                    });
                }
                crate::converters::Event::Suspended => {
                    event_writers.lifetime.send(ApplicationLifetime::Suspended);
                    #[cfg(target_os = "android")]
                    {
                        // Android sending this event invalidates the existing surfaces/windows.
                        //
                        // Remove and drop the `RawHandleWrapper` from the entity so Android will
                        // destroy the surface/window.
                        if let Ok(entity) = extra.suspend.get_single() {
                            extra.commands.entity(entity).remove::<RawHandleWrapper>();
                        }
                    }
                }
                crate::converters::Event::Resumed => {
                    if just_started {
                        event_writers.lifetime.send(ApplicationLifetime::Started);
                    } else {
                        event_writers.lifetime.send(ApplicationLifetime::Resumed);
                    }

                    #[cfg(target_os = "android")]
                    {
                        if let Ok(entity) = extra.resume.get_single() {
                            use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

                            // Re-create the window on the backend and link it to its entity.
                            let (window, _) = windows.get(entity).unwrap();

                            let raw_handle_wrapper = extra.main_thread.run(|tls| {
                                tls.resource_scope(|tls, mut winit_windows: Mut<WinitWindows>| {
                                    tls.resource_scope(|tls, mut adapters: Mut<AccessKitAdapters>| {
                                        // SAFETY: `bevy_winit` guarantees that this resource can
                                        // only be inserted by its `App` runner and that the stored
                                        // pointer is valid.
                                        let event_loop = unsafe {
                                            tls.resource::<crate::EventLoopWindowTarget<AppEvent>>()
                                                .get()
                                        };

                                        let winit_window = winit_windows.create_window(
                                            event_loop,
                                            entity,
                                            window,
                                            &mut map,
                                            &mut adapters,
                                            &mut extra.handlers,
                                            &extra.accessibility_requested,
                                        );

                                        RawHandleWrapper {
                                            window_handle: winit_window.raw_window_handle(),
                                            display_handle: winit_window.raw_display_handle(),
                                        }
                                    })
                                })
                            });

                            // Re-insert the component.
                            extra.commands.entity(entity).insert(raw_handle_wrapper);
                        }
                    }
                }
                _ => (),
            }
        }
    }

    pub(crate) fn spawn_app_thread(
        mut sub_apps: SubApps,
        event_loop_proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    ) {
        std::thread::Builder::new()
            .name("app".to_string())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
                    let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();
                    let mut focused_windows_state: SystemState<(
                        Res<WinitSettings>,
                        Query<&Window>,
                    )> = SystemState::from_world(sub_apps.main.world_mut());

                    let mut rx = sub_apps
                        .main
                        .world_mut()
                        .remove_resource::<WinitEventReceiver<AppEvent>>()
                        .unwrap();

                    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
                    {
                        let mut create_windows =
                            IntoSystem::into_system(crate::system::create_windows::<AppEvent>);
                        create_windows.initialize(sub_apps.main.world_mut());
                        create_windows.run((), sub_apps.main.world_mut());
                        create_windows.apply_deferred(sub_apps.main.world_mut());
                    }

                    let mut control_flow = ControlFlow::Poll;
                    loop {
                        let now = Instant::now();
                        match control_flow {
                            ControlFlow::Poll => match rx.try_recv() {
                                Ok(_) | Err(TryRecvError::Empty) => {}
                                Err(TryRecvError::Disconnected) => {
                                    trace!("terminating app because event loop disconnected");
                                    return;
                                }
                            },
                            ControlFlow::Wait => match rx.recv() {
                                Ok(_) => {}
                                Err(_) => {
                                    trace!("terminating app because event loop disconnected");
                                    return;
                                }
                            },
                            ControlFlow::WaitUntil(next) => {
                                let timeout =
                                    next.checked_duration_since(now).unwrap_or(Duration::ZERO);
                                rx.state.wait_elapsed = timeout.is_zero();
                                match rx.recv_timeout(timeout) {
                                    Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                                    Err(RecvTimeoutError::Disconnected) => {
                                        trace!("terminating app because event loop disconnected");
                                        return;
                                    }
                                }
                            }
                            ControlFlow::ExitWithCode(_) => {
                                trace!("exiting app");
                                // return sub-apps to the main thread
                                if event_loop_proxy
                                    .send_event(AppEvent::Exit(sub_apps))
                                    .is_err()
                                {
                                    trace!("terminating app because event loop disconnected");
                                }
                                return;
                            }
                        }

                        if rx.state.active.should_run() {
                            if rx.active == ActiveState::WillSuspend {
                                rx.active = ActiveState::Suspended;
                                #[cfg(target_os = "android")]
                                {
                                    // Android sending this event invalidates the existing
                                    // surfaces/windows.
                                    //
                                    // Remove and drop the `RawHandleWrapper` from the entity so
                                    // Android will destroy the surface/window.
                                    let mut query = sub_apps
                                        .main
                                        .world_mut()
                                        .query_filtered::<Entity, With<PrimaryWindow>>();
                                    let entity = query.single(sub_apps.main.world());
                                    sub_apps
                                        .main
                                        .world_mut()
                                        .entity_mut(entity)
                                        .remove::<RawHandleWrapper>();
                                }
                            }

                            let (config, windows) =
                                focused_windows_state.get(sub_apps.main.world());
                            let focused = windows.iter().any(|window| window.focused);
                            let should_update = match config.update_mode(focused) {
                                UpdateMode::Continuous => true,
                                UpdateMode::Reactive { .. } => {
                                    rx.state.wait_elapsed
                                        || rx.state.redraw_requested
                                        || rx.state.window_event_received
                                        || rx.state.device_event_received
                                }
                                UpdateMode::ReactiveLowPower { .. } => {
                                    rx.state.wait_elapsed
                                        || rx.state.redraw_requested
                                        || rx.state.window_event_received
                                }
                            };

                            if should_update {
                                // reset these flags
                                rx.state.wait_elapsed = false;
                                rx.state.redraw_requested = false;
                                rx.state.window_event_received = false;
                                rx.state.device_event_received = false;
                                rx.state.last_update = now;

                                sub_apps.main.world_mut().insert_resource(rx);
                                sub_apps.update();
                                rx = sub_apps
                                    .main
                                    .world_mut()
                                    .remove_resource::<WinitEventReceiver<AppEvent>>()
                                    .unwrap();

                                // decide when to run the next update
                                let (config, windows) =
                                    focused_windows_state.get(sub_apps.main.world());
                                let focused = windows.iter().any(|window| window.focused);
                                match config.update_mode(focused) {
                                    UpdateMode::Continuous => control_flow = ControlFlow::Poll,
                                    UpdateMode::Reactive { wait }
                                    | UpdateMode::ReactiveLowPower { wait } => {
                                        if let Some(next) = rx.state.last_update.checked_add(wait) {
                                            rx.state.scheduled_update = Some(next);
                                            control_flow = ControlFlow::WaitUntil(next);
                                        } else {
                                            rx.state.scheduled_update = None;
                                            control_flow = ControlFlow::Wait;
                                        }
                                    }
                                }

                                if let Some(redraw_events) = sub_apps
                                    .main
                                    .world()
                                    .get_resource::<Events<RequestRedraw>>()
                                {
                                    if redraw_event_reader.read(redraw_events).last().is_some() {
                                        rx.state.redraw_requested = true;
                                        control_flow = ControlFlow::Poll;
                                    }
                                }

                                if rx.state.active = ActiveState::Suspended {
                                    // Wait for a `Resume` event.
                                    control_flow = ControlFlow::Wait;
                                }

                                if let Some(exit_events) =
                                    sub_apps.main.world().get_resource::<Events<AppExit>>()
                                {
                                    if app_exit_event_reader.read(exit_events).last().is_some() {
                                        control_flow = ControlFlow::Exit;
                                    }
                                }
                            }
                        }
                    }
                }));

                if let Some(payload) = result.err() {
                    let _ = event_loop_proxy.send_event(AppEvent::Error(payload));
                }
            })
            .unwrap();
    }

    /// The default [`App::runner`] for the [`WinitPlugin`](super::WinitPlugin).
    pub(crate) fn winit_runner(mut app: App) {
        let return_from_run = app.world().resource::<WinitSettings>().return_from_run;
        let mut event_loop = app
            .tls
            .lock()
            .remove_resource::<crate::EventLoop<AppEvent>>()
            .unwrap()
            .into_inner();
        let event_loop_proxy = event_loop.create_proxy();

        // insert app -> winit channel
        //
        // This is done here because it's the only chance to insert the TLS channel resource to the
        // rendering sub-app before it's moved to another thread.
        // TODO: rework app setup
        app.sub_apps.iter_mut().for_each(|sub_app| {
            let app_send = crate::EventLoopProxy::new(event_loop_proxy.clone());
            app.tls.insert_channel(sub_app.world_mut(), app_send);
        });

        let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();

        let (mut winit_send, winit_recv) = winit_channel::<AppEvent>();
        let mut winit_recv = Some(winit_recv);
        let mut locals = None;

        let mut finished_and_setup_done = app.plugins_state() == PluginsState::Cleaned;

        let event_cb = move |event: winit::event::Event<AppEvent>,
                             event_loop: &winit::event_loop::EventLoopWindowTarget<AppEvent>,
                             control_flow: &mut ControlFlow| {
            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();

            let mut should_start = false;
            if !finished_and_setup_done {
                if app.plugins_state() != PluginsState::Ready {
                    tick_global_task_pools_on_main_thread();
                } else {
                    app.finish();
                    app.cleanup();
                    finished_and_setup_done = true;
                    should_start = true;
                }

                if let Some(app_exit_events) = app.world().get_resource::<Events<AppExit>>() {
                    if app_exit_event_reader.read(app_exit_events).last().is_some() {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }

                *control_flow = ControlFlow::Poll;
            } else {
                // Since the app runs in its own thread, this thread should sleep when
                // it has no events to process.
                *control_flow = ControlFlow::Wait;
            }

            if should_start {
                // split app
                let (mut sub_apps, tls, _) = mem::take(&mut app).into_parts();
                locals = Some(tls);

                // insert winit -> app channel
                let winit_recv = winit_recv.take().unwrap();
                sub_apps.main.world_mut().insert_resource(winit_recv);

                // send sub-apps to separate thread
                spawn_app_thread(sub_apps, event_loop_proxy.clone());
            }

            match event {
                winit::event::Event::WindowEvent { window_id, event } => {
                    if let Some(tls) = locals.as_mut() {
                        let tls_guard = tls.lock();

                        let winit_windows = tls_guard.resource::<WinitWindows>();
                        let access_kit_adapters = tls_guard.resource::<AccessKitAdapters>();

                        // Let AccessKit process the event before it reaches the engine.
                        if let Some(entity) = winit_windows.get_window_entity(window_id) {
                            if let Some(window) = winit_windows.get_window(entity) {
                                if let Some(adapter) = access_kit_adapters.get(&entity) {
                                    // Unlike `on_event` suggests, this call has meaningful side
                                    // effects. AccessKit may eventually filter events here, but
                                    // they currently don't.
                                    // See:
                                    // - https://github.com/AccessKit/accesskit/issues/300
                                    // - https://github.com/bevyengine/bevy/pull/10239#issuecomment-1775572176
                                    let _ = adapter.on_event(window, &event);
                                }
                            }
                        }
                    }

                    match event {
                        winit::event::WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                        } => {
                            if let Some(tls) = locals.as_mut() {
                                // This event requires special handling because writes to `new_inner_size`
                                // must happen here. It can't be written asynchronously.
                                let tls_guard = tls.lock();
                                let winit_windows = tls_guard.resource::<WinitWindows>();
                                if let Some(window) = winit_windows.cached_windows.get(&window_id) {
                                    if let Some(sf_override) =
                                        window.resolution.scale_factor_override()
                                    {
                                        // This window is overriding the OS-suggested DPI, so its physical
                                        // size should be set based on the overriding value. Its logical
                                        // size already incorporates any resize constraints.
                                        *new_inner_size = winit::dpi::LogicalSize::new(
                                            window.width(),
                                            window.height(),
                                        )
                                        .to_physical::<u32>(sf_override);
                                    }
                                }
                            }

                            let _ = winit_send.send(converters::Event::WindowEvent {
                                window_id,
                                event: converters::WindowEvent::ScaleFactorChanged {
                                    scale_factor,
                                    new_inner_size: *new_inner_size,
                                },
                            });
                        }
                        _ => {
                            let _ = winit_send.send(convert_window_event(event));
                        }
                    }
                }
                winit::event::Event::UserEvent(event) => {
                    assert!(finished_and_setup_done);
                    match event {
                        AppEvent::Task(f) => {
                            let tls = locals.as_mut().unwrap();

                            tls.lock()
                                .insert_resource(crate::EventLoopWindowTarget::new(event_loop));

                            {
                                #[cfg(feature = "trace")]
                                let _span = bevy_utils::tracing::info_span!("TLS access").entered();
                                f(&mut tls.lock());
                            }

                            tls.lock()
                                .remove_resource::<crate::EventLoopWindowTarget<AppEvent>>();
                        }
                        AppEvent::Exit(_) => {
                            *control_flow = ControlFlow::Exit;
                        }
                        AppEvent::Error(payload) => {
                            resume_unwind(payload);
                        }
                    }
                }
                winit::event::Event::MainEventsCleared => {
                    let _ = winit_send.send_clear(convert_event(event));
                }
                _ => {
                    let _ = winit_send.send(convert_event(event));
                }
            }
        };

        trace!("starting winit event loop");
        if return_from_run {
            run_return(&mut event_loop, event_cb);
        } else {
            run(event_loop, event_cb);
        }
    }
}
