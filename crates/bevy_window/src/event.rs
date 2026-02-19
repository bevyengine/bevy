use alloc::string::String;
use bevy_ecs::{entity::Entity, event::BufferedEvent};
use bevy_input::{
    gestures::*,
    keyboard::{KeyboardFocusLost, KeyboardInput},
    mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    touch::TouchInput,
};
use bevy_math::{IVec2, Vec2};

#[cfg(feature = "std")]
use std::path::PathBuf;

#[cfg(not(feature = "std"))]
use alloc::string::String as PathBuf;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::WindowTheme;

/// A window event that is sent whenever a window's logical size has changed.
#[derive(BufferedEvent, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct RequestRedraw;

/// An event that is sent whenever a new window is created.
///
/// To create a new window, spawn an entity with a [`crate::Window`] on it.
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct WindowCloseRequested {
    /// Window to close.
    pub window: Entity,
}

/// An event that is sent whenever a window is closed. This will be sent when
/// the window entity loses its [`Window`](crate::window::Window) component or is despawned.
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
/// you should not use it for non-cursor-like behavior such as 3D camera control. Please see `MouseMotion` instead.
///
/// [`WindowEvent::CursorMoved`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.CursorMoved
#[derive(BufferedEvent, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct CursorMoved {
    /// Window that the cursor moved inside.
    pub window: Entity,
    /// The cursor position in logical pixels.
    pub position: Vec2,
    /// The change in the position of the cursor since the last event was sent.
    /// This value is `None` if the cursor was outside the window area during the last frame.
    // Because the range of this data is limited by the display area and it may have been
    //  transformed by the OS to implement effects such as cursor acceleration, it should
    // not be used to implement non-cursor-like interactions such as 3D camera control.
    pub delta: Option<Vec2>,
}

/// An event that is sent whenever the user's cursor enters a window.
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct CursorEntered {
    /// Window that the cursor entered.
    pub window: Entity,
}

/// An event that is sent whenever the user's cursor leaves a window.
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct CursorLeft {
    /// Window that the cursor left.
    pub window: Entity,
}

/// An Input Method Editor event.
///
/// This event is the translated version of the `WindowEvent::Ime` from the `winit` crate.
///
/// It is only sent if IME was enabled on the window with [`Window::ime_enabled`](crate::window::Window::ime_enabled).
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
    /// After this event, you will receive events `Ime::Preedit` and `Ime::Commit`.
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
/// whether the window is closed, minimized, set invisible,
/// or fully occluded by another window.
///
/// It is the translated version of [`WindowEvent::Occluded`] from the `winit` crate.
///
/// [`WindowEvent::Occluded`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.Occluded
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct WindowOccluded {
    /// Window that changed occluded state.
    pub window: Entity,
    /// Whether it was occluded (true) or not occluded (false).
    pub occluded: bool,
}

/// An event that indicates a window's scale factor has changed.
#[derive(BufferedEvent, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct WindowScaleFactorChanged {
    /// Window that had its scale factor changed.
    pub window: Entity,
    /// The new scale factor.
    pub scale_factor: f64,
}

/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(BufferedEvent, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct WindowBackendScaleFactorChanged {
    /// Window that had its scale factor changed by the backend.
    pub window: Entity,
    /// The new scale factor.
    pub scale_factor: f64,
}

/// Events related to files being dragged and dropped on a window.
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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
#[derive(BufferedEvent, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct WindowThemeChanged {
    /// Window for which the system theme has changed.
    pub window: Entity,
    /// The new system theme.
    pub theme: WindowTheme,
}

/// Application lifetime events
#[derive(BufferedEvent, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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

/// Wraps all `bevy_window` and `bevy_input` events in a common enum.
///
/// Read these events with `EventReader<WindowEvent>` if you need to
/// access window events in the order they were received from the
/// operating system. Otherwise, the event types are individually
/// readable with `EventReader<E>` (e.g. `EventReader<KeyboardInput>`).
#[derive(BufferedEvent, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum WindowEvent {
    /// An application lifecycle event.
    AppLifecycle(AppLifecycle),
    /// The user's cursor has entered a window.
    CursorEntered(CursorEntered),
    ///The user's cursor has left a window.
    CursorLeft(CursorLeft),
    /// The user's cursor has moved inside a window.
    CursorMoved(CursorMoved),
    /// A file drag and drop event.
    FileDragAndDrop(FileDragAndDrop),
    /// An Input Method Editor event.
    Ime(Ime),
    /// A redraw of all of the application's windows has been requested.
    RequestRedraw(RequestRedraw),
    /// The window's OS-reported scale factor has changed.
    WindowBackendScaleFactorChanged(WindowBackendScaleFactorChanged),
    /// The OS has requested that a window be closed.
    WindowCloseRequested(WindowCloseRequested),
    /// A new window has been created.
    WindowCreated(WindowCreated),
    /// A window has been destroyed by the underlying windowing system.
    WindowDestroyed(WindowDestroyed),
    /// A window has received or lost focus.
    WindowFocused(WindowFocused),
    /// A window has been moved.
    WindowMoved(WindowMoved),
    /// A window has started or stopped being occluded.
    WindowOccluded(WindowOccluded),
    /// A window's logical size has changed.
    WindowResized(WindowResized),
    /// A window's scale factor has changed.
    WindowScaleFactorChanged(WindowScaleFactorChanged),
    /// Sent for windows that are using the system theme when the system theme changes.
    WindowThemeChanged(WindowThemeChanged),

    /// The state of a mouse button has changed.
    MouseButtonInput(MouseButtonInput),
    /// The physical position of a pointing device has changed.
    MouseMotion(MouseMotion),
    /// The mouse wheel has moved.
    MouseWheel(MouseWheel),

    /// A two finger pinch gesture.
    PinchGesture(PinchGesture),
    /// A two finger rotation gesture.
    RotationGesture(RotationGesture),
    /// A double tap gesture.
    DoubleTapGesture(DoubleTapGesture),
    /// A pan gesture.
    PanGesture(PanGesture),

    /// A touch input state change.
    TouchInput(TouchInput),

    /// A keyboard input.
    KeyboardInput(KeyboardInput),
    /// Sent when focus has been lost for all Bevy windows.
    ///
    /// Used to clear pressed key state.
    KeyboardFocusLost(KeyboardFocusLost),
}

impl From<AppLifecycle> for WindowEvent {
    fn from(e: AppLifecycle) -> Self {
        Self::AppLifecycle(e)
    }
}

impl From<CursorEntered> for WindowEvent {
    fn from(e: CursorEntered) -> Self {
        Self::CursorEntered(e)
    }
}

impl From<CursorLeft> for WindowEvent {
    fn from(e: CursorLeft) -> Self {
        Self::CursorLeft(e)
    }
}

impl From<CursorMoved> for WindowEvent {
    fn from(e: CursorMoved) -> Self {
        Self::CursorMoved(e)
    }
}

impl From<FileDragAndDrop> for WindowEvent {
    fn from(e: FileDragAndDrop) -> Self {
        Self::FileDragAndDrop(e)
    }
}

impl From<Ime> for WindowEvent {
    fn from(e: Ime) -> Self {
        Self::Ime(e)
    }
}

impl From<RequestRedraw> for WindowEvent {
    fn from(e: RequestRedraw) -> Self {
        Self::RequestRedraw(e)
    }
}

impl From<WindowBackendScaleFactorChanged> for WindowEvent {
    fn from(e: WindowBackendScaleFactorChanged) -> Self {
        Self::WindowBackendScaleFactorChanged(e)
    }
}

impl From<WindowCloseRequested> for WindowEvent {
    fn from(e: WindowCloseRequested) -> Self {
        Self::WindowCloseRequested(e)
    }
}

impl From<WindowCreated> for WindowEvent {
    fn from(e: WindowCreated) -> Self {
        Self::WindowCreated(e)
    }
}

impl From<WindowDestroyed> for WindowEvent {
    fn from(e: WindowDestroyed) -> Self {
        Self::WindowDestroyed(e)
    }
}

impl From<WindowFocused> for WindowEvent {
    fn from(e: WindowFocused) -> Self {
        Self::WindowFocused(e)
    }
}

impl From<WindowMoved> for WindowEvent {
    fn from(e: WindowMoved) -> Self {
        Self::WindowMoved(e)
    }
}

impl From<WindowOccluded> for WindowEvent {
    fn from(e: WindowOccluded) -> Self {
        Self::WindowOccluded(e)
    }
}

impl From<WindowResized> for WindowEvent {
    fn from(e: WindowResized) -> Self {
        Self::WindowResized(e)
    }
}

impl From<WindowScaleFactorChanged> for WindowEvent {
    fn from(e: WindowScaleFactorChanged) -> Self {
        Self::WindowScaleFactorChanged(e)
    }
}

impl From<WindowThemeChanged> for WindowEvent {
    fn from(e: WindowThemeChanged) -> Self {
        Self::WindowThemeChanged(e)
    }
}

impl From<MouseButtonInput> for WindowEvent {
    fn from(e: MouseButtonInput) -> Self {
        Self::MouseButtonInput(e)
    }
}

impl From<MouseMotion> for WindowEvent {
    fn from(e: MouseMotion) -> Self {
        Self::MouseMotion(e)
    }
}

impl From<MouseWheel> for WindowEvent {
    fn from(e: MouseWheel) -> Self {
        Self::MouseWheel(e)
    }
}

impl From<PinchGesture> for WindowEvent {
    fn from(e: PinchGesture) -> Self {
        Self::PinchGesture(e)
    }
}

impl From<RotationGesture> for WindowEvent {
    fn from(e: RotationGesture) -> Self {
        Self::RotationGesture(e)
    }
}

impl From<DoubleTapGesture> for WindowEvent {
    fn from(e: DoubleTapGesture) -> Self {
        Self::DoubleTapGesture(e)
    }
}

impl From<PanGesture> for WindowEvent {
    fn from(e: PanGesture) -> Self {
        Self::PanGesture(e)
    }
}

impl From<TouchInput> for WindowEvent {
    fn from(e: TouchInput) -> Self {
        Self::TouchInput(e)
    }
}

impl From<KeyboardInput> for WindowEvent {
    fn from(e: KeyboardInput) -> Self {
        Self::KeyboardInput(e)
    }
}

impl From<KeyboardFocusLost> for WindowEvent {
    fn from(e: KeyboardFocusLost) -> Self {
        Self::KeyboardFocusLost(e)
    }
}
