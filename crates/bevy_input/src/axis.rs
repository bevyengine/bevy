//! The generic axis type.

use bevy_ecs::system::Resource;
use bevy_utils::HashMap;
use std::hash::Hash;

/// Stores the position data of the input devices of type `T`.
///
/// The values are stored as `f32`s, using [`Axis::set`].
/// Use [`Axis::get`] to retrieve the value clamped between [`Axis::MIN`] and [`Axis::MAX`]
/// inclusive, or unclamped using [`Axis::get_unclamped`].
#[derive(Debug, Resource)]
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
    /// The smallest possible axis value.
    pub const MIN: f32 = -1.0;

    /// The largest possible axis value.
    pub const MAX: f32 = 1.0;

    /// Sets the position data of the `input_device` to `position_data`.
    ///
    /// If the `input_device`:
    /// - was present before, the position data is updated, and the old value is returned.
    /// - wasn't present before, `None` is returned.
    pub fn set(&mut self, input_device: T, position_data: f32) -> Option<f32> {
        self.axis_data.insert(input_device, position_data)
    }

    /// Returns the position data of the provided `input_device`.
    ///
    /// This will be clamped between [`Axis::MIN`] and [`Axis::MAX`] inclusive.
    pub fn get(&self, input_device: T) -> Option<f32> {
        self.axis_data
            .get(&input_device)
            .copied()
            .map(|value| value.clamp(Self::MIN, Self::MAX))
    }

    /// Returns the unclamped position data of the provided `input_device`.
    ///
    /// This value may be outside the [`Axis::MIN`] and [`Axis::MAX`] range.
    ///
    /// Use for things like camera zoom, where you want devices like mouse wheels to be able to
    /// exceed the normal range. If being able to move faster on one input device
    /// than another would give an unfair advantage, you should likely use [`Axis::get`] instead.
    pub fn get_unclamped(&self, input_device: T) -> Option<f32> {
        self.axis_data.get(&input_device).copied()
    }

    /// Removes the position data of the `input_device`, returning the position data if the input device was previously set.
    pub fn remove(&mut self, input_device: T) -> Option<f32> {
        self.axis_data.remove(&input_device)
    }
}

#[cfg(test)]
mod tests {
    use crate::{gamepad::GamepadButtonType, Axis};

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
            let mut axis = Axis::<GamepadButtonType>::default();

            axis.set(GamepadButtonType::RightTrigger, value);

            let actual = axis.get(GamepadButtonType::RightTrigger);
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn test_axis_remove() {
        let cases = [-1.0, -0.9, -0.1, 0.0, 0.1, 0.9, 1.0];

        for value in cases {
            let mut axis = Axis::<GamepadButtonType>::default();

            axis.set(GamepadButtonType::RightTrigger, value);
            assert!(axis.get(GamepadButtonType::RightTrigger).is_some());

            axis.remove(GamepadButtonType::RightTrigger);
            let actual = axis.get(GamepadButtonType::RightTrigger);
            let expected = None;

            assert_eq!(expected, actual);
        }
    }
}
