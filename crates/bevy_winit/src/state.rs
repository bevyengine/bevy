use approx::relative_eq;
use bevy_app::{App, AppExit, PluginsState};
use bevy_ecs::change_detection::{DetectChanges, NonSendMut, Res};
use bevy_ecs::entity::Entity;
use bevy_ecs::event::{EventWriter, ManualEventReader};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_ecs::world::FromWorld;
use bevy_input::{
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    touchpad::{TouchpadMagnify, TouchpadRotate},
};
use bevy_log::{error, trace, warn};
use bevy_math::{ivec2, DVec2, Vec2};
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;
use bevy_utils::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

#[allow(deprecated)]
use bevy_window::{
    AppLifecycle, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime, ReceivedCharacter,
    RequestRedraw, Window, WindowBackendScaleFactorChanged, WindowCloseRequested, WindowDestroyed,
    WindowFocused, WindowMoved, WindowOccluded, WindowResized, WindowScaleFactorChanged,
    WindowThemeChanged,
};
#[cfg(target_os = "android")]
use bevy_window::{PrimaryWindow, RawHandleWrapper};

use crate::accessibility::AccessKitAdapters;
use crate::system::CachedWindow;
use crate::{
    converters, create_windows, react_to_resize, AppSendEvent, CreateWindowParams, UpdateMode,
    UserEvent, WinitEvent, WinitSettings, WinitWindows,
};

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
pub(crate) struct WinitAppRunnerState {
    /// The running app.
    app: App,
    /// Exit value once the loop is finished.
    app_exit: Option<AppExit>,
    /// Current update mode of the app.
    update_mode: UpdateMode,
    /// Is `true` if a new [`WindowEvent`] has been received since the last update.
    window_event_received: bool,
    /// Is `true` if a new [`DeviceEvent`] has been received since the last update.
    device_event_received: bool,
    /// Is `true` if a new [`UserEvent`] has been received since the last update.
    user_event_received: bool,
    /// Is `true` if the app has requested a redraw since the last update.
    redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update` to run another update.
    wait_elapsed: bool,
    /// Number of "forced" updates to trigger on application start
    startup_forced_updates: u32,

    /// Current app lifecycle state.
    lifecycle: AppLifecycle,
    /// The previous app lifecycle state.
    previous_lifecycle: AppLifecycle,
    /// Winit events to send
    winit_events: Vec<WinitEvent>,
}

impl WinitAppRunnerState {
    fn new(app: App) -> Self {
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
            wait_elapsed: false,
            // 3 seems to be enough, 5 is a safe margin
            startup_forced_updates: 5,
            winit_events: Vec::new(),
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

impl ApplicationHandler<UserEvent> for WinitAppRunnerState {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Mark the state as `WillResume`. This will let the schedule run one extra time
        // when actually resuming the app
        self.lifecycle = AppLifecycle::WillResume;
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Mark the state as `WillSuspend`. This will let the schedule run one last time
        // before actually suspending to let the application react
        self.lifecycle = AppLifecycle::WillSuspend;
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
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

        // create any new windows
        // (even if app did not update, some may have been created by plugin setup)
        let mut create_window =
            SystemState::<CreateWindowParams<Added<Window>>>::from_world(self.world_mut());
        create_windows(event_loop, create_window.get_mut(self.world_mut()));
        create_window.apply(self.world_mut());

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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();

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
                    ) = create_window.get_mut(self.world_mut());

                    let winit_window = winit_windows.create_window(
                        event_loop,
                        entity,
                        &window,
                        &mut adapters,
                        &mut handlers,
                        &accessibility_requested,
                    );

                    let wrapper = RawHandleWrapper::new(winit_window).unwrap();

                    self.world_mut().entity_mut(entity).insert(wrapper);
                }
            }
        }

        // Notifies a lifecycle change
        if self.lifecycle != self.previous_lifecycle {
            self.previous_lifecycle = self.lifecycle;
            self.winit_events.send(self.lifecycle);
        }

        // This is recorded before running app.update(), to run the next cycle after a correct timeout.
        // If the cycle takes more than the wait timeout, it will be re-executed immediately.
        let begin_frame_time = Instant::now();

        if should_update {
            // Not redrawing, but the timeout elapsed.
            self.run_app_update();

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
            UpdateMode::Reactive { wait } | UpdateMode::ReactiveLowPower { wait } => {
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
            return;
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
            self.winit_events.send(MouseMotion { delta });
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.window_event_received = true;

        let mut event_writer_system_state: SystemState<(
            EventWriter<WindowResized>,
            NonSend<WinitWindows>,
            Query<(&mut Window, &mut CachedWindow)>,
            NonSendMut<AccessKitAdapters>,
        )> = SystemState::new(self.world_mut());

        let (mut window_resized, winit_windows, mut windows, mut access_kit_adapters) =
            event_writer_system_state.get_mut(self.world_mut());

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
                react_to_resize(&mut win, size, &mut window_resized, window);
            }
            WindowEvent::CloseRequested => self.winit_events.send(WindowCloseRequested { window }),
            WindowEvent::KeyboardInput { ref event, .. } => {
                if event.state.is_pressed() {
                    if let Some(char) = &event.text {
                        let char = char.clone();
                        #[allow(deprecated)]
                        self.winit_events.send(ReceivedCharacter { window, char });
                    }
                }
                self.winit_events
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
                self.winit_events.send(CursorMoved {
                    window,
                    position,
                    delta,
                });
            }
            WindowEvent::CursorEntered { .. } => {
                self.winit_events.send(CursorEntered { window });
            }
            WindowEvent::CursorLeft { .. } => {
                win.set_physical_cursor_position(None);
                self.winit_events.send(CursorLeft { window });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.winit_events.send(MouseButtonInput {
                    button: converters::convert_mouse_button(button),
                    state: converters::convert_element_state(state),
                    window,
                });
            }
            WindowEvent::PinchGesture { delta, .. } => {
                self.winit_events.send(TouchpadMagnify(delta as f32));
            }
            WindowEvent::RotationGesture { delta, .. } => {
                self.winit_events.send(TouchpadRotate(delta));
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                event::MouseScrollDelta::LineDelta(x, y) => {
                    self.winit_events.send(MouseWheel {
                        unit: MouseScrollUnit::Line,
                        x,
                        y,
                        window,
                    });
                }
                event::MouseScrollDelta::PixelDelta(p) => {
                    self.winit_events.send(MouseWheel {
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
                self.winit_events
                    .send(converters::convert_touch_input(touch, location, window));
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

                self.winit_events.send(WindowBackendScaleFactorChanged {
                    window,
                    scale_factor,
                });
                if scale_factor_override.is_none() && !relative_eq!(new_factor, prior_factor) {
                    self.winit_events.send(WindowScaleFactorChanged {
                        window,
                        scale_factor,
                    });
                }

                if !width_equal || !height_equal {
                    self.winit_events.send(WindowResized {
                        window,
                        width: new_logical_width,
                        height: new_logical_height,
                    });
                }
            }
            WindowEvent::Focused(focused) => {
                win.focused = focused;
                self.winit_events.send(WindowFocused { window, focused });
            }
            WindowEvent::Occluded(occluded) => {
                self.winit_events.send(WindowOccluded { window, occluded });
            }
            WindowEvent::DroppedFile(path_buf) => {
                self.winit_events
                    .send(FileDragAndDrop::DroppedFile { window, path_buf });
            }
            WindowEvent::HoveredFile(path_buf) => {
                self.winit_events
                    .send(FileDragAndDrop::HoveredFile { window, path_buf });
            }
            WindowEvent::HoveredFileCancelled => {
                self.winit_events
                    .send(FileDragAndDrop::HoveredFileCanceled { window });
            }
            WindowEvent::Moved(position) => {
                let position = ivec2(position.x, position.y);
                win.position.set(position);
                self.winit_events.send(WindowMoved { window, position });
            }
            WindowEvent::Ime(event) => match event {
                event::Ime::Preedit(value, cursor) => {
                    self.winit_events.send(Ime::Preedit {
                        window,
                        value,
                        cursor,
                    });
                }
                event::Ime::Commit(value) => {
                    self.winit_events.send(Ime::Commit { window, value });
                }
                event::Ime::Enabled => {
                    self.winit_events.send(Ime::Enabled { window });
                }
                event::Ime::Disabled => {
                    self.winit_events.send(Ime::Disabled { window });
                }
            },
            WindowEvent::ThemeChanged(theme) => {
                self.winit_events.send(WindowThemeChanged {
                    window,
                    theme: converters::convert_winit_theme(theme),
                });
            }
            WindowEvent::Destroyed => {
                self.winit_events.send(WindowDestroyed { window });
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {
        self.user_event_received = true;
        self.redraw_requested = true;
    }
}

impl WinitAppRunnerState {
    fn should_update(&self, update_mode: UpdateMode) -> bool {
        let handle_event = match update_mode {
            UpdateMode::Continuous | UpdateMode::Reactive { .. } => {
                self.wait_elapsed
                    || self.user_event_received
                    || self.window_event_received
                    || self.device_event_received
            }
            UpdateMode::ReactiveLowPower { .. } => {
                self.wait_elapsed || self.user_event_received || self.window_event_received
            }
        };

        handle_event && self.lifecycle.is_active()
    }

    fn run_app_update(&mut self) {
        self.reset_on_update();

        self.forward_winit_events();

        if self.app.plugins_state() == PluginsState::Cleaned {
            self.app.update();
        }
    }

    fn forward_winit_events(&mut self) {
        let buffered_events = self.winit_events.drain(..).collect::<Vec<_>>();

        if buffered_events.is_empty() {
            return;
        }

        let world = self.world_mut();

        for winit_event in buffered_events.iter() {
            match winit_event.clone() {
                WinitEvent::AppLifecycle(e) => {
                    world.send_event(e);
                }
                WinitEvent::CursorEntered(e) => {
                    world.send_event(e);
                }
                WinitEvent::CursorLeft(e) => {
                    world.send_event(e);
                }
                WinitEvent::CursorMoved(e) => {
                    world.send_event(e);
                }
                WinitEvent::FileDragAndDrop(e) => {
                    world.send_event(e);
                }
                WinitEvent::Ime(e) => {
                    world.send_event(e);
                }
                WinitEvent::ReceivedCharacter(e) => {
                    world.send_event(e);
                }
                WinitEvent::RequestRedraw(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowBackendScaleFactorChanged(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowCloseRequested(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowCreated(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowDestroyed(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowFocused(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowMoved(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowOccluded(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowResized(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowScaleFactorChanged(e) => {
                    world.send_event(e);
                }
                WinitEvent::WindowThemeChanged(e) => {
                    world.send_event(e);
                }
                WinitEvent::MouseButtonInput(e) => {
                    world.send_event(e);
                }
                WinitEvent::MouseMotion(e) => {
                    world.send_event(e);
                }
                WinitEvent::MouseWheel(e) => {
                    world.send_event(e);
                }
                WinitEvent::TouchpadMagnify(e) => {
                    world.send_event(e);
                }
                WinitEvent::TouchpadRotate(e) => {
                    world.send_event(e);
                }
                WinitEvent::TouchInput(e) => {
                    world.send_event(e);
                }
                WinitEvent::KeyboardInput(e) => {
                    world.send_event(e);
                }
            }
        }

        world
            .resource_mut::<Events<WinitEvent>>()
            .send_batch(buffered_events);
    }
}

/// The default [`App::runner`] for the [`WinitPlugin`] plugin.
///
/// Overriding the app's [runner](bevy_app::App::runner) while using `WinitPlugin` will bypass the
/// `EventLoop`.
pub fn winit_runner(mut app: App) -> AppExit {
    if app.plugins_state() == PluginsState::Ready {
        app.finish();
        app.cleanup();
    }

    let event_loop = app
        .world_mut()
        .remove_non_send_resource::<EventLoop<UserEvent>>()
        .unwrap();

    app.world_mut()
        .insert_non_send_resource(event_loop.create_proxy());

    let mut runner_state = WinitAppRunnerState::new(app);

    trace!("starting winit event loop");
    // TODO(clean): the winit docs mention using `spawn` instead of `run` on WASM.
    if let Err(err) = event_loop.run_app(&mut runner_state) {
        error!("winit event loop returned an error: {err}");
    }

    // If everything is working correctly then the event loop only exits after it's sent an exit code.
    runner_state.app_exit.unwrap_or_else(|| {
        error!("Failed to receive a app exit code! This is a bug");
        AppExit::error()
    })
}
