#![warn(missing_docs)]

use accesskit_winit::Adapter;
use bevy_a11y::{
    accesskit::{NodeBuilder, NodeClassSet, NodeId, Role, Tree, TreeUpdate},
    AccessibilityRequested,
};
use bevy_ecs::entity::Entity;

use bevy_utils::{tracing::warn, HashMap};
use bevy_window::{CursorGrabMode, Window, WindowMode, WindowPosition, WindowResolution};

use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    monitor::MonitorHandle,
};

use crate::{
    accessibility::{AccessKitAdapters, WinitActionHandler, WinitActionHandlers},
    converters::{convert_enabled_buttons, convert_window_level, convert_window_theme},
};

/// A resource mapping window entities to their `winit`-backend [`Window`](winit::window::Window)
/// states.
#[derive(Debug, Default)]
pub struct WinitWindows {
    /// Stores [`winit`] windows by window identifier.
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    /// Maps entities to `winit` window identifiers.
    pub entity_to_winit: HashMap<Entity, winit::window::WindowId>,
    /// Maps `winit` window identifiers to entities.
    pub winit_to_entity: HashMap<winit::window::WindowId, Entity>,
    // Many `winit` window functions (e.g. `set_window_icon`) can only be called on the main thread.
    // If they're called on other threads, the program might hang. This marker indicates that this
    // type is not thread-safe and will be `!Send` and `!Sync`.
    _not_send_sync: core::marker::PhantomData<*const ()>,
}

impl WinitWindows {
    /// Creates a `winit` window and associates it with our entity.
    pub fn create_window(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        entity: Entity,
        window: &Window,
        adapters: &mut AccessKitAdapters,
        handlers: &mut WinitActionHandlers,
        accessibility_requested: &AccessibilityRequested,
    ) -> &winit::window::Window {
        let mut winit_window_builder = winit::window::WindowBuilder::new();

        // Due to a UIA limitation, winit windows need to be invisible for the
        // AccessKit adapter is initialized.
        winit_window_builder = winit_window_builder.with_visible(false);

        winit_window_builder = match window.mode {
            WindowMode::BorderlessFullscreen => winit_window_builder.with_fullscreen(Some(
                winit::window::Fullscreen::Borderless(event_loop.primary_monitor()),
            )),
            WindowMode::Fullscreen => {
                winit_window_builder.with_fullscreen(Some(winit::window::Fullscreen::Exclusive(
                    get_best_videomode(&event_loop.primary_monitor().unwrap()),
                )))
            }
            WindowMode::SizedFullscreen => winit_window_builder.with_fullscreen(Some(
                winit::window::Fullscreen::Exclusive(get_fitting_videomode(
                    &event_loop.primary_monitor().unwrap(),
                    window.width() as u32,
                    window.height() as u32,
                )),
            )),
            WindowMode::Windowed => {
                if let Some(position) = winit_window_position(
                    &window.position,
                    &window.resolution,
                    event_loop.available_monitors(),
                    event_loop.primary_monitor(),
                    None,
                ) {
                    winit_window_builder = winit_window_builder.with_position(position);
                }

                let logical_size = LogicalSize::new(window.width(), window.height());
                if let Some(sf) = window.resolution.scale_factor_override() {
                    winit_window_builder.with_inner_size(logical_size.to_physical::<f64>(sf))
                } else {
                    winit_window_builder.with_inner_size(logical_size)
                }
            }
        };

        winit_window_builder = winit_window_builder
            .with_window_level(convert_window_level(window.window_level))
            .with_theme(window.window_theme.map(convert_window_theme))
            .with_resizable(window.resizable)
            .with_enabled_buttons(convert_enabled_buttons(window.enabled_buttons))
            .with_decorations(window.decorations)
            .with_transparent(window.transparent)
            .with_visible(window.visible);

        let constraints = window.resize_constraints.check_constraints();
        let min_inner_size = LogicalSize {
            width: constraints.min_width,
            height: constraints.min_height,
        };
        let max_inner_size = LogicalSize {
            width: constraints.max_width,
            height: constraints.max_height,
        };

        let winit_window_builder =
            if constraints.max_width.is_finite() && constraints.max_height.is_finite() {
                winit_window_builder
                    .with_min_inner_size(min_inner_size)
                    .with_max_inner_size(max_inner_size)
            } else {
                winit_window_builder.with_min_inner_size(min_inner_size)
            };

        #[allow(unused_mut)]
        let mut winit_window_builder = winit_window_builder.with_title(window.title.as_str());

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowBuilderExtWebSys;

            if let Some(selector) = &window.canvas {
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let canvas = document
                    .query_selector(&selector)
                    .expect("Cannot query for canvas element.");
                if let Some(canvas) = canvas {
                    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok();
                    winit_window_builder = winit_window_builder.with_canvas(canvas);
                } else {
                    panic!("Cannot find element: {}.", selector);
                }
            }

            winit_window_builder =
                winit_window_builder.with_prevent_default(window.prevent_default_event_handling)
        }

        let winit_window = winit_window_builder.build(event_loop).unwrap();
        let name = window.title.clone();

        let mut root_builder = NodeBuilder::new(Role::Window);
        root_builder.set_name(name.into_boxed_str());
        let root = root_builder.build(&mut NodeClassSet::lock_global());

        let accesskit_window_id = NodeId(entity.to_bits());
        let handler = WinitActionHandler::default();
        let accessibility_requested = accessibility_requested.clone();
        let adapter = Adapter::with_action_handler(
            &winit_window,
            move || {
                accessibility_requested.set(true);
                TreeUpdate {
                    nodes: vec![(accesskit_window_id, root)],
                    tree: Some(Tree::new(accesskit_window_id)),
                    focus: accesskit_window_id,
                }
            },
            Box::new(handler.clone()),
        );
        adapters.insert(entity, adapter);
        handlers.insert(entity, handler);

        // Do not set the grab mode on window creation if it's none. It can fail on mobile.
        if window.cursor.grab_mode != CursorGrabMode::None {
            attempt_grab(&winit_window, window.cursor.grab_mode);
        }

        winit_window.set_cursor_visible(window.cursor.visible);

        // Do not set the cursor hittest on window creation if it's false, as it will always fail on
        // some platforms and log an unfixable warning.
        if !window.cursor.hit_test {
            if let Err(err) = winit_window.set_cursor_hittest(window.cursor.hit_test) {
                warn!(
                    "Could not set cursor hit test for window {:?}: {:?}",
                    window.title, err
                );
            }
        }

        self.entity_to_winit.insert(entity, winit_window.id());
        self.winit_to_entity.insert(winit_window.id(), entity);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            if window.canvas.is_none() {
                let canvas = winit_window.canvas();

                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let body = document.body().unwrap();

                body.append_child(&canvas)
                    .expect("Append canvas to HTML body.");
            }
        }

        self.windows
            .entry(winit_window.id())
            .insert(winit_window)
            .into_mut()
    }

    /// Get the winit window that is associated with our entity.
    pub fn get_window(&self, entity: Entity) -> Option<&winit::window::Window> {
        self.entity_to_winit
            .get(&entity)
            .and_then(|winit_id| self.windows.get(winit_id))
    }

    /// Get the entity associated with the winit window id.
    ///
    /// This is mostly just an intermediary step between us and winit.
    pub fn get_window_entity(&self, winit_id: winit::window::WindowId) -> Option<Entity> {
        self.winit_to_entity.get(&winit_id).cloned()
    }

    /// Remove a window from winit.
    ///
    /// This should mostly just be called when the window is closing.
    pub fn remove_window(&mut self, entity: Entity) -> Option<winit::window::Window> {
        let winit_id = self.entity_to_winit.remove(&entity)?;
        // Don't remove from `winit_to_window_id` so we know the window used to exist.
        self.windows.remove(&winit_id)
    }
}

/// Gets the "best" video mode which fits the given dimensions.
///
/// The heuristic for "best" prioritizes width, height, and refresh rate in that order.
pub fn get_fitting_videomode(
    monitor: &winit::monitor::MonitorHandle,
    width: u32,
    height: u32,
) -> winit::monitor::VideoMode {
    let mut modes = monitor.video_modes().collect::<Vec<_>>();

    fn abs_diff(a: u32, b: u32) -> u32 {
        if a > b {
            return a - b;
        }
        b - a
    }

    modes.sort_by(|a, b| {
        use std::cmp::Ordering::*;
        match abs_diff(a.size().width, width).cmp(&abs_diff(b.size().width, width)) {
            Equal => {
                match abs_diff(a.size().height, height).cmp(&abs_diff(b.size().height, height)) {
                    Equal => b
                        .refresh_rate_millihertz()
                        .cmp(&a.refresh_rate_millihertz()),
                    default => default,
                }
            }
            default => default,
        }
    });

    modes.first().unwrap().clone()
}

/// Gets the "best" videomode from a monitor.
///
/// The heuristic for "best" prioritizes width, height, and refresh rate in that order.
pub fn get_best_videomode(monitor: &winit::monitor::MonitorHandle) -> winit::monitor::VideoMode {
    let mut modes = monitor.video_modes().collect::<Vec<_>>();
    modes.sort_by(|a, b| {
        use std::cmp::Ordering::*;
        match b.size().width.cmp(&a.size().width) {
            Equal => match b.size().height.cmp(&a.size().height) {
                Equal => b
                    .refresh_rate_millihertz()
                    .cmp(&a.refresh_rate_millihertz()),
                default => default,
            },
            default => default,
        }
    });

    modes.first().unwrap().clone()
}

pub(crate) fn attempt_grab(winit_window: &winit::window::Window, grab_mode: CursorGrabMode) {
    let grab_result = match grab_mode {
        bevy_window::CursorGrabMode::None => {
            winit_window.set_cursor_grab(winit::window::CursorGrabMode::None)
        }
        bevy_window::CursorGrabMode::Confined => winit_window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .or_else(|_e| winit_window.set_cursor_grab(winit::window::CursorGrabMode::Locked)),
        bevy_window::CursorGrabMode::Locked => winit_window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .or_else(|_e| winit_window.set_cursor_grab(winit::window::CursorGrabMode::Confined)),
    };

    if let Err(err) = grab_result {
        let err_desc = match grab_mode {
            bevy_window::CursorGrabMode::Confined | bevy_window::CursorGrabMode::Locked => "grab",
            bevy_window::CursorGrabMode::None => "ungrab",
        };

        bevy_utils::tracing::error!("Unable to {} cursor: {}", err_desc, err);
    }
}

/// Compute the physical window position for a given [`WindowPosition`].
// Ideally we could generify this across window backends, but we only really have winit atm
// so whatever.
pub fn winit_window_position(
    position: &WindowPosition,
    resolution: &WindowResolution,
    mut available_monitors: impl Iterator<Item = MonitorHandle>,
    primary_monitor: Option<MonitorHandle>,
    current_monitor: Option<MonitorHandle>,
) -> Option<PhysicalPosition<i32>> {
    match position {
        WindowPosition::Automatic => {
            /* Window manager will handle position */
            None
        }
        WindowPosition::Centered(monitor_selection) => {
            use bevy_window::MonitorSelection::*;
            let maybe_monitor = match monitor_selection {
                Current => {
                    if current_monitor.is_none() {
                        warn!("Can't select current monitor on window creation or cannot find current monitor!");
                    }
                    current_monitor
                }
                Primary => primary_monitor,
                Index(n) => available_monitors.nth(*n),
            };

            if let Some(monitor) = maybe_monitor {
                let screen_size = monitor.size();

                // We use the monitors scale factor here since `WindowResolution.scale_factor` is
                // not yet populated when windows are created during plugin setup.
                let scale_factor = monitor.scale_factor();

                // Logical to physical window size
                let (width, height): (u32, u32) =
                    LogicalSize::new(resolution.width(), resolution.height())
                        .to_physical::<u32>(scale_factor)
                        .into();

                let position = PhysicalPosition {
                    x: screen_size.width.saturating_sub(width) as f64 / 2.
                        + monitor.position().x as f64,
                    y: screen_size.height.saturating_sub(height) as f64 / 2.
                        + monitor.position().y as f64,
                };

                Some(position.cast::<i32>())
            } else {
                warn!("Couldn't get monitor selected with: {monitor_selection:?}");
                None
            }
        }
        WindowPosition::At(position) => {
            Some(PhysicalPosition::new(position[0] as f64, position[1] as f64).cast::<i32>())
        }
    }
}

// WARNING: this only works under the assumption that wasm runtime is single threaded
#[cfg(target_arch = "wasm32")]
unsafe impl Send for WinitWindows {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WinitWindows {}
