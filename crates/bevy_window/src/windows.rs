use super::{Window, WindowId};
use bevy_ecs::prelude::Resource;
use bevy_utils::HashMap;

/// A collection of [`Window`]s with unique [`WindowId`]s.
#[derive(Debug, Default, Resource)]
pub struct Windows {
    windows: HashMap<WindowId, Window>,
}

impl Windows {
    /// Add the provided [`Window`] to the [`Windows`] resource.
    pub fn add(&mut self, window: Window) {
        self.windows.insert(window.id(), window);
    }

    /// Get a reference to the [`Window`] of `id`.
    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    /// Get a mutable reference to the provided [`WindowId`].
    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    /// Get a reference to the primary [`Window`].
    pub fn get_primary(&self) -> Option<&Window> {
        self.get(WindowId::primary())
    }

    /// Get a reference to the primary [`Window`].
    ///
    /// # Panics
    ///
    /// Panics if the primary window does not exist in [`Windows`].
    pub fn primary(&self) -> &Window {
        self.get_primary().expect("Primary window does not exist")
    }

    /// Get a mutable reference to the primary [`Window`].
    pub fn get_primary_mut(&mut self) -> Option<&mut Window> {
        self.get_mut(WindowId::primary())
    }

    /// Get a mutable reference to the primary [`Window`].
    ///
    /// # Panics
    ///
    /// Panics if the primary window does not exist in [`Windows`].
    pub fn primary_mut(&mut self) -> &mut Window {
        self.get_primary_mut()
            .expect("Primary window does not exist")
    }

    /// Get a reference to the focused [`Window`].
    pub fn get_focused(&self) -> Option<&Window> {
        self.windows.values().find(|window| window.is_focused())
    }

    /// Get a mutable reference to the focused [`Window`].
    pub fn get_focused_mut(&mut self) -> Option<&mut Window> {
        self.windows.values_mut().find(|window| window.is_focused())
    }

    /// Returns the scale factor for the [`Window`] of `id`, or `1.0` if the window does not exist.
    pub fn scale_factor(&self, id: WindowId) -> f64 {
        if let Some(window) = self.get(id) {
            window.scale_factor()
        } else {
            1.0
        }
    }

    /// An iterator over all registered [`Window`]s.
    pub fn iter(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }

    /// A mutable iterator over all registered [`Window`]s.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Window> {
        self.windows.values_mut()
    }

    pub fn remove(&mut self, id: WindowId) -> Option<Window> {
        self.windows.remove(&id)
    }
}
