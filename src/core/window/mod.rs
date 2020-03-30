#[cfg(feature = "winit")]
pub mod winit;
mod events;
mod windows;

pub use events::*;
pub use windows::*;

use uuid::Uuid;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(Uuid);

pub struct Window {
    pub id: WindowId,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
}

impl Window {
    pub fn new(window_descriptor: &WindowDescriptor) -> Self {
        Window {
            id: WindowId(Uuid::new_v4()),
            height: window_descriptor.height,
            width: window_descriptor.width,
            title: window_descriptor.title.clone(),
            vsync: window_descriptor.vsync,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "bevy".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}