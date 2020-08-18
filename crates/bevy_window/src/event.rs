use super::{WindowDescriptor, WindowId};

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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Cursor {
    Vertical(WindowId),
    Horizontal(WindowId),
}
#[derive(Debug, Clone)]
pub struct CursorMoved {
    pub id: Cursor,
    pub position: f32,
}

pub type AxisId = u32;

#[derive(Debug, Clone)]
pub struct Motion {
    pub axis: AxisId,
    pub value: f32,
}
