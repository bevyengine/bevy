use std::path::PathBuf;

use super::touch::{ForceTouch, TouchPhase};
use super::{WindowDescriptor, WindowId};
use bevy_math::{IVec2, Vec2};

/// A touch input event.
///
/// ## Logic
///
/// Every time the user touches the screen, a new [`TouchPhase::Started`] event with an unique
/// identifier for the finger is generated. When the finger is lifted, the [`TouchPhase::Ended`]
/// event is generated with the same finger id.
///
/// After a [`TouchPhase::Started`] event has been emitted, there may be zero or more [`TouchPhase::Moved`]
/// events when the finger is moved or the touch pressure changes.
///
/// The finger id may be reused by the system after an [`TouchPhase::Ended`] event. The user
/// should assume that a new [`TouchPhase::Started`] event received with the same id has nothing
/// to do with the old finger and is a new finger.
///
/// A [`TouchPhase::Cancelled`] event is emitted when the system has canceled tracking this
/// touch, such as when the window loses focus, or on iOS if the user moves the
/// device against their face.
///
/// ## Note
///
/// This event is the translated version of the `WindowEvent::Touch` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TouchInput {
    /// The phase of the touch input.
    pub phase: TouchPhase,
    /// The position of the finger on the touchscreen.
    pub position: Vec2,
    /// Describes how hard the screen was pressed.
    ///
    /// May be [`None`] if the platform does not support pressure sensitivity.
    /// This feature is only available on **iOS** 9.0+ and **Windows** 8+.
    pub force: Option<ForceTouch>,
    /// The unique identifier of the finger.
    pub id: u64,
    /// The id of the window that was touched.
    pub window_id: WindowId,
}

/// An event reporting that the ESC key has been pressed
/// while the current window is in focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowESC {
    pub window_id: WindowId,
}

/// An event reporting that the mouse cursor has moved on a window.
///
/// The event is sent only if the cursor is over one of the application's windows.
/// It is the translated version of [`WindowEvent::CursorMoved`] from the `winit` crate.
///
/// [`WindowEvent::CursorMoved`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.CursorMoved
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CursorMoved {
    /// The identifier of the window the cursor has moved on.
    pub id: WindowId,

    /// The position of the cursor, in window coordinates.
    pub position: Vec2,
}

/// A window event that is sent whenever a window's logical size has changed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowResized {
    pub id: WindowId,
    /// The new logical width of the window.
    pub width: f32,
    /// The new logical height of the window.
    pub height: f32,
}

/// An event that indicates that a new window should be created.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateWindow {
    pub id: WindowId,
    pub descriptor: WindowDescriptor,
}

/// An event that indicates the window should redraw, even if its control flow is set to `Wait` and
/// there have been no window events.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestRedraw;

/// An event that is sent whenever a new window is created.
///
/// To create a new window, send a [`CreateWindow`] event - this
/// event will be sent in the handler for that event.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowCreated {
    pub id: WindowId,
}

/// An event that is sent whenever the operating systems requests that a window
/// be closed. This will be sent when the close button of the window is pressed.
///
/// If the default [`WindowPlugin`] is used, these events are handled
/// by [closing] the corresponding [`Window`].  
/// To disable this behaviour, set `close_when_requested` on the [`WindowPlugin`]
/// to `false`.
///
/// [`WindowPlugin`]: crate::WindowPlugin
/// [`Window`]: crate::Window
/// [closing]: crate::Window::close
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowCloseRequested {
    pub id: WindowId,
}

/// An event that is sent whenever a window is closed. This will be sent by the
/// handler for [`Window::close`].
///
/// [`Window::close`]: crate::Window::close
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowClosed {
    pub id: WindowId,
}

/// An event that is sent whenever the user's cursor enters a window.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CursorEntered {
    pub id: WindowId,
}
/// An event that is sent whenever the user's cursor leaves a window.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CursorLeft {
    pub id: WindowId,
}

/// An event that is sent whenever a window receives a character from the OS or underlying system.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ReceivedCharacter {
    pub id: WindowId,
    pub char: char,
}

/// An event that indicates a window has received or lost focus.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowFocused {
    pub id: WindowId,
    pub focused: bool,
}

/// An event that indicates a window's scale factor has changed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowScaleFactorChanged {
    pub id: WindowId,
    pub scale_factor: f64,
}
/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowBackendScaleFactorChanged {
    pub id: WindowId,
    pub scale_factor: f64,
}

/// Events related to files being dragged and dropped on a window.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum FileDragAndDrop {
    DroppedFile { id: WindowId, path_buf: PathBuf },

    HoveredFile { id: WindowId, path_buf: PathBuf },

    HoveredFileCancelled { id: WindowId },
}

/// An event that is sent when a window is repositioned in physical pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowMoved {
    pub id: WindowId,
    pub position: IVec2,
}
