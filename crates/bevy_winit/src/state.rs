use approx::relative_eq;
use bevy_app::{App, AppExit, PluginsState};
use bevy_ecs::change_detection::{DetectChanges, NonSendMut, Res};
use bevy_ecs::entity::Entity;
use bevy_ecs::event::{EventWriter, ManualEventReader};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_ecs::world::FromWorld;
use bevy_input::{
    gestures::*,
    keyboard::KeyboardFocusLost,
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
};
use bevy_log::{error, trace, warn};
use bevy_math::{ivec2, DVec2, Vec2};
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;
use bevy_utils::Instant;
use std::marker::PhantomData;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
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
    converters, create_windows, AppSendEvent, CreateWindowParams, UpdateMode, WinitEvent,
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
    /// Is `true` if a new [`T`] event has been received since the last update.
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
        app.add_event::<T>();

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
            wait_elapsed: false,
            // 3 seems to be enough, 5 is a safe margin
            startup_forced_updates: 5,
            winit_events: Vec::new(),
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
            WindowEvent::CloseRequested => self.winit_events.send(WindowCloseRequested { window }),
            WindowEvent::KeyboardInput {
                ref event,
                is_synthetic,
                ..
            } => {
                // Winit sends "synthetic" key press events when the window gains focus. These
                // should not be handled, so we only process key events if they are not synthetic
                // key presses. "synthetic" key release events should still be handled though, for
                // properly releasing keys when the window loses focus.
                if !(is_synthetic && event.state.is_pressed()) {
                    // Process the keyboard input event, as long as it's not a synthetic key press.
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
                self.winit_events.send(PinchGesture(delta as f32));
            }
            WindowEvent::RotationGesture { delta, .. } => {
                self.winit_events.send(RotationGesture(delta));
            }
            WindowEvent::DoubleTapGesture { .. } => {
                self.winit_events.send(DoubleTapGesture);
            }
            WindowEvent::PanGesture { delta, .. } => {
                self.winit_events.send(PanGesture(Vec2 {
                    x: delta.x,
                    y: delta.y,
                }));
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
            WindowEvent::Focused(focused) => {
                win.focused = focused;
                self.winit_events.send(WindowFocused { window, focused });
                if !focused {
                    self.winit_events.send(KeyboardFocusLost);
                }
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // create any new windows
        // (even if app did not update, some may have been created by plugin setup)
        let mut create_window =
            SystemState::<CreateWindowParams<Added<Window>>>::from_world(self.world_mut());
        create_windows(event_loop, create_window.get_mut(self.world_mut()));
        create_window.apply(self.world_mut());

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
                WinitEvent::PinchGesture(e) => {
                    world.send_event(e);
                }
                WinitEvent::RotationGesture(e) => {
                    world.send_event(e);
                }
                WinitEvent::DoubleTapGesture(e) => {
                    world.send_event(e);
                }
                WinitEvent::PanGesture(e) => {
                    world.send_event(e);
                }
                WinitEvent::TouchInput(e) => {
                    world.send_event(e);
                }
                WinitEvent::KeyboardInput(e) => {
                    world.send_event(e);
                }
                WinitEvent::KeyboardFocusLost(e) => {
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
