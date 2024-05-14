use std::sync::mpsc::sync_channel;
use std::time::Instant;
use approx::relative_eq;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_os = "android")]
pub use winit::platform::android::activity as android_activity;
use winit::window::WindowId;
use bevy_app::{App, AppExit, PluginsState};
use bevy_ecs::change_detection::{DetectChanges, NonSendMut, Res};
use bevy_ecs::entity::Entity;
use bevy_ecs::event::{EventWriter, ManualEventReader};
use bevy_ecs::prelude::{Added, Events, NonSend, Query};
use bevy_ecs::system::SystemState;
use bevy_ecs::world::FromWorld;
use bevy_log::{error, trace, warn};
use bevy_math::{DVec2, ivec2, Vec2};
use bevy_input::{
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    touchpad::{TouchpadMagnify, TouchpadRotate},
};
use bevy_tasks::tick_global_task_pools_on_main_thread;

#[cfg(target_os = "android")]
use bevy_window::{PrimaryWindow, RawHandleWrapper};
#[allow(deprecated)]
use bevy_window::{
    ApplicationLifetime, CursorEntered, CursorLeft, CursorMoved,
    FileDragAndDrop, Ime, ReceivedCharacter, RequestRedraw, Window,
    WindowBackendScaleFactorChanged, WindowCloseRequested, WindowCreated, WindowDestroyed,
    WindowFocused, WindowMoved, WindowOccluded, WindowResized, WindowScaleFactorChanged,
    WindowThemeChanged,
};

use crate::{AppSendEvent, converters, create_windows, CreateWindowParams, react_to_resize, UpdateMode, UserEvent, WinitEvent, WinitSettings, WinitWindows};
use crate::accessibility::AccessKitAdapters;
use crate::system::CachedWindow;
use crate::winit_event::forward_winit_events;

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> =
    std::sync::OnceLock::new();

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum UpdateState {
    NotYetStarted,
    Active,
    Suspended,
    WillSuspend,
    WillResume,
}

impl UpdateState {
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        match self {
            Self::NotYetStarted | Self::Suspended => false,
            Self::Active | Self::WillSuspend | Self::WillResume => true,
        }
    }
}

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
pub(crate) struct WinitAppRunnerState {
    /// Current activity state of the app.
    pub(crate) app: App,

    /// Current activity state of the app.
    pub(crate) activity_state: UpdateState,
    /// Current update mode of the app.
    pub(crate) update_mode: UpdateMode,
    /// Is `true` if a new [`WindowEvent`] has been received since the last update.
    pub(crate) window_event_received: bool,
    /// Is `true` if a new [`DeviceEvent`] has been received since the last update.
    pub(crate) device_event_received: bool,
    /// Is `true` if the app has requested a redraw since the last update.
    pub(crate) redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update` to run another update.
    pub(crate) wait_elapsed: bool,
    /// Number of "forced" updates to trigger on application start
    pub(crate) startup_forced_updates: u32,

    /// Winit events to send
    winit_events: Vec<WinitEvent>,
}

impl WinitAppRunnerState {
    fn new(app: App) -> Self {
        Self {
            app,
            activity_state: UpdateState::NotYetStarted,
            update_mode: UpdateMode::Continuous,
            window_event_received: false,
            device_event_received: false,
            redraw_requested: false,
            wait_elapsed: false,
            // 3 seems to be enough, 5 is a safe margin
            startup_forced_updates: 5,
            winit_events: Vec::new(),
        }
    }
    pub(crate) fn reset_on_update(&mut self) {
        self.window_event_received = false;
        self.device_event_received = false;
    }
}

impl ApplicationHandler<UserEvent> for WinitAppRunnerState {
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
            SystemState::<CreateWindowParams<Added<Window>>>::from_world(self.app.world_mut());
        create_windows(event_loop, create_window.get_mut(self.app.world_mut()));
        create_window.apply(self.app.world_mut());



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
        match self.activity_state {
            UpdateState::NotYetStarted => self.winit_events.send(ApplicationLifetime::Started),
            _ => self.winit_events.send(ApplicationLifetime::Resumed),
        }
        self.activity_state = UpdateState::WillResume;
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {
        self.redraw_requested = true;
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let mut event_writer_system_state: SystemState<(
            EventWriter<WindowResized>,
            NonSend<WinitWindows>,
            Query<(&mut Window, &mut CachedWindow)>,
            NonSendMut<AccessKitAdapters>,
        )> = SystemState::new(self.app.world_mut());

        let (mut window_resized, winit_windows, mut windows, mut access_kit_adapters) =
            event_writer_system_state.get_mut(self.app.world_mut());

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

        self.window_event_received = true;

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
                self.winit_events.send(converters::convert_keyboard_input(event, window));
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
                self.winit_events.send(converters::convert_touch_input(touch, location, window));
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
                self.winit_events.send(FileDragAndDrop::DroppedFile { window, path_buf });
            }
            WindowEvent::HoveredFile(path_buf) => {
                self.winit_events.send(FileDragAndDrop::HoveredFile { window, path_buf });
            }
            WindowEvent::HoveredFileCancelled => {
                self.winit_events.send(FileDragAndDrop::HoveredFileCanceled { window });
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
            WindowEvent::RedrawRequested => {
                self.run_app_update();
            }
            _ => {}
        }

        let mut windows = self.app.world_mut().query::<(&mut Window, &mut CachedWindow)>();
        if let Ok((window_component, mut cache)) = windows.get_mut(self.app.world_mut(), window) {
            if window_component.is_changed() {
                cache.window = window_component.clone();
            }
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        self.device_event_received = true;
        if let DeviceEvent::MouseMotion { delta: (x, y) } = event {
            let delta = Vec2::new(x as f32, y as f32);
            self.winit_events.send(MouseMotion { delta });
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();

        let mut focused_windows_state: SystemState<(Res<WinitSettings>, Query<(Entity, &Window)>)> =
            SystemState::new(self.app.world_mut());

        if let Some(app_redraw_events) = self.app.world().get_resource::<Events<RequestRedraw>>() {
            if redraw_event_reader.read(app_redraw_events).last().is_some() {
                self.redraw_requested = true;
            }
        }

        let (config, windows) = focused_windows_state.get(self.app.world());
        let focused = windows.iter().any(|(_, window)| window.focused);

        let mut update_mode = config.update_mode(focused);
        let mut should_update = self.should_update(update_mode);

        if self.startup_forced_updates > 0 {
            self.startup_forced_updates -= 1;
            // Ensure that an update is triggered on the first iterations for app initialization
            should_update = true;
        }

        if self.activity_state == UpdateState::WillSuspend {
            self.activity_state = UpdateState::Suspended;
            // Trigger one last update to enter the suspended state
            should_update = true;

            #[cfg(target_os = "android")]
            {
                // Remove the `RawHandleWrapper` from the primary window.
                // This will trigger the surface destruction.
                let mut query = app
                    .world_mut()
                    .query_filtered::<Entity, With<PrimaryWindow>>();
                let entity = query.single(&app.world());
                app.world_mut()
                    .entity_mut(entity)
                    .remove::<RawHandleWrapper>();
            }
        }

        if self.activity_state == UpdateState::WillResume {
            self.activity_state = UpdateState::Active;
            // Trigger the update to enter the active state
            should_update = true;
            // Trigger the next redraw ro refresh the screen immediately
            self.redraw_requested = true;

            #[cfg(target_os = "android")]
            {
                // Get windows that are cached but without raw handles. Those window were already created, but got their
                // handle wrapper removed when the app was suspended.
                let mut query = app
                    .world_mut()
                    .query_filtered::<(Entity, &Window), (With<CachedWindow>, Without<bevy_window::RawHandleWrapper>)>();
                if let Ok((entity, window)) = query.get_single(&app.world()) {
                    let window = window.clone();

                    let (
                        ..,
                        mut winit_windows,
                        mut adapters,
                        mut handlers,
                        accessibility_requested,
                    ) = create_window.get_mut(app.world_mut());

                    let winit_window = winit_windows.create_window(
                        event_loop,
                        entity,
                        &window,
                        &mut adapters,
                        &mut handlers,
                        &accessibility_requested,
                    );

                    let wrapper = RawHandleWrapper::new(winit_window).unwrap();

                    app.world_mut().entity_mut(entity).insert(wrapper);
                }
            }
        }

        // This is recorded before running app.update(), to run the next cycle after a correct timeout.
        // If the cycle takes more than the wait timeout, it will be re-executed immediately.
        let begin_frame_time = Instant::now();

        if should_update {
            // Not redrawing, but the timeout elapsed.
            self.run_app_update();

            // Running the app may have changed the WinitSettings resource, so we have to re-extract it.
            let (config, windows) = focused_windows_state.get(self.app.world());
            let focused = windows.iter().any(|(_, window)| window.focused);

            update_mode = config.update_mode(focused);
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
                            let winit_windows = self.app.world().non_send_resource::<WinitWindows>();
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

        if update_mode != self.update_mode {
            // Trigger the next redraw since we're changing the update mode
            self.redraw_requested = true;
            self.update_mode = update_mode;
        }

        if self.redraw_requested
            && self.activity_state != UpdateState::Suspended
        {
            let winit_windows = self.app.world().non_send_resource::<WinitWindows>();
            for window in winit_windows.windows.values() {
                window.request_redraw();
            }
            self.redraw_requested = false;
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.winit_events.send(ApplicationLifetime::Suspended);
        // Mark the state as `WillSuspend`. This will let the schedule run one last time
        // before actually suspending to let the application react
        self.activity_state = UpdateState::WillSuspend;
    }
}

impl WinitAppRunnerState {
    fn should_update(&self, update_mode: UpdateMode) -> bool {
        let handle_event = match update_mode {
            UpdateMode::Continuous | UpdateMode::Reactive { .. } => {
                self.wait_elapsed
                    || self.window_event_received
                    || self.device_event_received
            }
            UpdateMode::ReactiveLowPower { .. } => {
                self.wait_elapsed || self.window_event_received
            }
        };

        handle_event && self.activity_state.is_active()
    }

    fn run_app_update(
        &mut self,
    ) {
        self.reset_on_update();

        self.forward_winit_events();

        if self.app.plugins_state() == PluginsState::Cleaned {
            self.app.update();
        }
    }

    fn forward_winit_events(&mut self) {
        let buffered_events = &mut self.winit_events;
        let app = &mut self.app;

        if buffered_events.is_empty() {
            return;
        }

        for winit_event in buffered_events.iter() {
            match winit_event.clone() {
                WinitEvent::ApplicationLifetime(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::CursorEntered(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::CursorLeft(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::CursorMoved(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::FileDragAndDrop(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::Ime(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::ReceivedCharacter(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::RequestRedraw(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowBackendScaleFactorChanged(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowCloseRequested(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowCreated(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowDestroyed(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowFocused(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowMoved(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowOccluded(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowResized(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowScaleFactorChanged(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::WindowThemeChanged(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::MouseButtonInput(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::MouseMotion(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::MouseWheel(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::TouchpadMagnify(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::TouchpadRotate(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::TouchInput(e) => {
                    app.world_mut().send_event(e);
                }
                WinitEvent::KeyboardInput(e) => {
                    app.world_mut().send_event(e);
                }
            }
        }
        app.world_mut()
            .resource_mut::<Events<WinitEvent>>()
            .send_batch(buffered_events.drain(..));
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

    // TODO: restore this
    // // Create a channel with a size of 1, since ideally only one exit code will be sent before exiting the app.
    // let (exit_sender, exit_receiver) = sync_channel(1);
    //
    //
    // let mut create_window =
    //     SystemState::<CreateWindowParams<Added<Window>>>::from_world(app.world_mut());
    // let mut winit_events = Vec::default();
    //
    // // set up the event loop
    // let event_handler = move |event, event_loop: &ActiveEventLoop| {
    //     // The event loop is in the process of exiting, so don't deliver any new events
    //     if event_loop.exiting() {
    //         return;
    //     }
    //
    //     crate::runner::handle_winit_event(
    //         &mut app,
    //         &mut runner_state,
    //         &mut create_window,
    //         &mut event_writer_system_state,
    //         &mut focused_windows_state,
    //         &mut redraw_event_reader,
    //         &mut winit_events,
    //         &exit_sender,
    //         event,
    //         event_loop,
    //     );
    // };

    trace!("starting winit event loop");
    // TODO(clean): the winit docs mention using `spawn` instead of `run` on WASM.
    if let Err(err) = event_loop.run_app(&mut runner_state) {
        error!("winit event loop returned an error: {err}");
    }

    // TODO: restore this
    // // If everything is working correctly then the event loop only exits after it's sent a exit code.
    // exit_receiver
    //     .try_recv()
    //     .map_err(|err| error!("Failed to receive a app exit code! This is a bug. Reason: {err}"))
    //     .unwrap_or(AppExit::error())

    // TODO: remove this
    AppExit::error()
}
