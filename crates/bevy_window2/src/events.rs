use std::path::PathBuf;

use bevy_ecs::entity::Entity;
use bevy_math::{IVec2, Vec2};

/// An event that is sent when the cursor has entered a window
#[derive(Debug, Clone)]
pub struct CursorEntered {
    /// Window id
    pub window_id: Entity,
}

/// An event that is sent when the cursor has left a window
#[derive(Debug, Clone)]
pub struct CursorLeft {
    /// Window id
    pub window_id: Entity,
}

/// An event that is sent when the cursor in a window has moved
#[derive(Debug, Clone)]
pub struct CursorMoved {
    /// Window id
    pub window_id: Entity,
    /// The new position of cursor
    pub position: Vec2,
}

/// Events related to files being dragged and dropped on a window.
#[derive(Debug, Clone)]
pub enum FileDragAndDrop {
    /// An event that is sent when a file has been dropped over a window
    DroppedFile {
        /// Window id
        window_id: Entity,
        /// Path of dropped file
        path_buf: PathBuf,
    },

    /// An event that is sent when a file is hovering over a window
    HoveredFile {
        /// Window id
        window_id: Entity,
        /// Path of hovered file
        path_buf: PathBuf,
    },

    /// An event that is sent when a file is no longer hovering over a window
    HoveredFileCancelled {
        /// Window id
        window_id: Entity,
    },
}

/// An event that is sent whenever a window receives a character from the OS or underlying system.
#[derive(Debug, Clone)]
pub struct ReceivedCharacter {
    /// Window id
    pub window_id: Entity,
    /// Received character
    pub char: char,
}

/// An event that indicates the window should redraw, even if its control flow is set to `Wait` and
/// there have been no window events.
#[derive(Debug, Clone)]
pub struct RequestRedraw;

/// An event that is sent whenever a close was requested for a window. For example: when the "close"
/// button is pressed on a window.
#[derive(Debug, Clone)]
pub struct WindowCloseRequested {
    /// Window id
    pub window_id: Entity,
}

/// An event that indicates a window has received or lost focus.
#[derive(Debug, Clone)]
pub struct WindowFocused {
    /// Window id
    pub window_id: Entity,
    /// Whether window has received or lost focus
    pub focused: bool,
}

/// An event that is sent when a window is repositioned in physical pixels.
#[derive(Debug, Clone)]
pub struct WindowMoved {
    /// Window id
    pub window_id: Entity,
    /// The new position of the window
    pub position: IVec2,
}

/// A window event that is sent whenever a windows logical size has changed
#[derive(Debug, Clone)]
pub struct WindowResized {
    /// Window id
    pub window_id: Entity,
    /// The new logical width of the window
    pub width: f32,
    /// The new logical height of the window
    pub height: f32,
}

/// An event that indicates a window's scale factor has changed.
#[derive(Debug, Clone)]
pub struct WindowScaleFactorChanged {
    /// Window id
    pub window_id: Entity,
    /// The new window scale factor
    pub scale_factor: f64,
}

/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(Debug, Clone)]
pub struct WindowScaleFactorBackendChanged {
    /// Window id
    pub window_id: Entity,
    /// The new window scale factor
    pub scale_factor: f64,
}
