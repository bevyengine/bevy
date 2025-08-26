use approx::relative_eq;
use bevy_app::{App, AppExit, PluginsState};
use bevy_ecs::{
    change_detection::{DetectChanges, Res},
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
use bevy_log::{trace, warn};
use bevy_math::{ivec2, DVec2, Vec2};
use bevy_platform::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;
use core::marker::PhantomData;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
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
use bevy_window::{CursorOptions, PrimaryWindow, RawHandleWrapper};

use crate::{
    accessibility::ACCESS_KIT_ADAPTERS,
    converters, create_windows,
    system::{create_monitors, CachedWindow, WinitWindowPressedKeys},
    AppSendEvent, CreateMonitorParams, CreateWindowParams, EventLoopProxyWrapper,
    RawWinitWindowEvent, UpdateMode, WinitSettings, WINIT_WINDOWS,
};

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
pub(crate) struct WinitAppRunnerState<T: BufferedEvent> {
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
    /// Raw Winit window events to send
    raw_winit_events: Vec<RawWinitWindowEvent>,
    _marker: PhantomData<T>,

    event_writer_system_state: SystemState<(
        EventWriter<'static, WindowResized>,
        EventWriter<'static, WindowBackendScaleFactorChanged>,
        EventWriter<'static, WindowScaleFactorChanged>,
        Query<
            'static,
            'static,
            (
                &'static mut Window,
                &'static mut CachedWindow,
                &'static mut WinitWindowPressedKeys,
            ),
        >,
    )>,
}

impl<T: BufferedEvent> WinitAppRunnerState<T> {
    fn new(mut app: App) -> Self {
        app.add_event::<T>();

        let event_writer_system_state: SystemState<(
            EventWriter<WindowResized>,
            EventWriter<WindowBackendScaleFactorChanged>,
            EventWriter<WindowScaleFactorChanged>,
            Query<(&mut Window, &mut CachedWindow, &mut WinitWindowPressedKeys)>,
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
            raw_winit_events: Vec::new(),
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

    pub(crate) fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }
}

impl<T: BufferedEvent> ApplicationHandler<T> for WinitAppRunnerState<T> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if event_loop.exiting() {
            return;
        }

        #[cfg(feature = "trace")]
        let _span = tracing::info_span!("winit event_handler").entered();

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

        self.world_mut().write_event(event);
        self.redraw_requested = true;
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.window_event_received = true;

        #[cfg_attr(
            not(target_os = "windows"),
            expect(unused_mut, reason = "only needs to be mut on windows for now")
        )]
        let mut manual_run_redraw_requested = false;

        WINIT_WINDOWS.with_borrow(|winit_windows| {
            ACCESS_KIT_ADAPTERS.with_borrow_mut(|access_kit_adapters| {
                let (
                    mut window_resized,
                    mut window_backend_scale_factor_changed,
                    mut window_scale_factor_changed,
                    mut windows,
                ) = self.event_writer_system_state.get_mut(self.app.world_mut());

                let Some(window) = winit_windows.get_window_entity(window_id) else {
                    warn!("Skipped event {event:?} for unknown winit Window Id {window_id:?}");
                    return;
                };

                let Ok((mut win, _, mut pressed_keys)) = windows.get_mut(window) else {
                    warn!(
                        "Window {window:?} is missing `Window` component, skipping event {event:?}"
                    );
                    return;
                };

                // Store a copy of the event to send to an EventWriter later.
                self.raw_winit_events.push(RawWinitWindowEvent {
                    window_id,
                    event: event.clone(),
                });

                // Allow AccessKit to respond to `WindowEvent`s before they reach
                // the engine.
                if let Some(adapter) = access_kit_adapters.get_mut(&window)
                    && let Some(winit_window) = winit_windows.get_window(window)
                {
                    adapter.process_event(winit_window, &event);
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
                        // gains or loses focus. These are not implemented on every platform, so we ignore
                        // winit's synthetic key pressed and implement the same mechanism ourselves.
                        // (See the `WinitWindowPressedKeys` component)
                        is_synthetic: false,
                        ..
                    } => {
                        let keyboard_input = converters::convert_keyboard_input(event, window);
                        if event.state.is_pressed() {
                            pressed_keys.0.insert(
                                keyboard_input.key_code,
                                keyboard_input.logical_key.clone(),
                            );
                        } else {
                            pressed_keys.0.remove(&keyboard_input.key_code);
                        }
                        self.bevy_window_events.send(keyboard_input);
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

                        // https://github.com/bevyengine/bevy/issues/17488
                        #[cfg(target_os = "windows")]
                        {
                            // Have the startup behavior run in about_to_wait, which prevents issues with
                            // invisible window creation. https://github.com/bevyengine/bevy/issues/18027
                            if self.startup_forced_updates == 0 {
                                manual_run_redraw_requested = true;
                            }
                        }
                    }
                    _ => {}
                }

                let mut windows = self.world_mut().query::<(&mut Window, &mut CachedWindow)>();
                if let Ok((window_component, mut cache)) = windows.get_mut(self.world_mut(), window)
                    && window_component.is_changed()
                {
                    **cache = window_component.clone();
                }
            });
        });

        if manual_run_redraw_requested {
            self.redraw_requested(_event_loop);
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

        // TODO: This is a workaround for https://github.com/bevyengine/bevy/issues/17488
        //       while preserving the iOS fix in https://github.com/bevyengine/bevy/pull/11245
        //       The monitor sync logic likely belongs in monitor event handlers and not here.
        #[cfg(not(target_os = "windows"))]
        self.redraw_requested(event_loop);

        // Have the startup behavior run in about_to_wait, which prevents issues with
        // invisible window creation. https://github.com/bevyengine/bevy/issues/18027
        #[cfg(target_os = "windows")]
        {
            WINIT_WINDOWS.with_borrow(|winit_windows| {
                let headless = winit_windows.windows.is_empty();
                let exiting = self.app_exit.is_some();
                let reactive = matches!(self.update_mode, UpdateMode::Reactive { .. });
                let all_invisible = winit_windows
                    .windows
                    .iter()
                    .all(|(_, w)| !w.is_visible().unwrap_or(false));
                if !exiting
                    && (self.startup_forced_updates > 0
                        || headless
                        || all_invisible
                        || reactive
                        || self.window_event_received)
                {
                    self.redraw_requested(event_loop);
                }
            });
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

impl<T: BufferedEvent> WinitAppRunnerState<T> {
    fn redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        let mut redraw_event_reader = EventCursor::<RequestRedraw>::default();

        let mut focused_windows_state: SystemState<(Res<WinitSettings>, Query<(Entity, &Window)>)> =
            SystemState::new(self.world_mut());

        if let Some(app_redraw_events) = self.world().get_resource::<Events<RequestRedraw>>()
            && redraw_event_reader.read(app_redraw_events).last().is_some()
        {
            self.redraw_requested = true;
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
            self.ran_update_since_last_redraw = false;

            #[cfg(target_os = "android")]
            {
                // Remove the `RawHandleWrapper` from the primary window.
                // This will trigger the surface destruction.
                let mut query = self
                    .world_mut()
                    .query_filtered::<Entity, With<PrimaryWindow>>();
                let entity = query.single(&self.world()).unwrap();
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
                    .query_filtered::<(Entity, &Window, &CursorOptions), (With<CachedWindow>, Without<RawHandleWrapper>)>();
                if let Ok((entity, window, cursor_options)) = query.single(&self.world()) {
                    let window = window.clone();
                    let cursor_options = cursor_options.clone();

                    WINIT_WINDOWS.with_borrow_mut(|winit_windows| {
                        ACCESS_KIT_ADAPTERS.with_borrow_mut(|adapters| {
                            let mut create_window =
                                SystemState::<CreateWindowParams>::from_world(self.world_mut());

                            let (.., mut handlers, accessibility_requested, monitors) =
                                create_window.get_mut(self.world_mut());

                            let winit_window = winit_windows.create_window(
                                event_loop,
                                entity,
                                &window,
                                &cursor_options,
                                adapters,
                                &mut handlers,
                                &accessibility_requested,
                                &monitors,
                            );

                            let wrapper = RawHandleWrapper::new(winit_window).unwrap();

                            self.world_mut().entity_mut(entity).insert(wrapper);
                        });
                    });
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
                #[cfg(feature = "custom_cursor")]
                self.update_cursors(event_loop);
                #[cfg(not(feature = "custom_cursor"))]
                self.update_cursors();
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
                        let visible = WINIT_WINDOWS.with_borrow(|winit_windows| {
                            winit_windows.windows.iter().any(|(_, w)| {
                                w.is_visible().unwrap_or(false)
                            })
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
                if let Some(next) = begin_frame_time.checked_add(wait)
                    && self.wait_elapsed
                {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(next));
                }
            }
        }

        if self.redraw_requested && self.lifecycle != AppLifecycle::Suspended {
            WINIT_WINDOWS.with_borrow(|winit_windows| {
                for window in winit_windows.windows.values() {
                    window.request_redraw();
                }
            });
            self.redraw_requested = false;
        }

        if let Some(app_exit) = self.app.should_exit() {
            self.app_exit = Some(app_exit);

            event_loop.exit();
        }
    }

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
        let raw_winit_events = self.raw_winit_events.drain(..).collect::<Vec<_>>();
        let buffered_events = self.bevy_window_events.drain(..).collect::<Vec<_>>();
        let world = self.world_mut();

        if !raw_winit_events.is_empty() {
            world
                .resource_mut::<Events<RawWinitWindowEvent>>()
                .write_batch(raw_winit_events);
        }

        for winit_event in buffered_events.iter() {
            match winit_event.clone() {
                BevyWindowEvent::AppLifecycle(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::CursorEntered(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::CursorLeft(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::CursorMoved(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::FileDragAndDrop(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::Ime(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::RequestRedraw(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowBackendScaleFactorChanged(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowCloseRequested(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowCreated(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowDestroyed(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowFocused(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowMoved(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowOccluded(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowResized(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowScaleFactorChanged(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::WindowThemeChanged(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::MouseButtonInput(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::MouseMotion(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::MouseWheel(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::PinchGesture(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::RotationGesture(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::DoubleTapGesture(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::PanGesture(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::TouchInput(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::KeyboardInput(e) => {
                    world.write_event(e);
                }
                BevyWindowEvent::KeyboardFocusLost(e) => {
                    world.write_event(e);
                }
            }
        }

        if !buffered_events.is_empty() {
            world
                .resource_mut::<Events<BevyWindowEvent>>()
                .write_batch(buffered_events);
        }
    }
}

/// The default [`App::runner`] for the [`WinitPlugin`](crate::WinitPlugin) plugin.
///
/// Overriding the app's [runner](bevy_app::App::runner) while using `WinitPlugin` will bypass the
/// `EventLoop`.
pub fn winit_runner<T: BufferedEvent>(mut app: App, event_loop: EventLoop<T>) -> AppExit {
    if app.plugins_state() == PluginsState::Ready {
        app.finish();
        app.cleanup();
    }

    app.world_mut()
        .insert_resource(EventLoopProxyWrapper(event_loop.create_proxy()));

    let runner_state = WinitAppRunnerState::new(app);

    trace!("starting winit event loop");
    // The winit docs mention using `spawn` instead of `run` on Wasm.
    // https://docs.rs/winit/latest/winit/platform/web/trait.EventLoopExtWebSys.html#tymethod.spawn_app
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            event_loop.spawn_app(runner_state);
            AppExit::Success
        } else {
            let mut runner_state = runner_state;
            if let Err(err) = event_loop.run_app(&mut runner_state) {
                bevy_log::error!("winit event loop returned an error: {err}");
            }
            // If everything is working correctly then the event loop only exits after it's sent an exit code.
            runner_state.app_exit.unwrap_or_else(|| {
                bevy_log::error!("Failed to receive an app exit code! This is a bug");
                AppExit::error()
            })
        }
    }
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

    window_resized.write(WindowResized {
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
    let prior_factor = window.resolution.scale_factor();
    window.resolution.set_scale_factor(scale_factor as f32);

    window_backend_scale_factor_changed.write(WindowBackendScaleFactorChanged {
        window: window_entity,
        scale_factor,
    });

    let scale_factor_override = window.resolution.scale_factor_override();

    if scale_factor_override.is_none() && !relative_eq!(scale_factor as f32, prior_factor) {
        window_scale_factor_changed.write(WindowScaleFactorChanged {
            window: window_entity,
            scale_factor,
        });
    }
}
