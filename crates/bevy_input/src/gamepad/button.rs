use crate::gamepad::{Gamepad, GamepadButtonType};

/// A button of a [`Gamepad`].
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Input`](crate::Input) and [`Axis`](crate::Axis)
/// to create `Bevy` resources. These resources store the data of the buttons and axes of a gamepad
/// and can be accessed inside of a system.
///
/// ## Access
///
/// To access the resources use one of the following:
/// - Non-mutable access of the buttons: `Res<Input<GamepadButton>>`
/// - Mutable access of the buttons: `ResMut<Input<GamepadButton>>`
/// - Non-mutable access of the axes: `Res<Axis<GamepadButton>>`
/// - Mutable access of the axes: `ResMut<Axis<GamepadButton>>`
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
