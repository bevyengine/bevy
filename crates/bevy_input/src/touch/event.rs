use crate::touch::{ForceTouch, TouchPhase};
use bevy_math::Vec2;

/// A touch input event.
///
/// This event is the translated version of the `WindowEvent::Touch` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
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
/// ## Access
///
/// To access or send touch input events use one of the following:
/// - To access touch input events: `EventReader<TouchInput>`
/// - To send touch input events: `EventWriter<TouchInput>`
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl TouchInput {
    /// Creates a new [`TouchInput`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::touch::{TouchInput, TouchPhase, ForceTouch};
    /// # use bevy_math::Vec2;
    /// #
    /// let touch_input = TouchInput::new(
    ///     TouchPhase::Started,
    ///     Vec2::new(1.0, 1.0),
    ///     Some(ForceTouch::Normalized(1.0)),
    ///     1,
    /// );
    /// ```
    pub fn new(phase: TouchPhase, position: Vec2, force: Option<ForceTouch>, id: u64) -> Self {
        Self {
            phase,
            position,
            force,
            id,
        }
    }
}
