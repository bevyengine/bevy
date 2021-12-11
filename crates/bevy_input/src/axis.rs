use bevy_utils::HashMap;
use std::hash::Hash;

/// Stores the position data of input devices of type T
///
/// Values are stored as `f32` values, which range from `min` to `max`.
/// The valid range is from -1.0 to 1.0, inclusive.
#[derive(Debug)]
pub struct Axis<T> {
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
    pub const MIN: f32 = -1.0;
    pub const MAX: f32 = 1.0;

    /// Inserts a position data for an input device,
    /// restricting the position data to an interval `min..=max`.
    ///
    /// If the input device wasn't present before, [None] is returned.
    ///
    /// If the input device was present, the position data is updated, and the old value is returned.
    pub fn set(&mut self, input_device: T, position_data: f32) -> Option<f32> {
        let new_position_data = position_data.clamp(Self::MIN, Self::MAX);
        self.axis_data.insert(input_device, new_position_data)
    }

    /// Returns a position data corresponding to the input device.
    pub fn get(&self, input_device: T) -> Option<f32> {
        self.axis_data.get(&input_device).copied()
    }

    /// Removes the position data of the input device,
    /// returning the position data if the input device was previously set.
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
    fn test_axis_set() {
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
            let gamepad_button = GamepadButton(Gamepad(1), GamepadButtonType::RightTrigger);
            let mut axis = Axis::<GamepadButton>::default();

            axis.set(gamepad_button, value);

            let actual = axis.get(gamepad_button);
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn test_axis_remove() {
        let cases = [-1.0, -0.9, -0.1, 0.0, 0.1, 0.9, 1.0];

        for value in cases {
            let gamepad_button = GamepadButton(Gamepad(1), GamepadButtonType::RightTrigger);
            let mut axis = Axis::<GamepadButton>::default();

            axis.set(gamepad_button, value);
            assert!(axis.get(gamepad_button).is_some());

            axis.remove(gamepad_button);
            let actual = axis.get(gamepad_button);
            let expected = None;

            assert_eq!(expected, actual);
        }
    }
}
