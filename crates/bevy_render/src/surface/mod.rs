mod viewport;

pub use viewport::*;

use crate::renderer::TextureId;
use bevy_window::WindowId;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum SurfaceId {
    Window(WindowId),
    Texture(TextureId),
}

impl SurfaceId {
    pub fn get_window(&self) -> Option<WindowId> {
        if let SurfaceId::Window(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn get_texture(&self) -> Option<TextureId> {
        if let SurfaceId::Texture(id) = self {
            Some(*id)
        } else {
            None
        }
    }
}

impl Default for SurfaceId {
    fn default() -> Self {
        WindowId::primary().into()
    }
}

impl From<WindowId> for SurfaceId {
    fn from(value: WindowId) -> Self {
        SurfaceId::Window(value)
    }
}

impl From<TextureId> for SurfaceId {
    fn from(value: TextureId) -> Self {
        SurfaceId::Texture(value)
    }
}
