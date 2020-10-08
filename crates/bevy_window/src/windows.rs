use super::{Window, WindowId};
use bevy_utils::HashMap;

#[derive(Debug, Default)]
pub struct Windows {
    windows: HashMap<WindowId, Window>,
}

impl Windows {
    pub fn add(&mut self, window: Window) {
        self.windows.insert(window.id, window);
    }

    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn get_primary(&self) -> Option<&Window> {
        self.get(WindowId::primary())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }
}
