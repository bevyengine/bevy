use bevy_window::{Window, WindowId, WindowMode};
use std::collections::HashMap;

#[derive(Default)]
pub struct WinitWindows {
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    pub window_id_to_winit: HashMap<WindowId, winit::window::WindowId>,
    pub winit_to_window_id: HashMap<winit::window::WindowId, WindowId>,
}

impl WinitWindows {
    pub fn create_window(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        window: &Window,
    ) {
        #[cfg(target_os = "windows")]
        let winit_window_builder = {
            use winit::platform::windows::WindowBuilderExtWindows;
            winit::window::WindowBuilder::new().with_drag_and_drop(false)
        };

        #[cfg(not(target_os = "windows"))]
        let mut winit_window_builder = winit::window::WindowBuilder::new();

        winit_window_builder = winit_window_builder.with_title(&window.title);

        let winit_window = match window.mode {
            WindowMode::BorderlessFullscreen => {
                let winit_window = winit_window_builder.build(&event_loop).unwrap();
                winit_window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                    winit_window.current_monitor(),
                )));
                winit_window
            }
            WindowMode::Fullscreen { use_size } => {
                let winit_window = winit_window_builder.build(&event_loop).unwrap();

                winit_window.set_fullscreen(Some(winit::window::Fullscreen::Exclusive(
                    match use_size {
                        true => get_fitting_videomode(&winit_window.current_monitor(), &window),
                        false => get_best_videomode(&winit_window.current_monitor()),
                    },
                )));
                winit_window
            }
            _ => winit_window_builder
                .with_inner_size(winit::dpi::PhysicalSize::new(window.width, window.height))
                .with_resizable(window.resizable)
                .build(&event_loop)
                .unwrap(),
        };

        self.window_id_to_winit.insert(window.id, winit_window.id());
        self.winit_to_window_id.insert(winit_window.id(), window.id);

        self.windows.insert(winit_window.id(), winit_window);
    }

    pub fn get_window(&self, id: WindowId) -> Option<&winit::window::Window> {
        self.window_id_to_winit
            .get(&id)
            .and_then(|id| self.windows.get(id))
    }

    pub fn get_window_id(&self, id: winit::window::WindowId) -> Option<WindowId> {
        self.winit_to_window_id.get(&id).cloned()
    }
}
fn get_fitting_videomode(
    monitor: &winit::monitor::MonitorHandle,
    window: &Window,
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
        match abs_diff(a.size().width, window.width).cmp(&abs_diff(b.size().width, window.width)) {
            Equal => {
                match abs_diff(a.size().height, window.height)
                    .cmp(&abs_diff(b.size().height, window.height))
                {
                    Equal => b.refresh_rate().cmp(&a.refresh_rate()),
                    default => default,
                }
            }
            default => default,
        }
    });

    modes.first().unwrap().clone()
}

fn get_best_videomode(monitor: &winit::monitor::MonitorHandle) -> winit::monitor::VideoMode {
    let mut modes = monitor.video_modes().collect::<Vec<_>>();
    modes.sort_by(|a, b| {
        use std::cmp::Ordering::*;
        match b.size().width.cmp(&a.size().width) {
            Equal => match b.size().height.cmp(&a.size().height) {
                Equal => b.refresh_rate().cmp(&a.refresh_rate()),
                default => default,
            },
            default => default,
        }
    });

    modes.first().unwrap().clone()
}
