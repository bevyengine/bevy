use super::{WindowDescriptor, WindowId};
use bevy_math::Vec2;

/// A window event that is sent whenever a window has been resized.
#[derive(Debug, Clone)]
pub struct WindowResized {
    pub id: WindowId,
    pub width: usize,
    pub height: usize,
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
}

/// An event that is sent whenever a close was requested for a window. For example: when the "close" button
/// is pressed on a window.
#[derive(Debug, Clone)]
pub struct WindowCloseRequested {
    pub id: WindowId,
}

#[derive(Debug, Clone)]
pub struct CursorMoved {
    pub id: WindowId,
    pub position: Vec2,
}

#[derive(Debug, Clone)]
pub enum CursorLockMode {
    Locked,
    Unlocked,
}

/// Event that should be sent when the cursor should be locked or unlocked.
#[derive(Debug, Clone)]
pub struct ChangeCursorLockState {
    pub id: WindowId,
    pub mode: CursorLockMode,
}

#[derive(Debug, Clone)]
pub enum CursorShowMode {
    Show,
    Hide,
}

/// Event that should be sent when the cursor should be hidden or shown.
#[derive(Debug, Clone)]
pub struct ChangeCursorVisibility {
    pub id: WindowId,
    pub mode: CursorShowMode,
}
