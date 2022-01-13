use crate::gamepad::{Gamepad, GamepadEventType};

/// An event of a [`Gamepad`].
///
/// This event is the translated version of the [`GamepadEventRaw`].
/// It is available to the end user and can be used for game logic.
///
/// ## Access
///
/// To access or send gamepad events use one of the following:
/// - To access gamepad events: `EventReader<GamepadEvent>`
/// - To send gamepad events: `EventWriter<GamepadEvent>`
///
/// ## Differences
///
/// The difference between the [`GamepadEventRaw`] and the [`GamepadEvent`] is that the latter respects
/// user defined [`GamepadSettings`](crate::gamepad::GamepadSettings) for the gamepad inputs.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEvent {
    /// The gamepad assigned to the event.
    pub gamepad: Gamepad,
    /// The type of the event.
    pub event_type: GamepadEventType,
}

impl GamepadEvent {
    /// Creates a new [`GamepadEvent`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadEvent, GamepadEventType, Gamepad};
    /// #
    /// let gamepad_event = GamepadEvent::new(
    ///     Gamepad::new(1),
    ///     GamepadEventType::Connected,
    /// );
    /// ```
    pub fn new(gamepad: Gamepad, event_type: GamepadEventType) -> Self {
        Self {
            gamepad,
            event_type,
        }
    }
}

/// A raw event of a [`Gamepad`].
///
/// This event is the translated version of the `EventType` from the `GilRs` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Access
///
/// To access or send raw gamepad events use one of the following:
/// - To access raw gamepad events: `EventReader<GamepadEventRaw>`
/// - To send raw gamepad events: `EventWriter<GamepadEventRaw>`
///
/// ## Differences
///
/// The difference between the `EventType` from the `GilRs` crate and the [`GamepadEventRaw`]
/// is that the latter has less events, because the button pressing logic is handled through the generic
/// [`Input<T>`](crate::input::Input<T>) instead of through events.
///
/// The difference between the [`GamepadEventRaw`] and the [`GamepadEvent`] can be seen in the documentation
/// of the [`GamepadEvent`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEventRaw {
    /// The gamepad assigned to the event.
    pub gamepad: Gamepad,
    /// The type of the event.
    pub event_type: GamepadEventType,
}

impl GamepadEventRaw {
    /// Creates a new [`GamepadEventRaw`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadEventRaw, GamepadEventType, Gamepad};
    /// #
    /// let gamepad_event_raw = GamepadEventRaw::new(
    ///     Gamepad::new(1),
    ///     GamepadEventType::Connected,
    /// );
    /// ```
    pub fn new(gamepad: Gamepad, event_type: GamepadEventType) -> Self {
        Self {
            gamepad,
            event_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::{Gamepad, GamepadEvent, GamepadEventRaw, GamepadEventType};

    mod gamepad_event {
        use super::*;

        #[test]
        fn test_new() {
            let gamepad = Gamepad::new(1);
            let event_type = GamepadEventType::Connected;
            let event = GamepadEvent::new(gamepad, event_type.clone());
            assert_eq!(event.gamepad, gamepad);
            assert_eq!(event.event_type, event_type);
        }
    }

    mod gamepad_event_raw {
        use super::*;

        #[test]
        fn test_new() {
            let gamepad = Gamepad::new(1);
            let event_type = GamepadEventType::Connected;
            let event = GamepadEventRaw::new(gamepad, event_type.clone());
            assert_eq!(event.gamepad, gamepad);
            assert_eq!(event.event_type, event_type);
        }
    }
}
