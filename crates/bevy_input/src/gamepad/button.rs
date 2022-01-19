use crate::gamepad::{Gamepad, GamepadButtonType};

/// A button of a [`Gamepad`].
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Input`](crate::Input) and [`Axis`](crate::Axis)
/// to create `bevy` resources. These resources store the data of the buttons and axes of a gamepad
/// and can be accessed inside of a system.
///
/// ## Access
///
/// To access the data you can use the [`Input<GamepadButton>`](crate::Input<GamepadButton>) or [`Axis<GamepadButton>`](crate::Axis<GamepadButton>) resource.
///
/// ## Updating
///
/// The resources are updated inside of the [`gamepad_event_system`](crate::gamepad::gamepad_event_system).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadButton {
    /// The gamepad on which the button is located on.
    pub gamepad: Gamepad,
    /// The type of the button.
    pub button_type: GamepadButtonType,
}

impl GamepadButton {
    /// Creates a new [`GamepadButton`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadButton, GamepadButtonType, Gamepad};
    /// #
    /// let gamepad_button = GamepadButton::new(
    ///     Gamepad::new(1),
    ///     GamepadButtonType::South,
    /// );
    /// ```
    pub fn new(gamepad: Gamepad, button_type: GamepadButtonType) -> Self {
        Self {
            gamepad,
            button_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::{Gamepad, GamepadButton, GamepadButtonType};

    #[test]
    fn test_new() {
        let gamepad = Gamepad::new(1);
        let button_type = GamepadButtonType::North;
        let button = GamepadButton::new(gamepad, button_type);
        assert_eq!(button.gamepad, gamepad);
        assert_eq!(button.button_type, button_type);
    }
}
