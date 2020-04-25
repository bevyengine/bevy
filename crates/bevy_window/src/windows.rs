use super::{Window, WindowId};
use std::collections::HashMap;

#[derive(Default)]
pub struct Windows {
    windows: HashMap<WindowId, Window>,
    primary_window: Option<WindowId>,
}

impl Windows {
    pub fn add(&mut self, window: Window) {
        if let None = self.primary_window {
            self.primary_window = Some(window.id);
        };

        self.windows.insert(window.id, window);
    }

    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn get_primary(&self) -> Option<&Window> {
        self.primary_window
            .and_then(|primary| self.windows.get(&primary))
    }

    pub fn is_primary(&self, window_id: WindowId) -> bool {
        self.get_primary()
            .map(|primary_window| primary_window.id == window_id)
            .unwrap_or(false)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }
}
