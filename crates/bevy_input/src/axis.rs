use bevy_utils::HashMap;
use std::hash::Hash;

/// Stores the position data of input devices of type T
///
/// Values are stored as `f32` values, which range from `min` to `max`.
/// The default valid range is from -1.0 to 1.0, inclusive.
#[derive(Debug)]
pub struct Axis<T> {
    axis_data: HashMap<T, f32>,
    min: f32,
    max: f32,
}

impl<T> Default for Axis<T>
where
    T: Copy + Eq + Hash,
{
    fn default() -> Self {
        Axis {
            axis_data: HashMap::default(),
            min: AXIS_MIN,
            max: AXIS_MAX,
        }
    }
}

impl<T> Axis<T>
where
    T: Copy + Eq + Hash,
{
    pub fn set(&mut self, axis: T, value: f32) -> Option<f32> {
        if value < self.min || value > self.max {
            None
        } else {
            self.axis_data.insert(axis, value)
        }
    }

    pub fn get(&self, axis: T) -> Option<f32> {
        self.axis_data.get(&axis).copied()
    }

    pub fn remove(&mut self, axis: T) -> Option<f32> {
        self.axis_data.remove(&axis)
    }

    pub fn get_min(&self) -> f32 {
        self.min
    }

    pub fn get_max(&self) -> f32 {
        self.max
    }
}

const AXIS_MIN: f32 = -1.0;
const AXIS_MAX: f32 = 1.0;

#[cfg(test)]
mod tests {
    use crate::{
        axis::AXIS_MAX,
        gamepad::{Gamepad, GamepadButton, GamepadButtonType},
        Axis,
    };

    use super::AXIS_MIN;

    #[test]
    fn test_axis_set() {
        let cases = [
            (-1.5, None),
            (-1.1, None),
            (-1.0, Some(-1.0)),
            (-0.9, Some(-0.9)),
            (-0.1, Some(-0.1)),
            (0.0, Some(0.0)),
            (0.1, Some(0.1)),
            (0.9, Some(0.9)),
            (1.0, Some(1.0)),
            (1.1, None),
            (1.6, None),
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

    #[test]
    fn test_axis_min() {
        let expected = AXIS_MIN;
        let axis = Axis::<GamepadButton>::default();

        let actual = axis.get_min();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_axis_max() {
        let expected = AXIS_MAX;
        let axis = Axis::<GamepadButton>::default();

        let actual = axis.get_max();
        assert_eq!(expected, actual);
    }
}
