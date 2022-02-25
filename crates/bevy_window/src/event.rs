use std::path::PathBuf;

use super::{WindowDescriptor, WindowId};
use bevy_math::{IVec2, Vec2};

/// A window event that is sent whenever a windows logical size has changed
#[derive(Debug, Clone)]
pub struct WindowResized {
    pub id: WindowId,
    /// The new logical width of the window
    pub width: f32,
    /// The new logical height of the window
    pub height: f32,
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

/// An event that is sent whenever a close was requested for a window. For example: when the "close"
/// button is pressed on a window.
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
pub struct CursorEntered {
    pub id: WindowId,
}

#[derive(Debug, Clone)]
pub struct CursorLeft {
    pub id: WindowId,
}

/// An event that is sent whenever a window receives a character from the OS or underlying system.
#[derive(Debug, Clone)]
pub struct ReceivedCharacter {
    pub id: WindowId,
    pub char: char,
}

/// An event that indicates a window has received or lost focus.
#[derive(Debug, Clone)]
pub struct WindowFocused {
    pub id: WindowId,
    pub focused: bool,
}

/// An event that indicates a window's scale factor has changed.
#[derive(Debug, Clone)]
pub struct WindowScaleFactorChanged {
    pub id: WindowId,
    pub scale_factor: f64,
}
/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(Debug, Clone)]
pub struct WindowBackendScaleFactorChanged {
    pub id: WindowId,
    pub scale_factor: f64,
}

/// Events related to files being dragged and dropped on a window.
#[derive(Debug, Clone)]
pub enum FileDragAndDrop {
    DroppedFile { id: WindowId, path_buf: PathBuf },

    HoveredFile { id: WindowId, path_buf: PathBuf },

    HoveredFileCancelled { id: WindowId },
}

/// An event that is sent when a window is repositioned in physical pixels.
#[derive(Debug, Clone)]
pub struct WindowMoved {
    pub id: WindowId,
    pub position: IVec2,
}
