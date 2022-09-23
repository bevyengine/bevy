use bevy_window::WindowId;
use bevy_math::Vec2;

/// An event reporting that the mouse cursor has moved on a window.
///
/// The event is sent only if the cursor is over one of the application's windows.
/// It is the translated version of [`WindowEvent::CursorMoved`] from the `winit` crate.
///
/// Not to be confused with the [`MouseMotion`] event from `bevy_input`.
///
/// [`WindowEvent::CursorMoved`]: https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.CursorMoved
/// [`MouseMotion`]: bevy_input::mouse::MouseMotion
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CursorMoved {
    /// The identifier of the window the cursor has moved on.
    pub id: WindowId,

    /// The position of the cursor, in window coordinates.
    pub position: Vec2,
}
