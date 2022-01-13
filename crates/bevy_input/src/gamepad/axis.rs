use crate::gamepad::{Gamepad, GamepadAxisType};

/// An axis of a [`Gamepad`].
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Axis`](crate::Axis) to create a `Bevy` resource.
/// This resource stores the data of the  axes of a gamepad and can be accessed inside of a system.
///
/// ## Access
///
/// To access the resource use one of the following:
/// - Non-mutable access of the axes: `Res<Axis<GamepadAxis>>`
/// - Mutable access of the axes: `ResMut<Axis<GamepadAxis>>`
///
/// ## Updating
///
/// The resource is updated inside of the [`gamepad_event_system`](crate::gamepad::gamepad_event_system).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadAxis {
    /// The gamepad on which the axis is located on.
    pub gamepad: Gamepad,
    /// The type of the axis.
    pub axis_type: GamepadAxisType,
}

impl GamepadAxis {
    /// Creates a new [`GamepadAxis`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadAxis, GamepadAxisType, Gamepad};
    /// #
    /// let gamepad_axis = GamepadAxis::new(
    ///     Gamepad::new(1),
    ///     GamepadAxisType::LeftStickX,
    /// );
    /// ```
    pub fn new(gamepad: Gamepad, axis_type: GamepadAxisType) -> Self {
        Self { gamepad, axis_type }
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::{Gamepad, GamepadAxis, GamepadAxisType};

    #[test]
    fn test_new() {
        let gamepad = Gamepad::new(1);
        let axis_type = GamepadAxisType::LeftStickX;
        let axis = GamepadAxis::new(gamepad, axis_type);
        assert_eq!(axis.gamepad, gamepad);
        assert_eq!(axis.axis_type, axis_type);
    }
}
