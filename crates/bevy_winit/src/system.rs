use std::collections::HashMap;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    lifecycle::RemovedComponents,
    message::MessageWriter,
    prelude::{Changed, Component},
    query::QueryFilter,
    system::{Local, NonSendMarker, Query, SystemParamItem},
};
use bevy_input::keyboard::{Key, KeyCode, KeyboardFocusLost, KeyboardInput};
use bevy_window::{
    ClosingWindow, CursorOptions, Monitor, PrimaryMonitor, RawHandleWrapper, VideoMode, Window,
    WindowClosed, WindowClosing, WindowCreated, WindowEvent, WindowFocused, WindowMode,
    WindowResized, WindowWrapper,
};
use tracing::{error, info, warn};

use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    event_loop::ActiveEventLoop,
};

use bevy_app::AppExit;
use bevy_ecs::{prelude::MessageReader, query::With, system::Res};
use bevy_math::{IVec2, UVec2};
#[cfg(target_os = "ios")]
use winit::platform::ios::WindowExtIOS;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use crate::{
    accessibility::ACCESS_KIT_ADAPTERS,
    converters::{
        convert_enabled_buttons, convert_resize_direction, convert_window_level,
        convert_window_theme, convert_winit_theme,
    },
    get_selected_videomode, select_monitor,
    state::react_to_resize,
    winit_monitors::WinitMonitors,
    CreateMonitorParams, CreateWindowParams, WINIT_WINDOWS,
};

/// Creates new windows on the [`winit`] backend for each entity with a newly-added
/// [`Window`] component.
///
/// If any of these entities are missing required components, those will be added with their
/// default values.
pub fn create_windows<F: QueryFilter + 'static>(
    event_loop: &ActiveEventLoop,
    (
        mut commands,
        mut created_windows,
        mut window_created_events,
        mut handlers,
        accessibility_requested,
        monitors,
    ): SystemParamItem<CreateWindowParams<F>>,
) {
    WINIT_WINDOWS.with_borrow_mut(|winit_windows| {
        ACCESS_KIT_ADAPTERS.with_borrow_mut(|adapters| {
            for (entity, mut window, cursor_options, handle_holder) in &mut created_windows {
                if winit_windows.get_window(entity).is_some() {
                    continue;
                }

                info!("Creating new window {} ({})", window.title.as_str(), entity);

                let winit_window = winit_windows.create_window(
                    event_loop,
                    entity,
                    &window,
                    cursor_options,
                    adapters,
                    &mut handlers,
                    &accessibility_requested,
                    &monitors,
                );

                if let Some(theme) = winit_window.theme() {
                    window.window_theme = Some(convert_winit_theme(theme));
                }

                window
                    .resolution
                    .set_scale_factor_and_apply_to_physical_size(winit_window.scale_factor() as f32);

                commands.entity(entity).insert((
                    CachedWindow(window.clone()),
                    CachedCursorOptions(cursor_options.clone()),
                    WinitWindowPressedKeys::default(),
                ));

                if let Ok(handle_wrapper) = RawHandleWrapper::new(winit_window) {
                    commands.entity(entity).insert(handle_wrapper.clone());
                    if let Some(handle_holder) = handle_holder {
                        *handle_holder.0.lock().unwrap() = Some(handle_wrapper);
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if window.fit_canvas_to_parent {
                        let canvas = winit_window
                            .canvas()
                            .expect("window.canvas() can only be called in main thread.");
                        let style = canvas.style();
                        style.set_property("width", "100%").unwrap();
                        style.set_property("height", "100%").unwrap();
                    }
                }

                #[cfg(target_os = "ios")]
                {
                    winit_window.recognize_pinch_gesture(window.recognize_pinch_gesture);
                    winit_window.recognize_rotation_gesture(window.recognize_rotation_gesture);
                    winit_window.recognize_doubletap_gesture(window.recognize_doubletap_gesture);
                    if let Some((min, max)) = window.recognize_pan_gesture {
                        winit_window.recognize_pan_gesture(true, min, max);
                    } else {
                        winit_window.recognize_pan_gesture(false, 0, 0);
                    }
                }

                window_created_events.write(WindowCreated { window: entity });
            }
        });
    });
}

/// Check whether keyboard focus was lost. This is different from window
/// focus in that swapping between Bevy windows keeps window focus.
pub(crate) fn check_keyboard_focus_lost(
    mut window_focused_reader: MessageReader<WindowFocused>,
    mut keyboard_focus_lost_writer: MessageWriter<KeyboardFocusLost>,
    mut keyboard_input_writer: MessageWriter<KeyboardInput>,
    mut window_event_writer: MessageWriter<WindowEvent>,
    mut q_windows: Query<&mut WinitWindowPressedKeys>,
) {
    let mut focus_lost = vec![];
    let mut focus_gained = false;
    for e in window_focused_reader.read() {
        if e.focused {
            focus_gained = true;
        } else {
            focus_lost.push(e.window);
        }
    }

    if !focus_gained {
        if !focus_lost.is_empty() {
            window_event_writer.write(WindowEvent::KeyboardFocusLost(KeyboardFocusLost));
            keyboard_focus_lost_writer.write(KeyboardFocusLost);
        }

        for window in focus_lost {
            let Ok(mut pressed_keys) = q_windows.get_mut(window) else {
                continue;
            };
            for (key_code, logical_key) in pressed_keys.0.drain() {
                let event = KeyboardInput {
                    key_code,
                    logical_key,
                    state: bevy_input::ButtonState::Released,
                    repeat: false,
                    window,
                    text: None,
                };
                window_event_writer.write(WindowEvent::KeyboardInput(event.clone()));
                keyboard_input_writer.write(event);
            }
        }
    }
}

/// Synchronize available monitors as reported by [`winit`] with [`Monitor`] entities in the world.
pub fn create_monitors(
    event_loop: &ActiveEventLoop,
    (mut commands, mut monitors): SystemParamItem<CreateMonitorParams>,
) {
    let primary_monitor = event_loop.primary_monitor();
    let mut seen_monitors = vec![false; monitors.monitors.len()];

    'outer: for monitor in event_loop.available_monitors() {
        for (idx, (m, _)) in monitors.monitors.iter().enumerate() {
            if &monitor == m {
                seen_monitors[idx] = true;
                continue 'outer;
            }
        }

        let size = monitor.size();
        let position = monitor.position();

        let entity = commands
            .spawn(Monitor {
                name: monitor.name(),
                physical_height: size.height,
                physical_width: size.width,
                physical_position: IVec2::new(position.x, position.y),
                refresh_rate_millihertz: monitor.refresh_rate_millihertz(),
                scale_factor: monitor.scale_factor(),
                video_modes: monitor
                    .video_modes()
                    .map(|v| {
                        let size = v.size();
                        VideoMode {
                            physical_size: UVec2::new(size.width, size.height),
                            bit_depth: v.bit_depth(),
                            refresh_rate_millihertz: v.refresh_rate_millihertz(),
                        }
                    })
                    .collect(),
            })
            .id();

        if primary_monitor.as_ref() == Some(&monitor) {
            commands.entity(entity).insert(PrimaryMonitor);
        }

        seen_monitors.push(true);
        monitors.monitors.push((monitor, entity));
    }

    let mut idx = 0;
    monitors.monitors.retain(|(_m, entity)| {
        if seen_monitors[idx] {
            idx += 1;
            true
        } else {
            info!("Monitor removed {}", entity);
            commands.entity(*entity).despawn();
            idx += 1;
            false
        }
    });
}

pub(crate) fn despawn_windows(
    closing: Query<Entity, With<ClosingWindow>>,
    mut closed: RemovedComponents<Window>,
    window_entities: Query<Entity, With<Window>>,
    mut closing_event_writer: MessageWriter<WindowClosing>,
    mut closed_event_writer: MessageWriter<WindowClosed>,
    mut windows_to_drop: Local<Vec<WindowWrapper<winit::window::Window>>>,
    mut exit_event_reader: MessageReader<AppExit>,
    _non_send_marker: NonSendMarker,
) {
    // Drop all the windows that are waiting to be closed
    windows_to_drop.clear();
    for window in closing.iter() {
        closing_event_writer.write(WindowClosing { window });
    }
    for window in closed.read() {
        info!("Closing window {}", window);
        // Guard to verify that the window is in fact actually gone,
        // rather than having the component added
        // and removed in the same frame.
        if !window_entities.contains(window) {
            WINIT_WINDOWS.with_borrow_mut(|winit_windows| {
                if let Some(window) = winit_windows.remove_window(window) {
                    // Keeping WindowWrapper that are dropped for one frame
                    // Otherwise the last `Arc` of the window could be in the rendering thread, and dropped there
                    // This would hang on macOS
                    // Keeping the wrapper and dropping it next frame in this system ensure its dropped in the main thread
                    windows_to_drop.push(window);
                }
            });
            closed_event_writer.write(WindowClosed { window });
        }
    }

    // On macOS, when exiting, we need to tell the rendering thread the windows are about to
    // close to ensure that they are dropped on the main thread. Otherwise, the app will hang.
    if !exit_event_reader.is_empty() {
        exit_event_reader.clear();
        for window in window_entities.iter() {
            closing_event_writer.write(WindowClosing { window });
        }
    }
}

/// The cached state of the window so we can check which properties were changed from within the app.
#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct CachedWindow(Window);

/// The cached state of the window so we can check which properties were changed from within the app.
#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct CachedCursorOptions(CursorOptions);

/// Propagates changes from [`Window`] entities to the [`winit`] backend.
///
/// # Notes
///
/// - [`Window::present_mode`] and [`Window::composite_alpha_mode`] changes are handled by the `bevy_render` crate.
/// - [`Window::transparent`] cannot be changed after the window is created.
/// - [`Window::canvas`] cannot be changed after the window is created.
/// - [`Window::focused`] cannot be manually changed to `false` after the window is created.
pub(crate) fn changed_windows(
    mut changed_windows: Query<(Entity, &mut Window, &mut CachedWindow), Changed<Window>>,
    monitors: Res<WinitMonitors>,
    mut window_resized: MessageWriter<WindowResized>,
    _non_send_marker: NonSendMarker,
) {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        for (entity, mut window, mut cache) in &mut changed_windows {
            let Some(winit_window) = winit_windows.get_window(entity) else {
                continue;
            };

            if window.title != cache.title {
                winit_window.set_title(window.title.as_str());
            }

            if window.mode != cache.mode {
                let new_mode = match window.mode {
                    WindowMode::BorderlessFullscreen(monitor_selection) => {
                        Some(Some(winit::window::Fullscreen::Borderless(select_monitor(
                            &monitors,
                            winit_window.primary_monitor(),
                            winit_window.current_monitor(),
                            &monitor_selection,
                        ))))
                    }
                    WindowMode::Fullscreen(monitor_selection, video_mode_selection) => {
                        let monitor = &select_monitor(
                            &monitors,
                            winit_window.primary_monitor(),
                            winit_window.current_monitor(),
                            &monitor_selection,
                        )
                        .unwrap_or_else(|| {
                            panic!("Could not find monitor for {monitor_selection:?}")
                        });

                        if let Some(video_mode) = get_selected_videomode(monitor, &video_mode_selection)
                        {
                            Some(Some(winit::window::Fullscreen::Exclusive(video_mode)))
                        } else {
                            warn!(
                                "Could not find valid fullscreen video mode for {:?} {:?}",
                                monitor_selection, video_mode_selection
                            );
                            None
                        }
                    }
                    WindowMode::Windowed => Some(None),
                };

                if let Some(new_mode) = new_mode
                    && winit_window.fullscreen() != new_mode {
                        winit_window.set_fullscreen(new_mode);
                    }
            }

            if window.resolution != cache.resolution {
                let mut physical_size = PhysicalSize::new(
                    window.resolution.physical_width(),
                    window.resolution.physical_height(),
                );

                let cached_physical_size = PhysicalSize::new(
                    cache.physical_width(),
                    cache.physical_height(),
                );

                let base_scale_factor = window.resolution.base_scale_factor();

                // Note: this may be different from `winit`'s base scale factor if
                // `scale_factor_override` is set to Some(f32)
                let scale_factor = window.scale_factor();
                let cached_scale_factor = cache.scale_factor();

                // Check and update `winit`'s physical size only if the window is not maximized
                if scale_factor != cached_scale_factor && !winit_window.is_maximized() {
                    let logical_size =
                        if let Some(cached_factor) = cache.resolution.scale_factor_override() {
                            physical_size.to_logical::<f32>(cached_factor as f64)
                        } else {
                            physical_size.to_logical::<f32>(base_scale_factor as f64)
                        };

                    // Scale factor changed, updating physical and logical size
                    if let Some(forced_factor) = window.resolution.scale_factor_override() {
                        // This window is overriding the OS-suggested DPI, so its physical size
                        // should be set based on the overriding value. Its logical size already
                        // incorporates any resize constraints.
                        physical_size = logical_size.to_physical::<u32>(forced_factor as f64);
                    } else {
                        physical_size = logical_size.to_physical::<u32>(base_scale_factor as f64);
                    }
                }

                if physical_size != cached_physical_size
                    && let Some(new_physical_size) = winit_window.request_inner_size(physical_size) {
                        react_to_resize(entity, &mut window, new_physical_size, &mut window_resized);
                    }
            }

            if window.physical_cursor_position() != cache.physical_cursor_position()
                && let Some(physical_position) = window.physical_cursor_position() {
                    let position = PhysicalPosition::new(physical_position.x, physical_position.y);

                    if let Err(err) = winit_window.set_cursor_position(position) {
                        error!("could not set cursor position: {}", err);
                    }
                }

            if window.decorations != cache.decorations
                && window.decorations != winit_window.is_decorated()
            {
                winit_window.set_decorations(window.decorations);
            }

            if window.resizable != cache.resizable
                && window.resizable != winit_window.is_resizable()
            {
                winit_window.set_resizable(window.resizable);
            }

            if window.enabled_buttons != cache.enabled_buttons {
                winit_window.set_enabled_buttons(convert_enabled_buttons(window.enabled_buttons));
            }

            if window.resize_constraints != cache.resize_constraints {
                let constraints = window.resize_constraints.check_constraints();
                let min_inner_size = LogicalSize {
                    width: constraints.min_width,
                    height: constraints.min_height,
                };
                let max_inner_size = LogicalSize {
                    width: constraints.max_width,
                    height: constraints.max_height,
                };

                winit_window.set_min_inner_size(Some(min_inner_size));
                if constraints.max_width.is_finite() && constraints.max_height.is_finite() {
                    winit_window.set_max_inner_size(Some(max_inner_size));
                }
            }

            if window.position != cache.position
                && let Some(position) = crate::winit_window_position(
                    &window.position,
                    &window.resolution,
                    &monitors,
                    winit_window.primary_monitor(),
                    winit_window.current_monitor(),
                ) {
                    let should_set = match winit_window.outer_position() {
                        Ok(current_position) => current_position != position,
                        _ => true,
                    };

                    if should_set {
                        winit_window.set_outer_position(position);
                    }
                }

            if let Some(maximized) = window.internal.take_maximize_request() {
                winit_window.set_maximized(maximized);
            }

            if let Some(minimized) = window.internal.take_minimize_request() {
                winit_window.set_minimized(minimized);
            }

            if window.internal.take_move_request()
                && let Err(e) = winit_window.drag_window() {
                    warn!("Winit returned an error while attempting to drag the window: {e}");
                }

            if let Some(resize_direction) = window.internal.take_resize_request()
                && let Err(e) =
                    winit_window.drag_resize_window(convert_resize_direction(resize_direction))
                {
                    warn!("Winit returned an error while attempting to drag resize the window: {e}");
                }

            if window.focused != cache.focused && window.focused {
                winit_window.focus_window();
            }

            if window.window_level != cache.window_level {
                winit_window.set_window_level(convert_window_level(window.window_level));
            }

            // Currently unsupported changes
            if window.transparent != cache.transparent {
                window.transparent = cache.transparent;
                warn!("Winit does not currently support updating transparency after window creation.");
            }

            #[cfg(target_arch = "wasm32")]
            if window.canvas != cache.canvas {
                window.canvas.clone_from(&cache.canvas);
                warn!(
                    "Bevy currently doesn't support modifying the window canvas after initialization."
                );
            }

            if window.ime_enabled != cache.ime_enabled {
                winit_window.set_ime_allowed(window.ime_enabled);
            }

            if window.ime_position != cache.ime_position {
                winit_window.set_ime_cursor_area(
                    LogicalPosition::new(window.ime_position.x, window.ime_position.y),
                    PhysicalSize::new(10, 10),
                );
            }

            if window.window_theme != cache.window_theme {
                winit_window.set_theme(window.window_theme.map(convert_window_theme));
            }

            if window.visible != cache.visible {
                winit_window.set_visible(window.visible);
            }

            #[cfg(target_os = "ios")]
            {
                if window.recognize_pinch_gesture != cache.recognize_pinch_gesture {
                    winit_window.recognize_pinch_gesture(window.recognize_pinch_gesture);
                }
                if window.recognize_rotation_gesture != cache.recognize_rotation_gesture {
                    winit_window.recognize_rotation_gesture(window.recognize_rotation_gesture);
                }
                if window.recognize_doubletap_gesture != cache.recognize_doubletap_gesture {
                    winit_window.recognize_doubletap_gesture(window.recognize_doubletap_gesture);
                }
                if window.recognize_pan_gesture != cache.recognize_pan_gesture {
                    match (
                        window.recognize_pan_gesture,
                        cache.recognize_pan_gesture,
                    ) {
                        (Some(_), Some(_)) => {
                            warn!("Bevy currently doesn't support modifying PanGesture number of fingers recognition. Please disable it before re-enabling it with the new number of fingers");
                        }
                        (Some((min, max)), _) => winit_window.recognize_pan_gesture(true, min, max),
                        _ => winit_window.recognize_pan_gesture(false, 0, 0),
                    }
                }

                if window.prefers_home_indicator_hidden != cache.prefers_home_indicator_hidden {
                    winit_window
                        .set_prefers_home_indicator_hidden(window.prefers_home_indicator_hidden);
                }
                if window.prefers_status_bar_hidden != cache.prefers_status_bar_hidden {
                    winit_window.set_prefers_status_bar_hidden(window.prefers_status_bar_hidden);
                }
                if window.preferred_screen_edges_deferring_system_gestures
                    != cache
                        .preferred_screen_edges_deferring_system_gestures
                {
                    use crate::converters::convert_screen_edge;
                    let preferred_edge =
                        convert_screen_edge(window.preferred_screen_edges_deferring_system_gestures);
                    winit_window.set_preferred_screen_edges_deferring_system_gestures(preferred_edge);
                }
            }
            **cache = window.clone();
        }
    });
}

pub(crate) fn changed_cursor_options(
    mut changed_windows: Query<
        (
            Entity,
            &Window,
            &mut CursorOptions,
            &mut CachedCursorOptions,
        ),
        Changed<CursorOptions>,
    >,
    _non_send_marker: NonSendMarker,
) {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        for (entity, window, mut cursor_options, mut cache) in &mut changed_windows {
            // This system already only runs when the cursor options change, so we need to bypass change detection or the next frame will also run this system
            let cursor_options = cursor_options.bypass_change_detection();
            let Some(winit_window) = winit_windows.get_window(entity) else {
                continue;
            };
            // Don't check the cache for the grab mode. It can change through external means, leaving the cache outdated.
            if let Err(err) =
                crate::winit_windows::attempt_grab(winit_window, cursor_options.grab_mode)
            {
                warn!(
                    "Could not set cursor grab mode for window {}: {}",
                    window.title, err
                );
                cursor_options.grab_mode = cache.grab_mode;
            } else {
                cache.grab_mode = cursor_options.grab_mode;
            }

            if cursor_options.visible != cache.visible {
                winit_window.set_cursor_visible(cursor_options.visible);
                cache.visible = cursor_options.visible;
            }

            if cursor_options.hit_test != cache.hit_test {
                if let Err(err) = winit_window.set_cursor_hittest(cursor_options.hit_test) {
                    warn!(
                        "Could not set cursor hit test for window {}: {}",
                        window.title, err
                    );
                    cursor_options.hit_test = cache.hit_test;
                } else {
                    cache.hit_test = cursor_options.hit_test;
                }
            }
        }
    });
}

/// This keeps track of which keys are pressed on each window.
/// When a window is unfocused, this is used to send key release events for all the currently held keys.
#[derive(Default, Component)]
pub struct WinitWindowPressedKeys(pub(crate) HashMap<KeyCode, Key>);
