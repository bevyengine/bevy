use std::path::PathBuf;

use bevy_ecs::entity::Entity;
use bevy_math::{IVec2, Vec2};
use bevy_reflect::{FromReflect, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A window event that is sent whenever a window's logical size has changed.
#[derive(Debug, Clone, PartialEq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowResized {
    /// Window that has changed.
    pub window: Entity,
    /// The new logical width of the window.
    pub width: f32,
    /// The new logical height of the window.
    pub height: f32,
}

// TODO: This would redraw all windows ? If yes, update docs to reflect this
/// An event that indicates the window should redraw, even if its control flow is set to `Wait` and
/// there have been no window events.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RequestRedraw;

/// An event that is sent whenever a new window is created.
///
/// To create a new window, spawn an entity with a [`crate::Window`] on it.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowCreated {
    /// Window that has been created.
    pub window: Entity,
}

/// An event that is sent whenever the operating systems requests that a window
/// be closed. This will be sent when the close button of the window is pressed.
///
/// If the default [`WindowPlugin`] is used, these events are handled
/// by closing the corresponding [`Window`].  
/// To disable this behaviour, set `close_when_requested` on the [`WindowPlugin`]
/// to `false`.
///
/// [`WindowPlugin`]: crate::WindowPlugin
/// [`Window`]: crate::Window
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowCloseRequested {
    /// Window to close.
    pub window: Entity,
}

/// An event that is sent whenever a window is closed. This will be sent when
/// the window entity loses its `Window` component or is despawned.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowClosed {
    /// Window that has been closed.
    ///
    /// Note that this entity probably no longer exists
    /// by the time this event is received.
    pub window: Entity,
}
/// An event reporting that the mouse cursor has moved inside a window.
///
/// The event is sent only if the cursor is over one of the application's windows.
/// It is the translated version of [`WindowEvent::CursorMoved`] from the `winit` crate.
///
/// Not to be confused with the [`MouseMotion`] event from `bevy_input`.
///
/// [`WindowEvent::CursorMoved`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.CursorMoved
/// [`MouseMotion`]: bevy_input::mouse::MouseMotion
#[derive(Debug, Clone, PartialEq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct CursorMoved {
    /// Window that the cursor moved inside.
    pub window: Entity,
    /// The cursor position in logical pixels.
    pub position: Vec2,
}

/// An event that is sent whenever the user's cursor enters a window.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct CursorEntered {
    /// Window that the cursor entered.
    pub window: Entity,
}

/// An event that is sent whenever the user's cursor leaves a window.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct CursorLeft {
    /// Window that the cursor left.
    pub window: Entity,
}

/// An event that is sent whenever a window receives a character from the OS or underlying system.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ReceivedCharacter {
    /// Window that received the character.
    pub window: Entity,
    /// Received character.
    pub char: char,
}

/// A Input Method Editor event.
///
/// This event is the translated version of the `WindowEvent::Ime` from the `winit` crate.
///
/// It is only sent if IME was enabled on the window with [`Window::ime_enabled`](crate::window::Window::ime_enabled).
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum Ime {
    /// Notifies when a new composing text should be set at the cursor position.
    Preedit {
        /// Window that received the event.
        window: Entity,
        /// Current value.
        value: String,
        /// Cursor begin and end position.
        ///
        /// `None` indicated the cursor should be hidden
        cursor: Option<(usize, usize)>,
    },
    /// Notifies when text should be inserted into the editor widget.
    Commit {
        /// Window that received the event.
        window: Entity,
        /// Input string
        value: String,
    },
    /// Notifies when the IME was enabled.
    ///
    /// After this event, you will receive events `Ime::Preedit` and `Ime::Commit`,
    /// and stop receiving events [`ReceivedCharacter`].
    Enabled {
        /// Window that received the event.
        window: Entity,
    },
    /// Notifies when the IME was disabled.
    Disabled {
        /// Window that received the event.
        window: Entity,
    },
}

/// An event that indicates a window has received or lost focus.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowFocused {
    /// Window that changed focus.
    pub window: Entity,
    /// Whether it was focused (true) or lost focused (false).
    pub focused: bool,
}

/// An event that indicates a window's scale factor has changed.
#[derive(Debug, Clone, PartialEq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowScaleFactorChanged {
    /// Window that had it's scale factor changed.
    pub window: Entity,
    /// The new scale factor.
    pub scale_factor: f64,
}

/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(Debug, Clone, PartialEq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowBackendScaleFactorChanged {
    /// Window that had it's scale factor changed by the backend.
    pub window: Entity,
    /// The new scale factor.
    pub scale_factor: f64,
}

/// Events related to files being dragged and dropped on a window.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum FileDragAndDrop {
    /// File is being dropped into a window.
    DroppedFile {
        /// Window the file was dropped into.
        window: Entity,
        /// Path to the file that was dropped in.
        path_buf: PathBuf,
    },

    /// File is currently being hovered over a window.
    HoveredFile {
        /// Window a file is possibly going to be dropped into.
        window: Entity,
        /// Path to the file that might be dropped in.
        path_buf: PathBuf,
    },

    /// File hovering was cancelled.
    HoveredFileCancelled {
        /// Window that had a cancelled file drop.
        window: Entity,
    },
}

/// An event that is sent when a window is repositioned in physical pixels.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowMoved {
    /// Window that moved.
    pub entity: Entity,
    /// Where the window moved to in physical pixels.
    pub position: IVec2,
}
