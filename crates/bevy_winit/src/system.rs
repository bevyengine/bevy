use bevy_ecs::{
    entity::Entity,
    event::EventWriter,
    prelude::{Changed, Component},
    query::QueryFilter,
    removal_detection::RemovedComponents,
    system::{Local, NonSendMut, Query, SystemParamItem},
};
use bevy_utils::tracing::{error, info, warn};
use bevy_window::{
    ClosingWindow, RawHandleWrapper, Window, WindowClosed, WindowClosing, WindowCreated,
    WindowMode, WindowResized, WindowWrapper,
};

use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;

use bevy_app::AppExit;
use bevy_ecs::prelude::EventReader;
use bevy_ecs::query::With;
#[cfg(target_os = "ios")]
use winit::platform::ios::WindowExtIOS;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use crate::state::react_to_resize;
use crate::{
    converters::{
        self, convert_enabled_buttons, convert_window_level, convert_window_theme,
        convert_winit_theme,
    },
    get_best_videomode, get_fitting_videomode, CreateWindowParams, WinitWindows,
};

/// Creates new windows on the [`winit`] backend for each entity with a newly-added
/// [`Window`] component.
///
/// If any of these entities are missing required components, those will be added with their
/// default values.
#[allow(clippy::too_many_arguments)]
pub fn create_windows<F: QueryFilter + 'static>(
    event_loop: &ActiveEventLoop,
    (
        mut commands,
        mut created_windows,
        mut window_created_events,
        mut winit_windows,
        mut adapters,
        mut handlers,
        accessibility_requested,
    ): SystemParamItem<CreateWindowParams<F>>,
) {
    for (entity, mut window, handle_holder) in &mut created_windows {
        if winit_windows.get_window(entity).is_some() {
            continue;
        }

        info!(
            "Creating new window {:?} ({:?})",
            window.title.as_str(),
            entity
        );

        let winit_window = winit_windows.create_window(
            event_loop,
            entity,
            &window,
            &mut adapters,
            &mut handlers,
            &accessibility_requested,
        );

        if let Some(theme) = winit_window.theme() {
            window.window_theme = Some(convert_winit_theme(theme));
        }

        window
            .resolution
            .set_scale_factor_and_apply_to_physical_size(winit_window.scale_factor() as f32);

        commands.entity(entity).insert(CachedWindow {
            window: window.clone(),
        });

        if let Ok(handle_wrapper) = RawHandleWrapper::new(winit_window) {
            let mut entity = commands.entity(entity);
            entity.insert(handle_wrapper.clone());
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

        window_created_events.send(WindowCreated { window: entity });
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn despawn_windows(
    closing: Query<Entity, With<ClosingWindow>>,
    mut closed: RemovedComponents<Window>,
    window_entities: Query<Entity, With<Window>>,
    mut closing_events: EventWriter<WindowClosing>,
    mut closed_events: EventWriter<WindowClosed>,
    mut winit_windows: NonSendMut<WinitWindows>,
    mut windows_to_drop: Local<Vec<WindowWrapper<winit::window::Window>>>,
    mut exit_events: EventReader<AppExit>,
) {
    // Drop all the windows that are waiting to be closed
    windows_to_drop.clear();
    for window in closing.iter() {
        closing_events.send(WindowClosing { window });
    }
    for window in closed.read() {
        info!("Closing window {:?}", window);
        // Guard to verify that the window is in fact actually gone,
        // rather than having the component added
        // and removed in the same frame.
        if !window_entities.contains(window) {
            if let Some(window) = winit_windows.remove_window(window) {
                // Keeping WindowWrapper that are dropped for one frame
                // Otherwise the last `Arc` of the window could be in the rendering thread, and dropped there
                // This would hang on macOS
                // Keeping the wrapper and dropping it next frame in this system ensure its dropped in the main thread
                windows_to_drop.push(window);
            }
            closed_events.send(WindowClosed { window });
        }
    }

    // On macOS, when exiting, we need to tell the rendering thread the windows are about to
    // close to ensure that they are dropped on the main thread. Otherwise, the app will hang.
    if !exit_events.is_empty() {
        exit_events.clear();
        for window in window_entities.iter() {
            closing_events.send(WindowClosing { window });
        }
    }
}

/// The cached state of the window so we can check which properties were changed from within the app.
#[derive(Debug, Clone, Component)]
pub struct CachedWindow {
    pub window: Window,
}

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
    winit_windows: NonSendMut<WinitWindows>,
    mut window_resized: EventWriter<WindowResized>,
) {
    for (entity, mut window, mut cache) in &mut changed_windows {
        let Some(winit_window) = winit_windows.get_window(entity) else {
            continue;
        };

        if window.title != cache.window.title {
            winit_window.set_title(window.title.as_str());
        }

        if window.mode != cache.window.mode {
            let new_mode = match window.mode {
                WindowMode::BorderlessFullscreen => {
                    Some(Some(winit::window::Fullscreen::Borderless(None)))
                }
                mode @ (WindowMode::Fullscreen | WindowMode::SizedFullscreen) => {
                    if let Some(current_monitor) = winit_window.current_monitor() {
                        let videomode = match mode {
                            WindowMode::Fullscreen => get_best_videomode(&current_monitor),
                            WindowMode::SizedFullscreen => get_fitting_videomode(
                                &current_monitor,
                                window.width() as u32,
                                window.height() as u32,
                            ),
                            _ => unreachable!(),
                        };

                        Some(Some(winit::window::Fullscreen::Exclusive(videomode)))
                    } else {
                        warn!("Could not determine current monitor, ignoring exclusive fullscreen request for window {:?}", window.title);
                        None
                    }
                }
                WindowMode::Windowed => Some(None),
            };

            if let Some(new_mode) = new_mode {
                if winit_window.fullscreen() != new_mode {
                    winit_window.set_fullscreen(new_mode);
                }
            }
        }

        if window.resolution != cache.window.resolution {
            let mut physical_size = PhysicalSize::new(
                window.resolution.physical_width(),
                window.resolution.physical_height(),
            );

            let cached_physical_size = PhysicalSize::new(
                cache.window.physical_width(),
                cache.window.physical_height(),
            );

            let base_scale_factor = window.resolution.base_scale_factor();

            // Note: this may be different from `winit`'s base scale factor if
            // `scale_factor_override` is set to Some(f32)
            let scale_factor = window.scale_factor();
            let cached_scale_factor = cache.window.scale_factor();

            // Check and update `winit`'s physical size only if the window is not maximized
            if scale_factor != cached_scale_factor && !winit_window.is_maximized() {
                let logical_size =
                    if let Some(cached_factor) = cache.window.resolution.scale_factor_override() {
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

            if physical_size != cached_physical_size {
                if let Some(new_physical_size) = winit_window.request_inner_size(physical_size) {
                    react_to_resize(entity, &mut window, new_physical_size, &mut window_resized);
                }
            }
        }

        if window.physical_cursor_position() != cache.window.physical_cursor_position() {
            if let Some(physical_position) = window.physical_cursor_position() {
                let position = PhysicalPosition::new(physical_position.x, physical_position.y);

                if let Err(err) = winit_window.set_cursor_position(position) {
                    error!("could not set cursor position: {:?}", err);
                }
            }
        }

        if window.cursor.icon != cache.window.cursor.icon {
            winit_window.set_cursor(converters::convert_cursor_icon(window.cursor.icon));
        }

        if window.cursor.grab_mode != cache.window.cursor.grab_mode {
            crate::winit_windows::attempt_grab(winit_window, window.cursor.grab_mode);
        }

        if window.cursor.visible != cache.window.cursor.visible {
            winit_window.set_cursor_visible(window.cursor.visible);
        }

        if window.cursor.hit_test != cache.window.cursor.hit_test {
            if let Err(err) = winit_window.set_cursor_hittest(window.cursor.hit_test) {
                window.cursor.hit_test = cache.window.cursor.hit_test;
                warn!(
                    "Could not set cursor hit test for window {:?}: {:?}",
                    window.title, err
                );
            }
        }

        if window.decorations != cache.window.decorations
            && window.decorations != winit_window.is_decorated()
        {
            winit_window.set_decorations(window.decorations);
        }

        if window.resizable != cache.window.resizable
            && window.resizable != winit_window.is_resizable()
        {
            winit_window.set_resizable(window.resizable);
        }

        if window.enabled_buttons != cache.window.enabled_buttons {
            winit_window.set_enabled_buttons(convert_enabled_buttons(window.enabled_buttons));
        }

        if window.resize_constraints != cache.window.resize_constraints {
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

        if window.position != cache.window.position {
            if let Some(position) = crate::winit_window_position(
                &window.position,
                &window.resolution,
                winit_window.available_monitors(),
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
        }

        if let Some(maximized) = window.internal.take_maximize_request() {
            winit_window.set_maximized(maximized);
        }

        if let Some(minimized) = window.internal.take_minimize_request() {
            winit_window.set_minimized(minimized);
        }

        if window.focused != cache.window.focused && window.focused {
            winit_window.focus_window();
        }

        if window.window_level != cache.window.window_level {
            winit_window.set_window_level(convert_window_level(window.window_level));
        }

        // Currently unsupported changes
        if window.transparent != cache.window.transparent {
            window.transparent = cache.window.transparent;
            warn!("Winit does not currently support updating transparency after window creation.");
        }

        #[cfg(target_arch = "wasm32")]
        if window.canvas != cache.window.canvas {
            window.canvas.clone_from(&cache.window.canvas);
            warn!(
                "Bevy currently doesn't support modifying the window canvas after initialization."
            );
        }

        if window.ime_enabled != cache.window.ime_enabled {
            winit_window.set_ime_allowed(window.ime_enabled);
        }

        if window.ime_position != cache.window.ime_position {
            winit_window.set_ime_cursor_area(
                LogicalPosition::new(window.ime_position.x, window.ime_position.y),
                PhysicalSize::new(10, 10),
            );
        }

        if window.window_theme != cache.window.window_theme {
            winit_window.set_theme(window.window_theme.map(convert_window_theme));
        }

        if window.visible != cache.window.visible {
            winit_window.set_visible(window.visible);
        }

        #[cfg(target_os = "ios")]
        {
            if window.recognize_pinch_gesture != cache.window.recognize_pinch_gesture {
                winit_window.recognize_pinch_gesture(window.recognize_pinch_gesture);
            }
            if window.recognize_rotation_gesture != cache.window.recognize_rotation_gesture {
                winit_window.recognize_rotation_gesture(window.recognize_rotation_gesture);
            }
            if window.recognize_doubletap_gesture != cache.window.recognize_doubletap_gesture {
                winit_window.recognize_doubletap_gesture(window.recognize_doubletap_gesture);
            }
            if window.recognize_pan_gesture != cache.window.recognize_pan_gesture {
                match (
                    window.recognize_pan_gesture,
                    cache.window.recognize_pan_gesture,
                ) {
                    (Some(_), Some(_)) => {
                        warn!("Bevy currently doesn't support modifying PanGesture number of fingers recognition. Please disable it before re-enabling it with the new number of fingers");
                    }
                    (Some((min, max)), _) => winit_window.recognize_pan_gesture(true, min, max),
                    _ => winit_window.recognize_pan_gesture(false, 0, 0),
                }
            }
        }

        cache.window = window.clone();
    }
}
