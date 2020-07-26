use bevy_window::{Window, WindowId};
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
        let winit_window = {
            use winit::platform::windows::WindowBuilderExtWindows;
            winit::window::WindowBuilder::new()
                .with_drag_and_drop(false)
                .build(&event_loop)
                .unwrap()
        };

        #[cfg(not(target_os = "windows"))]
        let winit_window = winit::window::Window::new(&event_loop).unwrap();

        self.window_id_to_winit.insert(window.id, winit_window.id());
        self.winit_to_window_id.insert(winit_window.id(), window.id);

        winit_window.set_title(&window.title);
        winit_window.set_inner_size(winit::dpi::PhysicalSize::new(window.width, window.height));

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
