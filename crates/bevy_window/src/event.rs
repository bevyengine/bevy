use super::{WindowDescriptor, WindowId};
use glam::Vec2;

/// A window event that is sent whenever a window has been resized.
#[derive(Debug, Clone)]
pub struct WindowResized {
    pub id: WindowId,
    pub width: usize,
    pub height: usize,
    pub is_primary: bool,
}

/// An event that indicates that a new window should be created.
#[derive(Debug, Clone)]
pub struct CreateWindow {
    pub id: WindowId,
    pub descriptor: WindowDescriptor,
}

/// An event that indicates a window should be closed.
#[derive(Debug, Clone)]
pub struct CloseWindow {
    pub id: WindowId,
}

/// An event that is sent whenever a new window is created.
#[derive(Debug, Clone)]
pub struct WindowCreated {
    pub id: WindowId,
    pub is_primary: bool,
}

/// An event that is sent whenever a close was requested for a window. For example: when the "close" button
/// is pressed on a window.
#[derive(Debug, Clone)]
pub struct WindowCloseRequested {
    pub id: WindowId,
    pub is_primary: bool,
}

#[derive(Debug, Clone)]
pub struct CursorMoved {
    pub id: WindowId,
    pub position: Vec2,
}
