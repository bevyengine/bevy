#[cfg(feature = "winit")]
pub mod winit;

use uuid::Uuid;

pub struct WindowId(Uuid);

pub struct Window {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
}

#[derive(Debug, Clone)]
pub struct WindowResize {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            id: Uuid::new_v4(),
            title: "bevy".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}