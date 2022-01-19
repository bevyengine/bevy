use crate::{keyboard::KeyCode, ButtonState};

/// A keyboard input event.
///
/// This event is the translated version of the `WindowEvent::KeyboardInput` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Access
///
/// To access or send keyboard input events use one of the following:
/// - To access keyboard input events: `EventReader<KeyboardInput>`
/// - To send keyboard input events: `EventWriter<KeyboardInput>`
///
/// ## Usage
///
/// The event is read inside of the [`keyboard_input_system`](crate::keyboard::keyboard_input_system)
/// to update the [`Input<KeyCode>`](crate::Input<KeyCode>) resource.
#[derive(Debug, Clone)]
pub struct KeyboardInput {
    /// The scan code of the key.
    pub scan_code: u32,
    /// The key code of the key.
    pub key_code: Option<KeyCode>,
    /// The press state of the key.
    pub state: ButtonState,
}

impl KeyboardInput {
    /// Creates a new [`KeyboardInput`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::{
    /// #     ButtonState,
    /// #     keyboard::{KeyboardInput, KeyCode}
    /// # };
    /// #
    /// let keyboard_input = KeyboardInput::new(
    ///     48,
    ///     Some(KeyCode::B),
    ///     ButtonState::Pressed,
    /// );
    /// ```
    pub fn new(scan_code: u32, key_code: Option<KeyCode>, state: ButtonState) -> Self {
        Self {
            scan_code,
            key_code,
            state,
        }
    }
}
