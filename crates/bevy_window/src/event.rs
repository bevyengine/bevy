#![allow(deprecated)]
use std::path::PathBuf;

use bevy_ecs::entity::Entity;
use bevy_ecs::event::Event;
use bevy_math::{IVec2, Vec2};
use bevy_reflect::Reflect;
use smol_str::SmolStr;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::WindowTheme;

/// A window event that is sent whenever a window's logical size has changed.
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
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

/// An event that indicates all of the application's windows should be redrawn,
/// even if their control flow is set to `Wait` and there have been no window events.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
/// To disable this behavior, set `close_when_requested` on the [`WindowPlugin`]
/// to `false`.
///
/// [`WindowPlugin`]: crate::WindowPlugin
/// [`Window`]: crate::Window
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
/// the window entity loses its [`Window`](crate::window::Window) component or is despawned.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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

/// An event that is sent whenever a window is closing. This will be sent when
/// after a [`WindowCloseRequested`] event is received and the window is in the process of closing.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowClosing {
    /// Window that has been requested to close and is the process of closing.
    pub window: Entity,
}

/// An event that is sent whenever a window is destroyed by the underlying window system.
///
/// Note that if your application only has a single window, this event may be your last chance to
/// persist state before the application terminates.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowDestroyed {
    /// Window that has been destroyed.
    ///
    /// Note that this entity probably no longer exists
    /// by the time this event is received.
    pub window: Entity,
}

/// An event reporting that the mouse cursor has moved inside a window.
///
/// The event is sent only if the cursor is over one of the application's windows.
/// It is the translated version of [`WindowEvent::CursorMoved`] from the `winit` crate with the addition of `delta`.
///
/// Not to be confused with the `MouseMotion` event from `bevy_input`.
///
/// Because the range of data is limited by the window area and it may have been transformed by the OS to implement certain effects like acceleration,
/// you should not use it for non-cursor-like behaviour such as 3D camera control. Please see `MouseMotion` instead.
///
/// [`WindowEvent::CursorMoved`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.CursorMoved
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
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
    /// The change in the position of the cursor since the last event was sent.
    /// This value is `None` if the cursor was outside the window area during the last frame.
    //
    // Because the range of this data is limited by the display area and it may have been
    //  transformed by the OS to implement effects such as cursor acceleration, it should
    // not be used to implement non-cursor-like interactions such as 3D camera control.
    pub delta: Option<Vec2>,
}

/// An event that is sent whenever the user's cursor enters a window.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
#[deprecated(since = "0.14.0", note = "Use `KeyboardInput` instead.")]
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
    pub char: SmolStr,
}

/// A Input Method Editor event.
///
/// This event is the translated version of the `WindowEvent::Ime` from the `winit` crate.
///
/// It is only sent if IME was enabled on the window with [`Window::ime_enabled`](crate::window::Window::ime_enabled).
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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

/// The window has been occluded (completely hidden from view).
///
/// This is different to window visibility as it depends on
/// whether the window is closed, minimised, set invisible,
/// or fully occluded by another window.
///
/// It is the translated version of [`WindowEvent::Occluded`] from the `winit` crate.
///
/// [`WindowEvent::Occluded`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.Occluded
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowOccluded {
    /// Window that changed occluded state.
    pub window: Entity,
    /// Whether it was occluded (true) or not occluded (false).
    pub occluded: bool,
}

/// An event that indicates a window's scale factor has changed.
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowScaleFactorChanged {
    /// Window that had its scale factor changed.
    pub window: Entity,
    /// The new scale factor.
    pub scale_factor: f64,
}

/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowBackendScaleFactorChanged {
    /// Window that had its scale factor changed by the backend.
    pub window: Entity,
    /// The new scale factor.
    pub scale_factor: f64,
}

/// Events related to files being dragged and dropped on a window.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
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

    /// File hovering was canceled.
    HoveredFileCanceled {
        /// Window that had a canceled file drop.
        window: Entity,
    },
}

/// An event that is sent when a window is repositioned in physical pixels.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowMoved {
    /// Window that moved.
    pub window: Entity,
    /// Where the window moved to in physical pixels.
    pub position: IVec2,
}

/// An event sent when the system theme changes for a window.
///
/// This event is only sent when the window is relying on the system theme to control its appearance.
/// i.e. It is only sent when [`Window::window_theme`](crate::window::Window::window_theme) is `None` and the system theme changes.
#[derive(Event, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct WindowThemeChanged {
    /// Window for which the system theme has changed.
    pub window: Entity,
    /// The new system theme.
    pub theme: WindowTheme,
}

/// Application lifetime events
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum AppLifecycle {
    /// The application is not started yet.
    Idle,
    /// The application is running.
    Running,
    /// The application is going to be suspended.
    /// Applications have one frame to react to this event before being paused in the background.
    WillSuspend,
    /// The application was suspended.
    Suspended,
    /// The application is going to be resumed.
    /// Applications have one extra frame to react to this event before being fully resumed.
    WillResume,
}

impl AppLifecycle {
    /// Return `true` if the app can be updated.
    #[inline]
    pub fn is_active(&self) -> bool {
        match self {
            Self::Idle | Self::Suspended => false,
            Self::Running | Self::WillSuspend | Self::WillResume => true,
        }
    }
}
