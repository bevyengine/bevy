mod converters;
mod system;
#[cfg(target_arch = "wasm32")]
mod web_resize;
mod winit_config;
mod winit_windows;

use system::{changed_windows, create_windows, despawn_windows, CachedWindow};

pub use winit_config::*;
pub use winit_windows::*;

use bevy_app::{App, AppExit, CoreSet, Plugin};
use bevy_ecs::event::{Events, ManualEventReader};
use bevy_ecs::prelude::*;
use bevy_ecs::system::{SystemParam, SystemState};
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    touch::TouchInput,
};
use bevy_math::{ivec2, DVec2, Vec2};
use bevy_utils::{
    tracing::{trace, warn},
    Instant,
};
use bevy_window::{
    exit_on_all_closed, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime,
    ReceivedCharacter, RequestRedraw, Window, WindowBackendScaleFactorChanged,
    WindowCloseRequested, WindowFocused, WindowMoved, WindowResized, WindowScaleFactorChanged,
};

#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

use winit::{
    event::{self, DeviceEvent, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget},
};

#[cfg(target_arch = "wasm32")]
use crate::web_resize::{CanvasParentResizeEventChannel, CanvasParentResizePlugin};

#[cfg(target_os = "android")]
pub static ANDROID_APP: once_cell::sync::OnceCell<AndroidApp> = once_cell::sync::OnceCell::new();

/// Integrates [`winit`], extending an [`App`] with capabilities for managing windows and
/// receiving window and input devicewriter.
///
/// **NOTE:** This plugin will replace the existing application runner function.
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
        app.init_non_send_resource::<WinitWindows>();

        #[cfg(target_arch = "wasm32")]
        app.add_plugin(CanvasParentResizePlugin);

        #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
        {
            // iOS and macOS do not like it when you create windows outside of the event loop.
            // See:
            // - https://github.com/rust-windowing/winit/blob/master/README.md#macos
            // - https://github.com/rust-windowing/winit/blob/master/README.md#ios
            //
            // And we just make Android match the iOS config.
            //
            // Otherwise, we try to create a window before `bevy_render` initializes
            // the renderer, so that we have a surface available to use as a hint.
            // This improves compatibility with wgpu backends, especially WASM/WebGL2.
            let create_windows = IntoSystem::into_system(create_windows);
            create_windows.run(&event_loop, &mut app.world);
            create_windows.apply_buffers(&mut app.world);
        }

        app.insert_non_send_resource(event_loop)
            .init_resource::<WinitSettings>()
            .set_runner(winit_runner)
            .add_systems(
                (
                    // `exit_on_all_closed` seemingly conflicts with `changed_window`
                    // but does not actually access any data (only checks if the query is empty)
                    changed_windows.ambiguous_with(exit_on_all_closed),
                    // apply all changes first, then despawn windows
                    despawn_windows.after(changed_windows),
                )
                    .in_base_set(CoreSet::Last),
            );
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
struct WinitEventWriters<'w> {
    // window events
    window_resized: EventWriter<'w, WindowResized>,
    window_close_requested: EventWriter<'w, WindowCloseRequested>,
    window_scale_factor_changed: EventWriter<'w, WindowScaleFactorChanged>,
    window_backend_scale_factor_changed: EventWriter<'w, WindowBackendScaleFactorChanged>,
    window_focused: EventWriter<'w, WindowFocused>,
    window_moved: EventWriter<'w, WindowMoved>,
    file_drag_and_drop: EventWriter<'w, FileDragAndDrop>,

    keyboard_input: EventWriter<'w, KeyboardInput>,
    character_input: EventWriter<'w, ReceivedCharacter>,
    mouse_button_input: EventWriter<'w, MouseButtonInput>,
    mouse_wheel_input: EventWriter<'w, MouseWheel>,
    touch_input: EventWriter<'w, TouchInput>,
    ime_input: EventWriter<'w, Ime>,

    cursor_moved: EventWriter<'w, CursorMoved>,
    cursor_entered: EventWriter<'w, CursorEntered>,
    cursor_left: EventWriter<'w, CursorLeft>,

    // device events
    mouse_motion: EventWriter<'w, MouseMotion>,
}

/// Metadata used to control app updates.
struct WinitAppRunnerState {
    /// Is `true` if the app is not suspended.
    active: bool,
    /// Is `true` if a new window or input device event has been received.
    window_or_device_event_received: bool,
    /// Is `true` if a new window event has been received.
    window_event_received: bool,
    /// Is `true` if the app has requested a redraw.
    redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update`.
    timeout_elapsed: bool,
    /// The time the most recent update started.
    last_update: Instant,
}

impl Default for WinitAppRunnerState {
    fn default() -> Self {
        Self {
            active: false,
            window_or_device_event_received: false,
            window_event_received: false,
            redraw_requested: false,
            timeout_elapsed: false,
            last_update: Instant::now(),
        }
    }
}

pub fn winit_runner(mut app: App) {
    let mut event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<()>>()
        .unwrap();

    let return_on_loop_exit = app.world.resource::<WinitSettings>().return_on_loop_exit;

    app.world
        .insert_non_send_resource(event_loop.create_proxy());

    let mut winit_state = WinitAppRunnerState::default();

    // prepare structures to access data in the world
    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();

    let mut focused_windows_state: SystemState<(Res<WinitSettings>, Query<&Window>)> =
        SystemState::new(&mut app.world);

    let mut event_writer_system_state: SystemState<(
        WinitEventWriters,
        NonSend<WinitWindows>,
        Query<(&mut Window, &mut CachedWindow)>,
    )> = SystemState::new(&mut app.world);

    let create_windows = IntoSystem::into_system(create_windows);

    // setup up the event loop
    let event_handler = move |event: Event<()>,
                              event_loop: &EventLoopWindowTarget<()>,
                              control_flow: &mut ControlFlow| {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("winit event_handler").entered();

        if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
            if app_exit_event_reader.iter(app_exit_events).last().is_some() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if winit_state.active {
            create_windows.run(event_loop, &mut app.world);
            create_windows.apply_buffers(&mut app.world);
        }

        match event {
            event::Event::NewEvents(start_cause) => {
                // reset these after each update
                winit_state.timeout_elapsed = false;
                winit_state.window_or_device_event_received = false;
                winit_state.window_event_received = false;

                match start_cause {
                    StartCause::ResumeTimeReached { .. } => {
                        // `WaitUntil` timeout
                        winit_state.timeout_elapsed = true;
                    }
                    _ => {
                        // something else triggered this iteration of the loop
                        // check timeout manually
                        let now = Instant::now();
                        let (winit_config, windows) = focused_windows_state.get(&app.world);
                        let focused = windows.iter().any(|window| window.focused);
                        winit_state.timeout_elapsed = match winit_config.update_mode(focused) {
                            UpdateMode::Continuous => true,
                            UpdateMode::Reactive { min_wait }
                            | UpdateMode::ReactiveLowPower { min_wait } => {
                                now.duration_since(winit_state.last_update) >= *min_wait
                            }
                        };
                    }
                }
            }
            event::Event::WindowEvent {
                event,
                window_id: winit_window_id,
                ..
            } => {
                let (mut writers, winit_windows, mut windows) =
                    event_writer_system_state.get_mut(&mut app.world);

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
                    if let Ok((window, info)) = windows.get_mut(window_entity) {
                        (window, info)
                    } else {
                        warn!(
                            "Window {:?} is missing `Window` component, skipping event {:?}",
                            window_entity, event
                        );
                        return;
                    };

                winit_state.window_or_device_event_received = true;
                winit_state.window_event_received = true;

                match event {
                    WindowEvent::Resized(size) => {
                        window
                            .resolution
                            .set_physical_resolution(size.width, size.height);

                        writers.window_resized.send(WindowResized {
                            window: window_entity,
                            width: window.width(),
                            height: window.height(),
                        });
                    }
                    WindowEvent::CloseRequested => {
                        writers.window_close_requested.send(WindowCloseRequested {
                            window: window_entity,
                        });
                    }
                    WindowEvent::KeyboardInput { ref input, .. } => {
                        writers
                            .keyboard_input
                            .send(converters::convert_keyboard_input(input));
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let physical_position = DVec2::new(
                            position.x,
                            // flip the coordinate system so it matches ours
                            window.resolution.physical_height() as f64 - position.y,
                        );

                        window.set_physical_cursor_position(Some(physical_position));
                        writers.cursor_moved.send(CursorMoved {
                            window: window_entity,
                            position: (physical_position / window.resolution.scale_factor())
                                .as_vec2(),
                        });
                    }
                    WindowEvent::CursorEntered { .. } => {
                        writers.cursor_entered.send(CursorEntered {
                            window: window_entity,
                        });
                    }
                    WindowEvent::CursorLeft { .. } => {
                        window.set_physical_cursor_position(None);
                        writers.cursor_left.send(CursorLeft {
                            window: window_entity,
                        });
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        writers.mouse_button_input.send(MouseButtonInput {
                            button: converters::convert_mouse_button(button),
                            state: converters::convert_element_state(state),
                        });
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        event::MouseScrollDelta::LineDelta(x, y) => {
                            writers.mouse_wheel_input.send(MouseWheel {
                                unit: MouseScrollUnit::Line,
                                x,
                                y,
                            });
                        }
                        event::MouseScrollDelta::PixelDelta(p) => {
                            writers.mouse_wheel_input.send(MouseWheel {
                                unit: MouseScrollUnit::Pixel,
                                x: p.x as f32,
                                y: p.y as f32,
                            });
                        }
                    },
                    WindowEvent::Touch(touch) => {
                        let location = touch.location.to_logical(window.resolution.scale_factor());
                        writers
                            .touch_input
                            .send(converters::convert_touch_input(touch, location));
                    }
                    WindowEvent::ReceivedCharacter(char) => {
                        writers.character_input.send(ReceivedCharacter {
                            window: window_entity,
                            char,
                        });
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {
                        writers.window_backend_scale_factor_changed.send(
                            WindowBackendScaleFactorChanged {
                                window: window_entity,
                                scale_factor,
                            },
                        );

                        let prior_factor = window.resolution.scale_factor();
                        window.resolution.set_scale_factor(scale_factor);
                        let new_factor = window.resolution.scale_factor();

                        if let Some(forced_factor) = window.resolution.scale_factor_override() {
                            // TODO: should this branch send a WindowsScaleFactorChanged event too?
                            // TODO: word this comment better
                            // If there is a scale factor override, then force that to be used
                            // Otherwise, use the OS suggested size
                            // We have already told the OS about our resize constraints, so
                            // the new_inner_size should take those into account
                            *new_inner_size =
                                winit::dpi::LogicalSize::new(window.width(), window.height())
                                    .to_physical::<u32>(forced_factor);
                        } else if approx::relative_ne!(new_factor, prior_factor) {
                            // send a change event if these are different enough
                            writers
                                .window_scale_factor_changed
                                .send(WindowScaleFactorChanged {
                                    window: window_entity,
                                    scale_factor,
                                });
                        }

                        let new_logical_width = (new_inner_size.width as f64 / new_factor) as f32;
                        let new_logical_height = (new_inner_size.height as f64 / new_factor) as f32;
                        if approx::relative_ne!(window.width(), new_logical_width)
                            || approx::relative_ne!(window.height(), new_logical_height)
                        {
                            writers.window_resized.send(WindowResized {
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
                        writers.window_focused.send(WindowFocused {
                            window: window_entity,
                            focused,
                        });
                    }
                    WindowEvent::DroppedFile(path_buf) => {
                        writers
                            .file_drag_and_drop
                            .send(FileDragAndDrop::DroppedFile {
                                window: window_entity,
                                path_buf,
                            });
                    }
                    WindowEvent::HoveredFile(path_buf) => {
                        writers
                            .file_drag_and_drop
                            .send(FileDragAndDrop::HoveredFile {
                                window: window_entity,
                                path_buf,
                            });
                    }
                    WindowEvent::HoveredFileCancelled => {
                        writers
                            .file_drag_and_drop
                            .send(FileDragAndDrop::HoveredFileCancelled {
                                window: window_entity,
                            });
                    }
                    WindowEvent::Moved(position) => {
                        let position = ivec2(position.x, position.y);
                        window.position.set(position);
                        writers.window_moved.send(WindowMoved {
                            entity: window_entity,
                            position,
                        });
                    }
                    WindowEvent::Ime(event) => match event {
                        event::Ime::Preedit(value, cursor) => {
                            writers.ime_input.send(Ime::Preedit {
                                window: window_entity,
                                value,
                                cursor,
                            });
                        }
                        event::Ime::Commit(value) => writers.ime_input.send(Ime::Commit {
                            window: window_entity,
                            value,
                        }),
                        event::Ime::Enabled => writers.ime_input.send(Ime::Enabled {
                            window: window_entity,
                        }),
                        event::Ime::Disabled => writers.ime_input.send(Ime::Disabled {
                            window: window_entity,
                        }),
                    },
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
                let (mut writers, _, _) = event_writer_system_state.get_mut(&mut app.world);
                writers.mouse_motion.send(MouseMotion {
                    delta: Vec2::new(x as f32, y as f32),
                });

                winit_state.window_or_device_event_received = true;
            }
            event::Event::Suspended => {
                winit_state.active = false;
                #[cfg(target_os = "android")]
                {
                    // When Android sends this event, it invalidates all render surfaces.
                    // Restart the application.
                    // TODO
                    // Upon resume, check if the new render surfaces are compatible with the
                    // existing render device. If not (which should basically never happen),
                    // *then* try to rebuild the renderer.
                    *control_flow = ControlFlow::Exit;
                }
            }
            event::Event::Resumed => {
                winit_state.active = true;
            }
            event::Event::MainEventsCleared => {
                if winit_state.active {
                    let (winit_config, windows) = focused_windows_state.get(&app.world);
                    let focused = windows.iter().any(|window| window.focused);
                    let update = match winit_config.update_mode(focused) {
                        UpdateMode::Continuous => true,
                        UpdateMode::Reactive { .. } => {
                            winit_state.timeout_elapsed
                                || winit_state.redraw_requested
                                || winit_state.window_or_device_event_received
                        }
                        UpdateMode::ReactiveLowPower { .. } => {
                            winit_state.timeout_elapsed
                                || winit_state.redraw_requested
                                || winit_state.window_event_received
                        }
                    };

                    if update {
                        winit_state.last_update = Instant::now();
                        app.update();
                    }
                }
            }
            Event::RedrawEventsCleared => {
                let now = Instant::now();
                let (winit_config, windows) = focused_windows_state.get(&app.world);
                let focused = windows.iter().any(|window| window.focused);
                *control_flow = match winit_config.update_mode(focused) {
                    UpdateMode::Continuous => ControlFlow::Poll,
                    UpdateMode::Reactive { min_wait }
                    | UpdateMode::ReactiveLowPower { min_wait } => {
                        if let Some(instant) = now.checked_add(*min_wait) {
                            ControlFlow::WaitUntil(instant)
                        } else {
                            ControlFlow::Wait
                        }
                    }
                };

                // check for any redraw requests submitted by the most recent update
                winit_state.redraw_requested = false;
                if let Some(app_redraw_events) = app.world.get_resource::<Events<RequestRedraw>>() {
                    if redraw_event_reader.iter(app_redraw_events).last().is_some() {
                        winit_state.redraw_requested = true;
                        *control_flow = ControlFlow::Poll;
                    }
                }
            }
            _ => (),
        }
    };

    trace!("starting winit event loop");
    if return_on_loop_exit {
        run_return(&mut event_loop, event_handler);
    } else {
        run(event_loop, event_handler);
    }
}
