use crate::{Axis, Axislike, Input, Inputlike};
use bevy_app::{EventReader, EventWriter};
use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{HashMap, HashSet};

use strum_macros::EnumIter;

/// A unique identifier for a gamepad, assigned sequentially
///
/// These are managed through the use of the [Gamepads] resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Gamepad(pub usize);

#[derive(Default)]
/// Container of unique connected [Gamepad]s
///
/// Gamepads are registered and deregistered automatically in [gamepad_event_system],
/// which also updates the input values stored in `buttons` and `axes`.
pub struct Gamepads {
    gamepads: HashSet<Gamepad>,
    pub buttons: HashMap<Gamepad, Input<GamepadButton>>,
    pub axes: HashMap<Gamepad, Axis<GamepadAxis>>,
}

impl Gamepads {
    /// Returns true if the [Gamepads] contains a [Gamepad].
    pub fn contains(&self, gamepad: &Gamepad) -> bool {
        self.gamepads.contains(gamepad)
    }

    /// Iterates over registered [Gamepad]s
    pub fn iter(&self) -> impl Iterator<Item = &Gamepad> + '_ {
        self.gamepads.iter()
    }

    /// Registers [Gamepad].
    fn register(&mut self, gamepad: Gamepad) {
        self.gamepads.insert(gamepad);
        self.buttons.insert(gamepad, Input::default());
        self.axes.insert(gamepad, Axis::default());
    }

    /// Deregisters [Gamepad].
    fn deregister(&mut self, gamepad: Gamepad) {
        self.gamepads.remove(&gamepad);
        self.buttons.remove(&gamepad);
        self.axes.remove(&gamepad);
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadEventType {
    Connected,
    Disconnected,
    ButtonChanged(GamepadButton, f32),
    AxisChanged(GamepadAxis, f32),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEvent(pub Gamepad, pub GamepadEventType);

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEventRaw(pub Gamepad, pub GamepadEventType);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadButton {
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

impl Inputlike for GamepadButton {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    DPadX,
    DPadY,
}

impl Axislike for GamepadAxis {}

#[derive(Default, Debug)]
pub struct GamepadSettings {
    pub default_button_settings: ButtonSettings,
    pub default_axis_settings: AxisSettings,
    pub button_settings: HashMap<GamepadButton, ButtonSettings>,
    pub axis_settings: HashMap<GamepadAxis, AxisSettings>,
}

impl GamepadSettings {
    pub fn button_settings(&self, button: GamepadButton) -> &ButtonSettings {
        self.button_settings
            .get(&button)
            .unwrap_or(&self.default_button_settings)
    }

    pub fn axis_settings(&self, axis: GamepadAxis) -> &AxisSettings {
        self.axis_settings
            .get(&axis)
            .unwrap_or(&self.default_axis_settings)
    }
}

#[derive(Debug, Clone)]
pub struct ButtonSettings {
    pub press: f32,
    pub release: f32,
}

impl Default for ButtonSettings {
    fn default() -> Self {
        ButtonSettings {
            press: 0.75,
            release: 0.65,
        }
    }
}

impl ButtonSettings {
    fn is_pressed(&self, value: f32) -> bool {
        value >= self.press
    }

    fn is_released(&self, value: f32) -> bool {
        value <= self.release
    }
}

/// Defines the sensitivity range and threshold for an axis.
///
/// Values that are lower than `negative_high` will be rounded to -1.0.
/// Values that are higher than `positive_high` will be rounded to 1.0.
/// Values that are in-between `negative_low` and `positive_low` will be rounded to 0.0.
/// Otherwise, values will not be rounded.
///
/// The valid range is from -1.0 to 1.0, inclusive.
#[derive(Debug, Clone)]
pub struct AxisSettings {
    pub positive_high: f32,
    pub positive_low: f32,
    pub negative_high: f32,
    pub negative_low: f32,
    ///`threshold` defines the minimum difference between old and new values to apply the changes.
    pub threshold: f32,
}

impl Default for AxisSettings {
    fn default() -> Self {
        AxisSettings {
            positive_high: 0.95,
            positive_low: 0.05,
            negative_high: -0.95,
            negative_low: -0.05,
            threshold: 0.01,
        }
    }
}

impl AxisSettings {
    fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value = if new_value <= self.positive_low && new_value >= self.negative_low {
            0.0
        } else if new_value >= self.positive_high {
            1.0
        } else if new_value <= self.negative_high {
            -1.0
        } else {
            new_value
        };

        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return None;
            }
        }

        Some(new_value)
    }
}

/// Processes raw gamepad events, updating input state to reflect the received events.
///
/// Updates both button inputs and axes.
/// Monitors gamepad connection and disconnection events, updating the [`Gamepads`] resource accordingly.
/// Takes [GamepadEventRaw][ and outputs processed [GamepadEvent], which reflect [GamepadSettings] correctly.
///
/// By default, runs during `CoreStage::PreUpdate` when added via [`InputPlugin`](crate::InputPlugin).
pub fn gamepad_event_system(
    mut gamepads: ResMut<Gamepads>,
    mut raw_events: EventReader<GamepadEventRaw>,
    mut events: EventWriter<GamepadEvent>,
    settings: Res<GamepadSettings>,
) {
    // Reset the buttons each frame so buttons are correctly just-pressed and just-released
    for (_gamepad, button_input) in gamepads.buttons.iter_mut() {
        button_input.clear();
    }

    for raw_event in raw_events.iter() {
        let (gamepad, event) = (raw_event.0, &raw_event.1);

        match event {
            GamepadEventType::Connected => {
                gamepads.register(gamepad);
                events.send(GamepadEvent(gamepad, event.clone()));
            }
            GamepadEventType::Disconnected => {
                gamepads.deregister(gamepad);
                events.send(GamepadEvent(gamepad, event.clone()));
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let axes = gamepads
                    .axes
                    .get_mut(&gamepad)
                    .expect("Gamepad axes were not registered correctly.");
                if let Some(filtered_value) = settings
                    .axis_settings(*axis_type)
                    .filter(*value, axes.get(*axis_type))
                {
                    axes.set(*axis_type, filtered_value);
                    events.send(GamepadEvent(
                        gamepad,
                        GamepadEventType::AxisChanged(*axis_type, filtered_value),
                    ))
                }
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let button_input = gamepads
                    .buttons
                    .get_mut(&gamepad)
                    .expect("Gamepad buttons were not registered correctly.");

                button_input.set_value(*button_type, *value);

                let button_property = settings.button_settings(*button_type);
                if button_input.pressed(*button_type) {
                    if button_property.is_released(*value) {
                        button_input.release(*button_type);
                    }
                } else if button_property.is_pressed(*value) {
                    button_input.press(*button_type);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AxisSettings, ButtonSettings};

    fn test_axis_settings_filter(
        settings: AxisSettings,
        new_value: f32,
        old_value: Option<f32>,
        expected: Option<f32>,
    ) {
        let actual = settings.filter(new_value, old_value);
        assert_eq!(
            expected, actual,
            "Testing filtering for {:?} with new_value = {:?}, old_value = {:?}",
            settings, new_value, old_value
        );
    }

    #[test]
    fn test_axis_settings_default_filter() {
        let cases = [
            (1.0, Some(1.0)),
            (0.99, Some(1.0)),
            (0.96, Some(1.0)),
            (0.95, Some(1.0)),
            (0.9499, Some(0.9499)),
            (0.84, Some(0.84)),
            (0.43, Some(0.43)),
            (0.05001, Some(0.05001)),
            (0.05, Some(0.0)),
            (0.04, Some(0.0)),
            (0.01, Some(0.0)),
            (0.0, Some(0.0)),
            (-1.0, Some(-1.0)),
            (-0.99, Some(-1.0)),
            (-0.96, Some(-1.0)),
            (-0.95, Some(-1.0)),
            (-0.9499, Some(-0.9499)),
            (-0.84, Some(-0.84)),
            (-0.43, Some(-0.43)),
            (-0.05001, Some(-0.05001)),
            (-0.05, Some(0.0)),
            (-0.04, Some(0.0)),
            (-0.01, Some(0.0)),
        ];

        for (new_value, expected) in cases {
            let settings = AxisSettings::default();
            test_axis_settings_filter(settings, new_value, None, expected);
        }
    }

    #[test]
    fn test_axis_settings_default_filter_with_old_values() {
        let cases = [
            (0.43, Some(0.44001), Some(0.43)),
            (0.43, Some(0.44), None),
            (0.43, Some(0.43), None),
            (0.43, Some(0.41999), Some(0.43)),
            (0.43, Some(0.17), Some(0.43)),
            (0.43, Some(0.84), Some(0.43)),
            (0.05, Some(0.055), Some(0.0)),
            (0.95, Some(0.945), Some(1.0)),
            (-0.43, Some(-0.44001), Some(-0.43)),
            (-0.43, Some(-0.44), None),
            (-0.43, Some(-0.43), None),
            (-0.43, Some(-0.41999), Some(-0.43)),
            (-0.43, Some(-0.17), Some(-0.43)),
            (-0.43, Some(-0.84), Some(-0.43)),
            (-0.05, Some(-0.055), Some(0.0)),
            (-0.95, Some(-0.945), Some(-1.0)),
        ];

        for (new_value, old_value, expected) in cases {
            let settings = AxisSettings::default();
            test_axis_settings_filter(settings, new_value, old_value, expected);
        }
    }

    #[test]
    fn test_button_settings_default_is_pressed() {
        let cases = [
            (1.0, true),
            (0.95, true),
            (0.9, true),
            (0.8, true),
            (0.75, true),
            (0.7, false),
            (0.65, false),
            (0.5, false),
            (0.0, false),
        ];

        for (value, expected) in cases {
            let settings = ButtonSettings::default();
            let actual = settings.is_pressed(value);

            assert_eq!(expected, actual, "Testing is pressed for value: {}", value);
        }
    }

    #[test]
    fn test_button_settings_default_is_released() {
        let cases = [
            (1.0, false),
            (0.95, false),
            (0.9, false),
            (0.8, false),
            (0.75, false),
            (0.7, false),
            (0.65, true),
            (0.5, true),
            (0.0, true),
        ];

        for (value, expected) in cases {
            let settings = ButtonSettings::default();
            let actual = settings.is_released(value);

            assert_eq!(expected, actual, "Testing is released for value: {}", value);
        }
    }
}
