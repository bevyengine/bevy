use approx::relative_eq;
use bevy_app::{App, AppExit, PluginsState};
use bevy_ecs::{
    change_detection::{DetectChanges, NonSendMut, Res},
    entity::Entity,
    event::{EventCursor, EventWriter},
    prelude::*,
    system::SystemState,
    world::FromWorld,
};
use bevy_input::{
    gestures::*,
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
};
use bevy_log::{error, trace, warn};
use bevy_math::{ivec2, DVec2, Vec2};
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;
use bevy_utils::{HashMap, Instant};
use core::marker::PhantomData;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use bevy_window::{
    AppLifecycle, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime, RequestRedraw,
    Window, WindowBackendScaleFactorChanged, WindowCloseRequested, WindowDestroyed,
    WindowEvent as BevyWindowEvent, WindowFocused, WindowMoved, WindowOccluded, WindowResized,
    WindowScaleFactorChanged, WindowThemeChanged,
};
#[cfg(target_os = "android")]
use bevy_window::{PrimaryWindow, RawHandleWrapper};

use crate::{
    accessibility::AccessKitAdapters,
    converters, create_windows,
    system::{create_monitors, CachedWindow},
    AppSendEvent, CreateMonitorParams, CreateWindowParams, EventLoopProxyWrapper, UpdateMode,
    WinitSettings, WinitWindows,
};

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
struct WinitAppRunnerState<T: Event> {
    /// The running app.
    app: App,
    /// Exit value once the loop is finished.
    app_exit: Option<AppExit>,
    /// Current update mode of the app.
    update_mode: UpdateMode,
    /// Is `true` if a new [`WindowEvent`] event has been received since the last update.
    window_event_received: bool,
    /// Is `true` if a new [`DeviceEvent`] event has been received since the last update.
    device_event_received: bool,
    /// Is `true` if a new `T` event has been received since the last update.
    user_event_received: bool,
    /// Is `true` if the app has requested a redraw since the last update.
    redraw_requested: bool,
    /// Is `true` if the app has already updated since the last redraw.
    ran_update_since_last_redraw: bool,
    /// Is `true` if enough time has elapsed since `last_update` to run another update.
    wait_elapsed: bool,
    /// Number of "forced" updates to trigger on application start
    startup_forced_updates: u32,

    /// Current app lifecycle state.
    lifecycle: AppLifecycle,
    /// The previous app lifecycle state.
    previous_lifecycle: AppLifecycle,
    /// Bevy window events to send
    bevy_window_events: Vec<bevy_window::WindowEvent>,
    _marker: PhantomData<T>,

    event_writer_system_state: SystemState<(
        EventWriter<'static, WindowResized>,
        EventWriter<'static, WindowBackendScaleFactorChanged>,
        EventWriter<'static, WindowScaleFactorChanged>,
        NonSend<'static, WinitWindows>,
        Query<'static, 'static, (&'static mut Window, &'static mut CachedWindow)>,
        NonSendMut<'static, AccessKitAdapters>,
    )>,
}

impl<T: Event> WinitAppRunnerState<T> {
    fn new(mut app: App) -> Self {
        app.add_event::<T>().init_resource::<CustomCursorCache>();

        let event_writer_system_state: SystemState<(
            EventWriter<WindowResized>,
            EventWriter<WindowBackendScaleFactorChanged>,
            EventWriter<WindowScaleFactorChanged>,
            NonSend<WinitWindows>,
            Query<(&mut Window, &mut CachedWindow)>,
            NonSendMut<AccessKitAdapters>,
        )> = SystemState::new(app.world_mut());

        Self {
            app,
            lifecycle: AppLifecycle::Idle,
            previous_lifecycle: AppLifecycle::Idle,
            app_exit: None,
            update_mode: UpdateMode::Continuous,
            window_event_received: false,
            device_event_received: false,
            user_event_received: false,
            redraw_requested: false,
            ran_update_since_last_redraw: false,
            wait_elapsed: false,
            // 3 seems to be enough, 5 is a safe margin
            startup_forced_updates: 5,
            bevy_window_events: Vec::new(),
            _marker: PhantomData,
            event_writer_system_state,
        }
    }

    fn reset_on_update(&mut self) {
        self.window_event_received = false;
        self.device_event_received = false;
        self.user_event_received = false;
    }

    fn world(&self) -> &World {
        self.app.world()
    }

    fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }
}

/// Identifiers for custom cursors used in caching.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CustomCursorCacheKey {
    /// u64 is used instead of `AssetId`, because `bevy_asset` can't be imported here.
    AssetIndex(u64),
    /// u128 is used instead of `AssetId`, because `bevy_asset` can't be imported here.
    AssetUuid(u128),
    /// A URL to a cursor.
    Url(String),
}

/// Caches custom cursors. On many platforms, creating custom cursors is expensive, especially on
/// the web.
#[derive(Debug, Clone, Default, Resource)]
pub struct CustomCursorCache(pub HashMap<CustomCursorCacheKey, winit::window::CustomCursor>);

/// A source for a cursor. Is created in `bevy_render` and consumed by the winit event loop.
#[derive(Debug)]
pub enum CursorSource {
    /// A custom cursor was identified to be cached, no reason to recreate it.
    CustomCached(CustomCursorCacheKey),
    /// A custom cursor was not cached, so it needs to be created by the winit event loop.
    Custom((CustomCursorCacheKey, winit::window::CustomCursorSource)),
    /// A system cursor was requested.
    System(winit::window::CursorIcon),
}

/// Component that indicates what cursor should be used for a window. Inserted
/// automatically after changing `CursorIcon` and consumed by the winit event
/// loop.
#[derive(Component, Debug)]
pub struct PendingCursor(pub Option<CursorSource>);

impl<T: Event> ApplicationHandler<T> for WinitAppRunnerState<T> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if event_loop.exiting() {
            return;
        }

        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();

        if self.app.plugins_state() != PluginsState::Cleaned {
            if self.app.plugins_state() != PluginsState::Ready {
                #[cfg(not(target_arch = "wasm32"))]
                tick_global_task_pools_on_main_thread();
            } else {
                self.app.finish();
                self.app.cleanup();
            }
            self.redraw_requested = true;
        }

        self.wait_elapsed = match cause {
            StartCause::WaitCancelled {
                requested_resume: Some(resume),
                ..
            } => {
                // If the resume time is not after now, it means that at least the wait timeout
                // has elapsed.
                resume <= Instant::now()
            }
            _ => true,
        };
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Mark the state as `WillResume`. This will let the schedule run one extra time
        // when actually resuming the app
        self.lifecycle = AppLifecycle::WillResume;
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: T) {
        self.user_event_received = true;

        self.world_mut().send_event(event);
        self.redraw_requested = true;
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.window_event_received = true;

        let (
            mut window_resized,
            mut window_backend_scale_factor_changed,
            mut window_scale_factor_changed,
            winit_windows,
            mut windows,
            mut access_kit_adapters,
        ) = self.event_writer_system_state.get_mut(self.app.world_mut());

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
        if let Some(adapter) = access_kit_adapters.get_mut(&window) {
            if let Some(winit_window) = winit_windows.get_window(window) {
                adapter.process_event(winit_window, &event);
            }
        }

        match event {
            WindowEvent::Resized(size) => {
                react_to_resize(window, &mut win, size, &mut window_resized);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                react_to_scale_factor_change(
                    window,
                    &mut win,
                    scale_factor,
                    &mut window_backend_scale_factor_changed,
                    &mut window_scale_factor_changed,
                );
            }
            WindowEvent::CloseRequested => self
                .bevy_window_events
                .send(WindowCloseRequested { window }),
            WindowEvent::KeyboardInput {
                ref event,
                // On some platforms, winit sends "synthetic" key press events when the window
                // gains or loses focus. These should not be handled, so we only process key
                // events if they are not synthetic key presses.
                is_synthetic: false,
                ..
            } => {
                self.bevy_window_events
                    .send(converters::convert_keyboard_input(event, window));
            }
            WindowEvent::CursorMoved { position, .. } => {
                let physical_position = DVec2::new(position.x, position.y);

                let last_position = win.physical_cursor_position();
                let delta = last_position.map(|last_pos| {
                    (physical_position.as_vec2() - last_pos) / win.resolution.scale_factor()
                });

                win.set_physical_cursor_position(Some(physical_position));
                let position = (physical_position / win.resolution.scale_factor() as f64).as_vec2();
                self.bevy_window_events.send(CursorMoved {
                    window,
                    position,
                    delta,
                });
            }
            WindowEvent::CursorEntered { .. } => {
                self.bevy_window_events.send(CursorEntered { window });
            }
            WindowEvent::CursorLeft { .. } => {
                win.set_physical_cursor_position(None);
                self.bevy_window_events.send(CursorLeft { window });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.bevy_window_events.send(MouseButtonInput {
                    button: converters::convert_mouse_button(button),
                    state: converters::convert_element_state(state),
                    window,
                });
            }
            WindowEvent::PinchGesture { delta, .. } => {
                self.bevy_window_events.send(PinchGesture(delta as f32));
            }
            WindowEvent::RotationGesture { delta, .. } => {
                self.bevy_window_events.send(RotationGesture(delta));
            }
            WindowEvent::DoubleTapGesture { .. } => {
                self.bevy_window_events.send(DoubleTapGesture);
            }
            WindowEvent::PanGesture { delta, .. } => {
                self.bevy_window_events.send(PanGesture(Vec2 {
                    x: delta.x,
                    y: delta.y,
                }));
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                event::MouseScrollDelta::LineDelta(x, y) => {
                    self.bevy_window_events.send(MouseWheel {
                        unit: MouseScrollUnit::Line,
                        x,
                        y,
                        window,
                    });
                }
                event::MouseScrollDelta::PixelDelta(p) => {
                    self.bevy_window_events.send(MouseWheel {
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
                self.bevy_window_events
                    .send(converters::convert_touch_input(touch, location, window));
            }
            WindowEvent::Focused(focused) => {
                win.focused = focused;
                self.bevy_window_events
                    .send(WindowFocused { window, focused });
            }
            WindowEvent::Occluded(occluded) => {
                self.bevy_window_events
                    .send(WindowOccluded { window, occluded });
            }
            WindowEvent::DroppedFile(path_buf) => {
                self.bevy_window_events
                    .send(FileDragAndDrop::DroppedFile { window, path_buf });
            }
            WindowEvent::HoveredFile(path_buf) => {
                self.bevy_window_events
                    .send(FileDragAndDrop::HoveredFile { window, path_buf });
            }
            WindowEvent::HoveredFileCancelled => {
                self.bevy_window_events
                    .send(FileDragAndDrop::HoveredFileCanceled { window });
            }
            WindowEvent::Moved(position) => {
                let position = ivec2(position.x, position.y);
                win.position.set(position);
                self.bevy_window_events
                    .send(WindowMoved { window, position });
            }
            WindowEvent::Ime(event) => match event {
                event::Ime::Preedit(value, cursor) => {
                    self.bevy_window_events.send(Ime::Preedit {
                        window,
                        value,
                        cursor,
                    });
                }
                event::Ime::Commit(value) => {
                    self.bevy_window_events.send(Ime::Commit { window, value });
                }
                event::Ime::Enabled => {
                    self.bevy_window_events.send(Ime::Enabled { window });
                }
                event::Ime::Disabled => {
                    self.bevy_window_events.send(Ime::Disabled { window });
                }
            },
            WindowEvent::ThemeChanged(theme) => {
                self.bevy_window_events.send(WindowThemeChanged {
                    window,
                    theme: converters::convert_winit_theme(theme),
                });
            }
            WindowEvent::Destroyed => {
                self.bevy_window_events.send(WindowDestroyed { window });
            }
            WindowEvent::RedrawRequested => {
                self.ran_update_since_last_redraw = false;
            }
            _ => {}
        }

        let mut windows = self.world_mut().query::<(&mut Window, &mut CachedWindow)>();
        if let Ok((window_component, mut cache)) = windows.get_mut(self.world_mut(), window) {
            if window_component.is_changed() {
                cache.window = window_component.clone();
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.device_event_received = true;

        if let DeviceEvent::MouseMotion { delta: (x, y) } = event {
            let delta = Vec2::new(x as f32, y as f32);
            self.bevy_window_events.send(MouseMotion { delta });
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut create_monitor = SystemState::<CreateMonitorParams>::from_world(self.world_mut());
        // create any new windows
        // (even if app did not update, some may have been created by plugin setup)
        let mut create_window =
            SystemState::<CreateWindowParams<Added<Window>>>::from_world(self.world_mut());
        create_monitors(event_loop, create_monitor.get_mut(self.world_mut()));
        create_monitor.apply(self.world_mut());
        create_windows(event_loop, create_window.get_mut(self.world_mut()));
        create_window.apply(self.world_mut());

        let mut redraw_event_reader = EventCursor::<RequestRedraw>::default();

        let mut focused_windows_state: SystemState<(Res<WinitSettings>, Query<(Entity, &Window)>)> =
            SystemState::new(self.world_mut());

        if let Some(app_redraw_events) = self.world().get_resource::<Events<RequestRedraw>>() {
            if redraw_event_reader.read(app_redraw_events).last().is_some() {
                self.redraw_requested = true;
            }
        }

        let (config, windows) = focused_windows_state.get(self.world());
        let focused = windows.iter().any(|(_, window)| window.focused);

        let mut update_mode = config.update_mode(focused);
        let mut should_update = self.should_update(update_mode);

        if self.startup_forced_updates > 0 {
            self.startup_forced_updates -= 1;
            // Ensure that an update is triggered on the first iterations for app initialization
            should_update = true;
        }

        if self.lifecycle == AppLifecycle::WillSuspend {
            self.lifecycle = AppLifecycle::Suspended;
            // Trigger one last update to enter the suspended state
            should_update = true;

            #[cfg(target_os = "android")]
            {
                // Remove the `RawHandleWrapper` from the primary window.
                // This will trigger the surface destruction.
                let mut query = self
                    .world_mut()
                    .query_filtered::<Entity, With<PrimaryWindow>>();
                let entity = query.single(&self.world());
                self.world_mut()
                    .entity_mut(entity)
                    .remove::<RawHandleWrapper>();
            }
        }

        if self.lifecycle == AppLifecycle::WillResume {
            self.lifecycle = AppLifecycle::Running;
            // Trigger the update to enter the running state
            should_update = true;
            // Trigger the next redraw to refresh the screen immediately
            self.redraw_requested = true;

            #[cfg(target_os = "android")]
            {
                // Get windows that are cached but without raw handles. Those window were already created, but got their
                // handle wrapper removed when the app was suspended.
                let mut query = self.world_mut()
                    .query_filtered::<(Entity, &Window), (With<CachedWindow>, Without<bevy_window::RawHandleWrapper>)>();
                if let Ok((entity, window)) = query.get_single(&self.world()) {
                    let window = window.clone();

                    let mut create_window =
                        SystemState::<CreateWindowParams>::from_world(self.world_mut());

                    let (
                        ..,
                        mut winit_windows,
                        mut adapters,
                        mut handlers,
                        accessibility_requested,
                        monitors,
                    ) = create_window.get_mut(self.world_mut());

                    let winit_window = winit_windows.create_window(
                        event_loop,
                        entity,
                        &window,
                        &mut adapters,
                        &mut handlers,
                        &accessibility_requested,
                        &monitors,
                    );

                    let wrapper = RawHandleWrapper::new(winit_window).unwrap();

                    self.world_mut().entity_mut(entity).insert(wrapper);
                }
            }
        }

        // Notifies a lifecycle change
        if self.lifecycle != self.previous_lifecycle {
            self.previous_lifecycle = self.lifecycle;
            self.bevy_window_events.send(self.lifecycle);
        }

        // This is recorded before running app.update(), to run the next cycle after a correct timeout.
        // If the cycle takes more than the wait timeout, it will be re-executed immediately.
        let begin_frame_time = Instant::now();

        if should_update {
            let (_, windows) = focused_windows_state.get(self.world());
            // If no windows exist, this will evaluate to `true`.
            let all_invisible = windows.iter().all(|w| !w.1.visible);

            // Not redrawing, but the timeout elapsed.
            //
            // Additional condition for Windows OS.
            // If no windows are visible, redraw calls will never succeed, which results in no app update calls being performed.
            // This is a temporary solution, full solution is mentioned here: https://github.com/bevyengine/bevy/issues/1343#issuecomment-770091684
            if !self.ran_update_since_last_redraw || all_invisible {
                self.run_app_update();
                self.update_cursors(event_loop);
                self.ran_update_since_last_redraw = true;
            } else {
                self.redraw_requested = true;
            }

            // Running the app may have changed the WinitSettings resource, so we have to re-extract it.
            let (config, windows) = focused_windows_state.get(self.world());
            let focused = windows.iter().any(|(_, window)| window.focused);
            update_mode = config.update_mode(focused);
        }

        // The update mode could have been changed, so we need to redraw and force an update
        if update_mode != self.update_mode {
            // Trigger the next redraw since we're changing the update mode
            self.redraw_requested = true;
            // Consider the wait as elapsed since it could have been cancelled by a user event
            self.wait_elapsed = true;

            self.update_mode = update_mode;
        }

        match update_mode {
            UpdateMode::Continuous => {
                // per winit's docs on [Window::is_visible](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.is_visible),
                // we cannot use the visibility to drive rendering on these platforms
                // so we cannot discern whether to beneficially use `Poll` or not?
                cfg_if::cfg_if! {
                    if #[cfg(not(any(
                        target_arch = "wasm32",
                        target_os = "android",
                        target_os = "ios",
                        all(target_os = "linux", any(feature = "x11", feature = "wayland"))
                    )))]
                    {
                        let winit_windows = self.world().non_send_resource::<WinitWindows>();
                        let visible = winit_windows.windows.iter().any(|(_, w)| {
                            w.is_visible().unwrap_or(false)
                        });

                        event_loop.set_control_flow(if visible {
                            ControlFlow::Wait
                        } else {
                            ControlFlow::Poll
                        });
                    }
                    else {
                        event_loop.set_control_flow(ControlFlow::Wait);
                    }
                }

                // Trigger the next redraw to refresh the screen immediately if waiting
                if let ControlFlow::Wait = event_loop.control_flow() {
                    self.redraw_requested = true;
                }
            }
            UpdateMode::Reactive { wait, .. } => {
                // Set the next timeout, starting from the instant before running app.update() to avoid frame delays
                if let Some(next) = begin_frame_time.checked_add(wait) {
                    if self.wait_elapsed {
                        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
                    }
                }
            }
        }

        if self.redraw_requested && self.lifecycle != AppLifecycle::Suspended {
            let winit_windows = self.world().non_send_resource::<WinitWindows>();
            for window in winit_windows.windows.values() {
                window.request_redraw();
            }
            self.redraw_requested = false;
        }

        if let Some(app_exit) = self.app.should_exit() {
            self.app_exit = Some(app_exit);

            event_loop.exit();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Mark the state as `WillSuspend`. This will let the schedule run one last time
        // before actually suspending to let the application react
        self.lifecycle = AppLifecycle::WillSuspend;
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let world = self.world_mut();
        world.clear_all();
    }
}

impl<T: Event> WinitAppRunnerState<T> {
    fn should_update(&self, update_mode: UpdateMode) -> bool {
        let handle_event = match update_mode {
            UpdateMode::Continuous => {
                self.wait_elapsed
                    || self.user_event_received
                    || self.window_event_received
                    || self.device_event_received
            }
            UpdateMode::Reactive {
                react_to_device_events,
                react_to_user_events,
                react_to_window_events,
                ..
            } => {
                self.wait_elapsed
                    || (react_to_device_events && self.device_event_received)
                    || (react_to_user_events && self.user_event_received)
                    || (react_to_window_events && self.window_event_received)
            }
        };

        handle_event && self.lifecycle.is_active()
    }

    fn run_app_update(&mut self) {
        self.reset_on_update();

        self.forward_bevy_events();

        if self.app.plugins_state() == PluginsState::Cleaned {
            self.app.update();
        }
    }

    fn forward_bevy_events(&mut self) {
        let buffered_events = self.bevy_window_events.drain(..).collect::<Vec<_>>();

        if buffered_events.is_empty() {
            return;
        }

        let world = self.world_mut();

        for winit_event in buffered_events.iter() {
            match winit_event.clone() {
                BevyWindowEvent::AppLifecycle(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::CursorEntered(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::CursorLeft(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::CursorMoved(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::FileDragAndDrop(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::Ime(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::RequestRedraw(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowBackendScaleFactorChanged(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowCloseRequested(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowCreated(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowDestroyed(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowFocused(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowMoved(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowOccluded(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowResized(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowScaleFactorChanged(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::WindowThemeChanged(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::MouseButtonInput(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::MouseMotion(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::MouseWheel(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::PinchGesture(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::RotationGesture(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::DoubleTapGesture(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::PanGesture(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::TouchInput(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::KeyboardInput(e) => {
                    world.send_event(e);
                }
                BevyWindowEvent::KeyboardFocusLost(e) => {
                    world.send_event(e);
                }
            }
        }

        world
            .resource_mut::<Events<BevyWindowEvent>>()
            .send_batch(buffered_events);
    }

    fn update_cursors(&mut self, event_loop: &ActiveEventLoop) {
        let mut windows_state: SystemState<(
            NonSendMut<WinitWindows>,
            ResMut<CustomCursorCache>,
            Query<(Entity, &mut PendingCursor), Changed<PendingCursor>>,
        )> = SystemState::new(self.world_mut());
        let (winit_windows, mut cursor_cache, mut windows) =
            windows_state.get_mut(self.world_mut());

        for (entity, mut pending_cursor) in windows.iter_mut() {
            let Some(winit_window) = winit_windows.get_window(entity) else {
                continue;
            };
            let Some(pending_cursor) = pending_cursor.0.take() else {
                continue;
            };

            let final_cursor: winit::window::Cursor = match pending_cursor {
                CursorSource::CustomCached(cache_key) => {
                    let Some(cached_cursor) = cursor_cache.0.get(&cache_key) else {
                        error!("Cursor should have been cached, but was not found");
                        continue;
                    };
                    cached_cursor.clone().into()
                }
                CursorSource::Custom((cache_key, cursor)) => {
                    let custom_cursor = event_loop.create_custom_cursor(cursor);
                    cursor_cache.0.insert(cache_key, custom_cursor.clone());
                    custom_cursor.into()
                }
                CursorSource::System(system_cursor) => system_cursor.into(),
            };
            winit_window.set_cursor(final_cursor);
        }
    }
}

/// The default [`App::runner`] for the [`WinitPlugin`](crate::WinitPlugin) plugin.
///
/// Overriding the app's [runner](bevy_app::App::runner) while using `WinitPlugin` will bypass the
/// `EventLoop`.
pub fn winit_runner<T: Event>(mut app: App) -> AppExit {
    if app.plugins_state() == PluginsState::Ready {
        app.finish();
        app.cleanup();
    }

    let event_loop = app
        .world_mut()
        .remove_non_send_resource::<EventLoop<T>>()
        .unwrap();

    app.world_mut()
        .insert_resource(EventLoopProxyWrapper(event_loop.create_proxy()));

    let mut runner_state = WinitAppRunnerState::new(app);

    trace!("starting winit event loop");
    // TODO(clean): the winit docs mention using `spawn` instead of `run` on Wasm.
    if let Err(err) = event_loop.run_app(&mut runner_state) {
        error!("winit event loop returned an error: {err}");
    }

    // If everything is working correctly then the event loop only exits after it's sent an exit code.
    runner_state.app_exit.unwrap_or_else(|| {
        error!("Failed to receive a app exit code! This is a bug");
        AppExit::error()
    })
}

pub(crate) fn react_to_resize(
    window_entity: Entity,
    window: &mut Mut<'_, Window>,
    size: PhysicalSize<u32>,
    window_resized: &mut EventWriter<WindowResized>,
) {
    window
        .resolution
        .set_physical_resolution(size.width, size.height);

    window_resized.send(WindowResized {
        window: window_entity,
        width: window.width(),
        height: window.height(),
    });
}

pub(crate) fn react_to_scale_factor_change(
    window_entity: Entity,
    window: &mut Mut<'_, Window>,
    scale_factor: f64,
    window_backend_scale_factor_changed: &mut EventWriter<WindowBackendScaleFactorChanged>,
    window_scale_factor_changed: &mut EventWriter<WindowScaleFactorChanged>,
) {
    window.resolution.set_scale_factor(scale_factor as f32);

    window_backend_scale_factor_changed.send(WindowBackendScaleFactorChanged {
        window: window_entity,
        scale_factor,
    });

    let prior_factor = window.resolution.scale_factor();
    let scale_factor_override = window.resolution.scale_factor_override();

    if scale_factor_override.is_none() && !relative_eq!(scale_factor as f32, prior_factor) {
        window_scale_factor_changed.send(WindowScaleFactorChanged {
            window: window_entity,
            scale_factor,
        });
    }
}
