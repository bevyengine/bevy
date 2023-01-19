use bevy_ecs::{
    entity::Entity,
    event::EventWriter,
    prelude::{Changed, Component, Resource},
    system::{Commands, NonSendMut, Query, RemovedComponents},
    world::Mut,
};
use bevy_utils::{
    tracing::{error, info, warn},
    HashMap,
};
use bevy_window::{RawHandleWrapper, Window, WindowClosed, WindowCreated};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use winit::{
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event_loop::EventLoopWindowTarget,
};

#[cfg(target_arch = "wasm32")]
use crate::web_resize::{CanvasParentResizeEventChannel, WINIT_CANVAS_SELECTOR};
use crate::{converters, get_best_videomode, get_fitting_videomode, WinitWindows};
#[cfg(target_arch = "wasm32")]
use bevy_ecs::system::ResMut;

/// System responsible for creating new windows whenever a `Window` component is added
/// to an entity.
///
/// This will default any necessary components if they are not already added.
pub(crate) fn create_window<'a>(
    mut commands: Commands,
    event_loop: &EventLoopWindowTarget<()>,
    created_windows: impl Iterator<Item = (Entity, Mut<'a, Window>)>,
    mut event_writer: EventWriter<WindowCreated>,
    mut winit_windows: NonSendMut<WinitWindows>,
    #[cfg(target_arch = "wasm32")] event_channel: ResMut<CanvasParentResizeEventChannel>,
) {
    for (entity, mut component) in created_windows {
        if winit_windows.get_window(entity).is_some() {
            continue;
        }

        info!(
            "Creating new window {:?} ({:?})",
            component.title.as_str(),
            entity
        );

        let winit_window = winit_windows.create_window(event_loop, entity, &component);
        let current_size = winit_window.inner_size();
        component
            .resolution
            .set_scale_factor(winit_window.scale_factor());
        commands
            .entity(entity)
            .insert(RawHandleWrapper {
                window_handle: winit_window.raw_window_handle(),
                display_handle: winit_window.raw_display_handle(),
            })
            .insert(WinitWindowInfo {
                previous: component.clone(),
                last_winit_size: PhysicalSize {
                    width: current_size.width,
                    height: current_size.height,
                },
            });

        #[cfg(target_arch = "wasm32")]
        {
            if component.fit_canvas_to_parent {
                let selector = if let Some(selector) = &component.canvas {
                    selector
                } else {
                    WINIT_CANVAS_SELECTOR
                };
                event_channel.listen_to_selector(entity, selector);
            }
        }

        event_writer.send(WindowCreated { window: entity });
    }
}

/// Cache for closing windows so we can get better debug information.
#[derive(Debug, Clone, Resource)]
pub struct WindowTitleCache(HashMap<Entity, String>);

pub(crate) fn despawn_window(
    closed: RemovedComponents<Window>,
    mut close_events: EventWriter<WindowClosed>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    for window in closed.iter() {
        info!("Closing window {:?}", window);

        winit_windows.remove_window(window);
        close_events.send(WindowClosed { window });
    }
}

/// Previous state of the window so we can check sub-portions of what actually was changed.
#[derive(Debug, Clone, Component)]
pub struct WinitWindowInfo {
    pub previous: Window,
    pub last_winit_size: PhysicalSize<u32>,
}

// Detect changes to the window and update the winit window accordingly.
//
// Notes:
// - [`Window::present_mode`] and [`Window::composite_alpha_mode`] updating should be handled in the bevy render crate.
// - [`Window::transparent`] currently cannot be updated after startup for winit.
// - [`Window::canvas`] currently cannot be updated after startup, not entirely sure if it would work well with the
//   event channel stuff.
pub(crate) fn changed_window(
    mut changed_windows: Query<(Entity, &mut Window, &mut WinitWindowInfo), Changed<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
) {
    for (entity, mut window, mut info) in &mut changed_windows {
        let previous = &info.previous;

        if let Some(winit_window) = winit_windows.get_window(entity) {
            if window.title != previous.title {
                winit_window.set_title(window.title.as_str());
            }

            if window.mode != previous.mode {
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
            if window.resolution != previous.resolution {
                let physical_size = PhysicalSize::new(
                    window.resolution.physical_width(),
                    window.resolution.physical_height(),
                );
                // Prevents "window.resolution values set from a winit resize event" from
                // being set here, creating feedback loops.
                if physical_size != info.last_winit_size {
                    winit_window.set_inner_size(physical_size);
                }
            }

            if window.cursor.position != previous.cursor.position {
                if let Some(physical_position) = window.cursor.position {
                    let inner_size = winit_window.inner_size();

                    let position = PhysicalPosition::new(
                        physical_position.x,
                        // Flip the coordinate space back to winit's context.
                        inner_size.height as f64 - physical_position.y,
                    );

                    if let Err(err) = winit_window.set_cursor_position(position) {
                        error!("could not set cursor position: {:?}", err);
                    }
                }
            }

            if window.cursor.icon != previous.cursor.icon {
                winit_window.set_cursor_icon(converters::convert_cursor_icon(window.cursor.icon));
            }

            if window.cursor.grab_mode != previous.cursor.grab_mode {
                crate::winit_windows::attempt_grab(winit_window, window.cursor.grab_mode);
            }

            if window.cursor.visible != previous.cursor.visible {
                winit_window.set_cursor_visible(window.cursor.visible);
            }

            if window.cursor.hit_test != previous.cursor.hit_test {
                if let Err(err) = winit_window.set_cursor_hittest(window.cursor.hit_test) {
                    window.cursor.hit_test = previous.cursor.hit_test;
                    warn!(
                        "Could not set cursor hit test for window {:?}: {:?}",
                        window.title, err
                    );
                }
            }

            if window.decorations != previous.decorations
                && window.decorations != winit_window.is_decorated()
            {
                winit_window.set_decorations(window.decorations);
            }

            if window.resizable != previous.resizable
                && window.resizable != winit_window.is_resizable()
            {
                winit_window.set_resizable(window.resizable);
            }

            if window.resize_constraints != previous.resize_constraints {
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

            if window.position != previous.position {
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

            if window.focused != previous.focused && window.focused {
                winit_window.focus_window();
            }

            if window.always_on_top != previous.always_on_top {
                winit_window.set_always_on_top(window.always_on_top);
            }

            // Currently unsupported changes
            if window.transparent != previous.transparent {
                window.transparent = previous.transparent;
                warn!(
                    "Winit does not currently support updating transparency after window creation."
                );
            }

            #[cfg(target_arch = "wasm32")]
            if window.canvas != previous.canvas {
                window.canvas = previous.canvas.clone();
                warn!(
                    "Bevy currently doesn't support modifying the window canvas after initialization."
                );
            }

            info.previous = window.clone();
        }
    }
}
