use crate::converters::convert_cursor_grab_mode;
use bevy_math::{DVec2, IVec2};
use bevy_utils::HashMap;
use bevy_window::{
    CursorGrabMode, MonitorSelection, RawHandleWrapper, Window, WindowDescriptor, WindowId,
    WindowMode,
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    window::Fullscreen,
};

#[derive(Debug, Default)]
pub struct WinitWindows {
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    pub window_id_to_winit: HashMap<WindowId, winit::window::WindowId>,
    pub winit_to_window_id: HashMap<winit::window::WindowId, WindowId>,
    // Some winit functions, such as `set_window_icon` can only be used from the main thread. If
    // they are used in another thread, the app will hang. This marker ensures `WinitWindows` is
    // only ever accessed with bevy's non-send functions and in NonSend systems.
    _not_send_sync: core::marker::PhantomData<*const ()>,
}

impl WinitWindows {
    pub fn create_window(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        window_id: WindowId,
        window_descriptor: &WindowDescriptor,
    ) -> Window {
        let mut winit_window_builder = winit::window::WindowBuilder::new();

        let &WindowDescriptor {
            width,
            height,
            position,
            monitor,
            scale_factor_override,
            ..
        } = window_descriptor;

        let logical_size = LogicalSize::new(width, height);

        let monitor = match monitor {
            MonitorSelection::Current => None,
            MonitorSelection::Primary => event_loop.primary_monitor(),
            MonitorSelection::Index(i) => event_loop.available_monitors().nth(i),
        };

        let selected_or_primary_monitor = monitor.clone().or_else(|| event_loop.primary_monitor());

        winit_window_builder = match window_descriptor.mode {
            WindowMode::BorderlessFullscreen => winit_window_builder
                .with_fullscreen(Some(Fullscreen::Borderless(selected_or_primary_monitor))),
            WindowMode::Fullscreen => winit_window_builder.with_fullscreen(Some(
                Fullscreen::Exclusive(get_best_videomode(&selected_or_primary_monitor.unwrap())),
            )),
            WindowMode::SizedFullscreen => winit_window_builder.with_fullscreen(Some(
                Fullscreen::Exclusive(get_fitting_videomode(
                    &selected_or_primary_monitor.unwrap(),
                    window_descriptor.width as u32,
                    window_descriptor.height as u32,
                )),
            )),
            _ => {
                if let Some(sf) = scale_factor_override {
                    winit_window_builder.with_inner_size(logical_size.to_physical::<f64>(sf))
                } else {
                    winit_window_builder.with_inner_size(logical_size)
                }
            }
            .with_resizable(window_descriptor.resizable)
            .with_decorations(window_descriptor.decorations)
            .with_transparent(window_descriptor.transparent)
            .with_always_on_top(window_descriptor.always_on_top),
        };

        let constraints = window_descriptor.resize_constraints.check_constraints();
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
        let mut winit_window_builder = winit_window_builder.with_title(&window_descriptor.title);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowBuilderExtWebSys;

            if let Some(selector) = &window_descriptor.canvas {
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
        }

        let winit_window = winit_window_builder.build(event_loop).unwrap();

        if window_descriptor.mode == WindowMode::Windowed {
            use bevy_window::WindowPosition::*;
            match position {
                Automatic => {
                    if let Some(monitor) = monitor {
                        winit_window.set_outer_position(monitor.position());
                    }
                }
                Centered => {
                    if let Some(monitor) = monitor.or_else(|| winit_window.current_monitor()) {
                        let monitor_position = monitor.position().cast::<f64>();
                        let size = monitor.size();

                        // Logical to physical window size
                        let PhysicalSize::<u32> { width, height } =
                            logical_size.to_physical(monitor.scale_factor());

                        let position = PhysicalPosition {
                            x: size.width.saturating_sub(width) as f64 / 2. + monitor_position.x,
                            y: size.height.saturating_sub(height) as f64 / 2. + monitor_position.y,
                        };

                        winit_window.set_outer_position(position);
                    }
                }
                At(position) => {
                    if let Some(monitor) = monitor.or_else(|| winit_window.current_monitor()) {
                        let monitor_position = DVec2::from(<(_, _)>::from(monitor.position()));
                        let position = monitor_position + position.as_dvec2();

                        if let Some(sf) = scale_factor_override {
                            winit_window.set_outer_position(
                                LogicalPosition::new(position.x, position.y).to_physical::<f64>(sf),
                            );
                        } else {
                            winit_window
                                .set_outer_position(LogicalPosition::new(position.x, position.y));
                        }
                    }
                }
            }
        }

        // Do not set the grab mode on window creation if it's none, this can fail on mobile
        if window_descriptor.cursor_grab_mode != CursorGrabMode::None {
            match winit_window
                .set_cursor_grab(convert_cursor_grab_mode(window_descriptor.cursor_grab_mode))
            {
                Ok(_) | Err(winit::error::ExternalError::NotSupported(_)) => {}
                Err(err) => Err(err).unwrap(),
            }
        }

        winit_window.set_cursor_visible(window_descriptor.cursor_visible);

        self.window_id_to_winit.insert(window_id, winit_window.id());
        self.winit_to_window_id.insert(winit_window.id(), window_id);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            if window_descriptor.canvas.is_none() {
                let canvas = winit_window.canvas();

                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let body = document.body().unwrap();

                body.append_child(&canvas)
                    .expect("Append canvas to HTML body.");
            }
        }

        let position = winit_window
            .outer_position()
            .ok()
            .map(|position| IVec2::new(position.x, position.y));
        let inner_size = winit_window.inner_size();
        let scale_factor = winit_window.scale_factor();
        let raw_handle = RawHandleWrapper {
            window_handle: winit_window.raw_window_handle(),
            display_handle: winit_window.raw_display_handle(),
        };
        self.windows.insert(winit_window.id(), winit_window);
        Window::new(
            window_id,
            window_descriptor,
            inner_size.width,
            inner_size.height,
            scale_factor,
            position,
            Some(raw_handle),
        )
    }

    pub fn get_window(&self, id: WindowId) -> Option<&winit::window::Window> {
        self.window_id_to_winit
            .get(&id)
            .and_then(|id| self.windows.get(id))
    }

    pub fn get_window_id(&self, id: winit::window::WindowId) -> Option<WindowId> {
        self.winit_to_window_id.get(&id).cloned()
    }

    pub fn remove_window(&mut self, id: WindowId) -> Option<winit::window::Window> {
        let winit_id = self.window_id_to_winit.remove(&id)?;
        // Don't remove from winit_to_window_id, to track that we used to know about this winit window
        self.windows.remove(&winit_id)
    }
}

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
