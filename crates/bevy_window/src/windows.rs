use super::{Window, WindowId};
use bevy_utils::HashMap;
pub use raw_window_handle::TrustedWindowHandle;

#[derive(Debug, Default)]
pub struct Windows {
    windows: HashMap<WindowId, Window>,
}

impl Windows {
    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn get_primary(&self) -> Option<&Window> {
        self.get(WindowId::primary())
    }

    pub fn get_primary_mut(&mut self) -> Option<&mut Window> {
        self.get_mut(WindowId::primary())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Window> {
        self.windows.values_mut()
    }
}

#[derive(Default)]
/// A map from [`WindowId`] to [`TrustedWindowHandle`], stored as a thread_local_resource on the main thread.
///
/// Accessed by graphics backends to allow them to create surfaces for their windows
pub struct WindowHandles {
    handles: HashMap<WindowId, TrustedWindowHandle>,
}

impl WindowHandles {
    pub fn get(&self, id: WindowId) -> Option<&TrustedWindowHandle> {
        self.handles.get(&id)
    }
}

// This is the only way to add to windows, which ensures handles and Windows are kept in sync
/// Add a window to the `bevy` windowing system.
/// In general, this should only called by the windowing backend (which is by default provided by `bevy_winit`)
///
/// To create a window in a bevy application, send a [`crate::CreateWindow`] `Event`.
pub fn create_window(
    windows: &mut Windows,
    window: Window,
    handles: &mut WindowHandles,
    handle: TrustedWindowHandle,
) {
    let id = window.id();
    windows.windows.insert(id, window);
    handles.handles.insert(id, handle);
}
