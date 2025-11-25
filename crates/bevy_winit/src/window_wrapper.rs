use alloc::sync::Arc;
use bevy_derive::Deref;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// A wrapper over a winit window.
///
/// This is cloneable, which allows us to extend the lifetime of the window, so it doesn't get eagerly
/// dropped while a pipelined renderer still has frames in flight that need to draw to it.
#[derive(Debug, Deref, Clone)]
pub struct WinitWindowWrapper {
    pub(crate) window: Arc<winit::window::Window>,
}

impl WinitWindowWrapper {
    /// Creates a `WinitWindowWrapper` from a window.
    pub fn new(window: winit::window::Window) -> WinitWindowWrapper {
        WinitWindowWrapper {
            window: Arc::new(window),
        }
    }
}

impl HasWindowHandle for WinitWindowWrapper {
    fn window_handle(
        &self,
    ) -> Result<winit::raw_window_handle::WindowHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window.window_handle()
    }
}

impl HasDisplayHandle for WinitWindowWrapper {
    fn display_handle(
        &self,
    ) -> Result<winit::raw_window_handle::DisplayHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window.display_handle()
    }
}
