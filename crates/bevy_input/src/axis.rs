use bevy_utils::HashMap;
use std::hash::Hash;

/// Stores the position data of the input devices of type `T`.
///
/// Values are stored as `f32` values, which range from -1.0 to 1.0, inclusive.
#[derive(Debug)]
pub struct Axis<T> {
    /// The position data of the input devices.
    axis_data: HashMap<T, f32>,
}

impl<T> Default for Axis<T>
where
    T: Copy + Eq + Hash,
{
    fn default() -> Self {
        Axis {
            axis_data: HashMap::default(),
        }
    }
}

impl<T> Axis<T>
where
    T: Copy + Eq + Hash,
{
    /// Inserts a position data for an input device, restricting the position data to an interval `min..=max`.
    ///
    /// If the `input_device`:
    /// - was present before, the position data is updated, and the old value is returned.
    /// - wasn't present before, [None] is returned.
    pub fn set(&mut self, input_device: T, position_data: f32) -> Option<f32> {
        let new_position_data = position_data.clamp(-1.0, 1.0);
        self.axis_data.insert(input_device, new_position_data)
    }

    /// Returns a position data corresponding to the input device.
    pub fn get(&self, input_device: T) -> Option<f32> {
        self.axis_data.get(&input_device).copied()
    }

    /// Removes the position data of the input device, returning the position data if the input device was previously set.
    pub fn remove(&mut self, input_device: T) -> Option<f32> {
        self.axis_data.remove(&input_device)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        gamepad::{Gamepad, GamepadButton, GamepadButtonType},
        Axis,
    };

    #[test]
    fn test_set() {
        let cases = [
            (-1.5, Some(-1.0)),
            (-1.1, Some(-1.0)),
            (-1.0, Some(-1.0)),
            (-0.9, Some(-0.9)),
            (-0.1, Some(-0.1)),
            (0.0, Some(0.0)),
            (0.1, Some(0.1)),
            (0.9, Some(0.9)),
            (1.0, Some(1.0)),
            (1.1, Some(1.0)),
            (1.6, Some(1.0)),
        ];

        for (value, expected) in cases {
            let gamepad_button =
                GamepadButton::new(Gamepad::new(1), GamepadButtonType::RightTrigger);
            let mut axis = Axis::<GamepadButton>::default();

            axis.set(gamepad_button, value);

            let actual = axis.get(gamepad_button);
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn test_get() {
        let cases = [-1.0, -0.9, -0.1, 0.0, 0.1, 0.9, 1.0];

        for value in cases {
            let mut axis = Axis::<GamepadButton>::default();
            let button = GamepadButton::new(Gamepad::new(1), GamepadButtonType::RightTrigger);
            axis.axis_data.insert(button, value);
            assert_eq!(axis.get(button).unwrap(), value);
        }
    }

    #[test]
    fn test_remove() {
        let cases = [-1.0, -0.9, -0.1, 0.0, 0.1, 0.9, 1.0];

        for value in cases {
            let button = GamepadButton::new(Gamepad::new(1), GamepadButtonType::RightTrigger);
            let mut axis = Axis::<GamepadButton>::default();

            axis.set(button, value);
            assert!(axis.get(button).is_some());

            axis.remove(button);
            let actual = axis.get(button);
            let expected = None;

            assert_eq!(expected, actual);
        }
    }
}
