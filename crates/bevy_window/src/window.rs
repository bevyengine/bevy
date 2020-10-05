use uuid::Uuid;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(Uuid);

impl WindowId {
    pub fn new() -> Self {
        WindowId(Uuid::new_v4())
    }

    pub fn primary() -> Self {
        WindowId(Uuid::from_u128(0))
    }

    pub fn is_primary(&self) -> bool {
        *self == WindowId::primary()
    }
}

use std::fmt;

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_simple().fmt(f)
    }
}

impl Default for WindowId {
    fn default() -> Self {
        WindowId::primary()
    }
}

#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub mode: WindowMode,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
}

/// Defines the way a window is displayed
/// The use_size option that is used in the Fullscreen variant
/// defines whether a videomode is chosen that best fits the width and height
/// in the Window structure, or if these are ignored.
/// E.g. when use_size is set to false the best video mode possible is chosen.
#[derive(Debug, Clone, Copy)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
    Fullscreen { use_size: bool },
}

impl Window {
    pub fn new(id: WindowId, window_descriptor: &WindowDescriptor) -> Self {
        Window {
            id,
            height: window_descriptor.height,
            width: window_descriptor.width,
            title: window_descriptor.title.clone(),
            vsync: window_descriptor.vsync,
            resizable: window_descriptor.resizable,
            decorations: window_descriptor.decorations,
            mode: window_descriptor.mode,
            #[cfg(target_arch = "wasm32")]
            canvas: window_descriptor.canvas.clone(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::manual_non_exhaustive)]
pub struct WindowDescriptor {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub mode: WindowMode,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,

    // this is a manual implementation of the non exhaustive pattern,
    // especially made to allow ..Default::default()
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "bevy".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
            resizable: true,
            decorations: true,
            mode: WindowMode::Windowed,
            #[cfg(target_arch = "wasm32")]
            canvas: None,
            __non_exhaustive: (),
        }
    }
}
