use bevy_utils::HashSet;

/// A gamepad with an associated `ID`.
///
/// ## Usage
///
/// It is used inside of [`GamepadEvent`](crate::gamepad::GamepadEvent)s and
/// [`GamepadEventRaw`](crate::gamepad::GamepadEventRaw)s to distinguish which
/// gamepad the event corresponds to.
///
/// ## Note
///
/// The `ID` of a gamepad is valid throughout a whole session.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Gamepad {
    /// The `ID` of the gamepad.
    pub id: usize,
}

impl Gamepad {
    /// Creates a new [`Gamepad`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::Gamepad;
    /// #
    /// let gamepad = Gamepad::new(1);
    /// ```
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

/// A collection of connected [`Gamepad`]s.
///
/// ## Usage
///
/// It is used to create a `Bevy` resource that stores all of the currently connected [`Gamepad`]s.
///
/// ## Access
///
/// To access the resource use one of the following:
/// - Non-mutable access of the gamepads: `Res<Gamepads>`
/// - Mutable access of the gamepads: `ResMut<Gamepads>`
///
/// ## Updating
///
/// The [`Gamepad`]s are registered and deregistered in the [`gamepad_connection_system`][crate::gamepad::gamepad_connection_system]
/// whenever a [`GamepadEventType::Connected`](crate::gamepad::GamepadEventType::Connected) or
/// [`GamepadEventType::Disconnected`](crate::gamepad::GamepadEventType::Disconnected) event is received.
#[derive(Debug, Default)]
pub struct Gamepads {
    /// The collection of the connected [`Gamepad`]s.
    gamepads: HashSet<Gamepad>,
}

impl Gamepads {
    /// Returns `true` if the `gamepad` is connected.
    pub fn contains(&self, gamepad: &Gamepad) -> bool {
        self.gamepads.contains(gamepad)
    }

    /// An iterator visiting all connected [`Gamepad`]s in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &Gamepad> + '_ {
        self.gamepads.iter()
    }

    /// Registers a [`Gamepad`].
    pub(crate) fn register(&mut self, gamepad: Gamepad) {
        self.gamepads.insert(gamepad);
    }

    /// Deregisters a [`Gamepad`].
    pub(crate) fn deregister(&mut self, gamepad: &Gamepad) {
        self.gamepads.remove(gamepad);
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::{Gamepad, Gamepads};
    use bevy_utils::HashSet;

    mod gamepad {
        use super::*;

        #[test]
        fn test_new() {
            let gamepad = Gamepad::new(1);
            assert_eq!(gamepad.id, 1);
        }
    }

    mod gamepads {
        use super::*;

        #[test]
        fn test_contains() {
            let gamepad = Gamepad::new(1);
            let mut gamepad_set = HashSet::default();
            gamepad_set.insert(gamepad);
            let gamepads = Gamepads {
                gamepads: gamepad_set,
            };
            assert!(gamepads.contains(&gamepad));
        }

        #[test]
        fn test_iter() {
            let mut gamepad_set = HashSet::default();
            gamepad_set.insert(Gamepad::new(1));
            gamepad_set.insert(Gamepad::new(2));
            gamepad_set.insert(Gamepad::new(3));

            let gamepads = Gamepads {
                gamepads: gamepad_set,
            };

            for gamepad in gamepads.iter() {
                assert!(gamepads.gamepads.contains(gamepad));
            }
        }

        #[test]
        fn test_register() {
            let gamepad = Gamepad::new(1);
            let mut gamepads = Gamepads::default();
            gamepads.register(gamepad);
            assert!(gamepads.gamepads.contains(&gamepad));
        }

        #[test]
        fn test_deregister() {
            let gamepad = Gamepad::new(1);
            let mut gamepad_set = HashSet::default();
            gamepad_set.insert(gamepad);
            let mut gamepads = Gamepads {
                gamepads: gamepad_set,
            };
            gamepads.deregister(&gamepad);
            assert!(!gamepads.gamepads.contains(&gamepad));
        }
    }
}
