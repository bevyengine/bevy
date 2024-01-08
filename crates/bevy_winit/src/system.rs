use bevy_a11y::AccessibilityRequested;
use bevy_ecs::{
    entity::Entity,
    event::EventWriter,
    prelude::{Changed, Component, Resource},
    removal_detection::RemovedComponents,
    system::{Commands, NonSendMut, Query, ResMut},
    world::Mut,
};
use bevy_utils::{
    tracing::{error, info, warn},
    EntityHashMap,
};
use bevy_window::{RawHandleWrapper, Window, WindowClosed, WindowCreated};

use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    event_loop::EventLoopWindowTarget,
};

use crate::{
    accessibility::{AccessKitAdapters, WinitActionHandlers},
    converters::{
        self, convert_enabled_buttons, convert_window_level, convert_window_theme,
        convert_winit_theme,
    },
    get_best_videomode, get_fitting_videomode, WindowAndInputEventWriters, WinitWindows,
};

/// Creates new windows on the [`winit`] backend for each entity with a newly-added
/// [`Window`] component.
///
/// If any of these entities are missing required components, those will be added with their
/// default values.
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_windows<'a>(
    event_loop: &EventLoopWindowTarget<()>,
    mut commands: Commands,
    created_windows: impl Iterator<Item = (Entity, Mut<'a, Window>)>,
    mut event_writer: EventWriter<WindowCreated>,
    mut winit_windows: NonSendMut<WinitWindows>,
    mut adapters: NonSendMut<AccessKitAdapters>,
    mut handlers: ResMut<WinitActionHandlers>,
    accessibility_requested: ResMut<AccessibilityRequested>,
) {
    for (entity, mut window) in created_windows {
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
            .set_scale_factor(winit_window.scale_factor() as f32);
        commands
            .entity(entity)
            .insert(RawHandleWrapper {
                window_handle: winit_window.window_handle().unwrap().as_raw(),
                display_handle: winit_window.display_handle().unwrap().as_raw(),
            })
            .insert(CachedWindow {
                window: window.clone(),
            });

        event_writer.send(WindowCreated { window: entity });
    }
}

/// Cache for closing windows so we can get better debug information.
#[derive(Debug, Clone, Resource)]
pub struct WindowTitleCache(EntityHashMap<Entity, String>);

pub(crate) fn despawn_windows(
    mut closed: RemovedComponents<Window>,
    window_entities: Query<&Window>,
    mut close_events: EventWriter<WindowClosed>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    for window in closed.read() {
        info!("Closing window {:?}", window);
        // Guard to verify that the window is in fact actually gone,
        // rather than having the component added and removed in the same frame.
        if !window_entities.contains(window) {
            winit_windows.remove_window(window);
            close_events.send(WindowClosed { window });
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
    mut event_writers: WindowAndInputEventWriters<'_>,
) {
    for (entity, mut window, mut cache) in &mut changed_windows {
        if let Some(winit_window) = winit_windows.get_window(entity) {
            if window.title != cache.window.title {
                winit_window.set_title(window.title.as_str());
            }

            if window.mode != cache.window.mode {
                let new_mode = match window.mode {
                    bevy_window::WindowMode::BorderlessFullscreen => {
                        Some(winit::window::Fullscreen::Borderless(None))
                    }
                    bevy_window::WindowMode::Fullscreen => {
                        Some(winit::window::Fullscreen::Exclusive(get_best_videomode(
                            &winit_window.current_monitor().unwrap(),
                        )))
                    }
                    bevy_window::WindowMode::SizedFullscreen => {
                        Some(winit::window::Fullscreen::Exclusive(get_fitting_videomode(
                            &winit_window.current_monitor().unwrap(),
                            window.width() as u32,
                            window.height() as u32,
                        )))
                    }
                    bevy_window::WindowMode::Windowed => None,
                };

                if winit_window.fullscreen() != new_mode {
                    winit_window.set_fullscreen(new_mode);
                }
            }
            if window.resolution != cache.window.resolution {
                let physical_size = PhysicalSize::new(
                    window.resolution.physical_width(),
                    window.resolution.physical_height(),
                );
                if let Some(size_now) = winit_window.request_inner_size(physical_size) {
                    crate::react_to_resize(&mut window, size_now, &mut event_writers, entity);
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
                winit_window.set_cursor_icon(converters::convert_cursor_icon(window.cursor.icon));
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
                warn!(
                    "Winit does not currently support updating transparency after window creation."
                );
            }

            #[cfg(target_arch = "wasm32")]
            if window.canvas != cache.window.canvas {
                window.canvas = cache.window.canvas.clone();
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

            cache.window = window.clone();
        }
    }
}
