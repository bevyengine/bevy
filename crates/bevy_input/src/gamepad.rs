use crate::{Axis, Input};
use bevy_app::{EventReader, EventWriter};
use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{tracing::info, HashMap, HashSet};
use thiserror::Error;

/// Errors that occur when setting settings for gamepad input
#[derive(Error, Debug)]
pub enum GamepadSettingsError {
    #[error("{0}")]
    InvalidAxisSetting(String),
}

type Result<T, E = GamepadSettingsError> = std::result::Result<T, E>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Gamepad(pub usize);

#[derive(Default)]
/// Container of unique connected [Gamepad]s
///
/// [Gamepad]s are registered and deregistered in [gamepad_connection_system]
pub struct Gamepads {
    gamepads: HashSet<Gamepad>,
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
    }

    /// Deregisters [Gamepad.
    fn deregister(&mut self, gamepad: &Gamepad) {
        self.gamepads.remove(gamepad);
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadEventType {
    Connected,
    Disconnected,
    ButtonChanged(GamepadButtonType, f32),
    AxisChanged(GamepadAxisType, f32),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEvent(pub Gamepad, pub GamepadEventType);

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEventRaw(pub Gamepad, pub GamepadEventType);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadButtonType {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadButton(pub Gamepad, pub GamepadButtonType);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadAxisType {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    DPadX,
    DPadY,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadAxis(pub Gamepad, pub GamepadAxisType);

#[derive(Default, Debug)]
pub struct GamepadSettings {
    pub default_button_settings: ButtonSettings,
    pub default_axis_settings: AxisSettings,
    pub default_button_axis_settings: ButtonAxisSettings,
    pub button_settings: HashMap<GamepadButton, ButtonSettings>,
    pub axis_settings: HashMap<GamepadAxis, AxisSettings>,
    pub button_axis_settings: HashMap<GamepadButton, ButtonAxisSettings>,
}

impl GamepadSettings {
    pub fn get_button_settings(&self, button: GamepadButton) -> &ButtonSettings {
        self.button_settings
            .get(&button)
            .unwrap_or(&self.default_button_settings)
    }

    pub fn get_axis_settings(&self, axis: GamepadAxis) -> &AxisSettings {
        self.axis_settings
            .get(&axis)
            .unwrap_or(&self.default_axis_settings)
    }

    pub fn get_button_axis_settings(&self, button: GamepadButton) -> &ButtonAxisSettings {
        self.button_axis_settings
            .get(&button)
            .unwrap_or(&self.default_button_axis_settings)
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
/// Values that are lower than `livezone_lowerbound` will be rounded to -1.0.
/// Values that are higher than `livezone_upperbound` will be rounded to 1.0.
/// Values that are in-between `deadzone_lowerbound` and `deadzone_upperbound` will be rounded to 0.0.
/// Otherwise, values will not be rounded.
///
/// The valid range is from -1.0 to 1.0, inclusive.
#[derive(Debug, Clone)]
pub struct AxisSettings {
    livezone_upperbound: f32,
    deadzone_upperbound: f32,
    deadzone_lowerbound: f32,
    livezone_lowerbound: f32,
    ///`threshold` defines the minimum difference between old and new values to apply the changes.
    threshold: f32,
}

impl Default for AxisSettings {
    fn default() -> Self {
        AxisSettings {
            livezone_upperbound: 0.95,
            deadzone_upperbound: 0.05,
            livezone_lowerbound: -0.95,
            deadzone_lowerbound: -0.05,
            threshold: 0.01,
        }
    }
}

impl AxisSettings {
    /// Get the value above which inputs will be rounded up to 1.0
    pub fn livezone_upperbound(&self) -> f32 {
        self.livezone_upperbound
    }

    /// Try to set the value above which inputs will be rounded up to 1.0
    ///
    /// # Errors
    ///
    /// If the value passed is less than deadzone_upperbound or greater than 1.0
    pub fn try_set_livezone_upperbound(&mut self, value: f32) -> Result<()> {
        if value < self.deadzone_upperbound || value > 1.0 {
            Err(GamepadSettingsError::InvalidAxisSetting(
                "livezone_upperbound must be greater than deadzone_upperbound and less than 1.0"
                    .to_owned(),
            ))
        } else {
            self.livezone_upperbound = value;
            Ok(())
        }
    }

    /// Try to set the value above which inputs will be rounded up to 1.0. If the value is less than
    /// deadzone_upperbound or greater than 1.0, the value will not be changed.
    ///
    /// Returns the new value of livezone_upperbound.
    pub fn set_livezone_upperbound(&mut self, value: f32) -> f32 {
        self.try_set_livezone_upperbound(value).ok();
        self.livezone_upperbound
    }

    /// Get the value below which positive inputs will be rounded down to 0.0
    pub fn deadzone_upperbound(&mut self) -> f32 {
        self.deadzone_upperbound
    }

    /// Try to set the value below which positive inputs will be rounded down to 0.0
    ///
    /// # Errors
    ///
    /// If the value passed is negative or greater than livezone_upperbound
    pub fn try_set_deadzone_upperbound(&mut self, value: f32) -> Result<()> {
        if value < 0.0 || value > self.livezone_upperbound {
            Err(GamepadSettingsError::InvalidAxisSetting(
                "deadzone_upperbound must be positive and less than livezone_upperbound".to_owned(),
            ))
        } else {
            self.deadzone_upperbound = value;
            Ok(())
        }
    }

    /// Try to set the value below which positive inputs will be rounded down to 0.0. If the value
    /// passed is negative or greater than livezone_upperbound, the value will not be changed.
    ///
    /// Returns the new value of deadzone_upperbound.
    pub fn set_deadzone_upperbound(&mut self, value: f32) -> f32 {
        self.try_set_deadzone_upperbound(value).ok();
        self.deadzone_upperbound
    }

    /// Get the value above which negative inputs will be rounded up to 0.0
    pub fn deadzone_lowerbound(&self) -> f32 {
        self.deadzone_lowerbound
    }

    /// Try to set the value above which negative inputs will be rounded up to 0.0
    ///
    /// # Errors
    ///
    /// If the value passed is positive or less than livezone_lowerbound
    pub fn try_set_deadzone_lowerbound(&mut self, value: f32) -> Result<()> {
        if value < self.livezone_lowerbound || value > 0.0 {
            Err(GamepadSettingsError::InvalidAxisSetting(
                "deadzone_lowerbound must be negative and greater than livezone_lowerbound"
                    .to_owned(),
            ))
        } else {
            self.deadzone_lowerbound = value;
            Ok(())
        }
    }

    /// Try to set the value above which negative inputs will be rounded up to 0.0. If the value
    /// passed is positive or less than livezone_lowerbound, the value will not be changed.
    ///
    /// Returns the new value of deadzone_lowerbound.
    pub fn set_deadzone_lowerbound(&mut self, value: f32) -> f32 {
        self.try_set_deadzone_lowerbound(value).ok();
        self.deadzone_lowerbound
    }

    /// Get the value below which inputs will be rounded down to -1.0
    pub fn livezone_lowerbound(&self) -> f32 {
        self.livezone_lowerbound
    }

    /// Try to get the value below which inputs will be rounded down to -1.0
    ///
    /// # Errors
    ///
    /// If the value passed is less than -1.0 or greater than deadzone_lowerbound
    pub fn try_set_livezone_lowerbound(&mut self, value: f32) -> Result<()> {
        if value < -1.0 || value > self.deadzone_lowerbound {
            Err(GamepadSettingsError::InvalidAxisSetting(
                "livezone_lowerbound must be greater than -1.0 and less than deadzone_lowerbound"
                    .to_owned(),
            ))
        } else {
            self.livezone_lowerbound = value;
            Ok(())
        }
    }

    /// Try to set the value below which inputs will be rounded down to -1.0. If the value passed is
    /// less than -1.0 or greater than deadzone_lowerbound, the value will not be changed.
    ///
    /// Returns the new value of livezone_lowerbound.
    pub fn set_livezone_lowerbound(&mut self, value: f32) -> f32 {
        self.try_set_livezone_lowerbound(value).ok();
        self.livezone_lowerbound
    }

    /// Get the minimum value by which input must change before the changes will be applied
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Try to set the minimum value by which input must change before the changes will be applied
    ///
    /// # Errors
    ///
    /// If the value passed is not within [0.0, 2.0]
    pub fn try_set_threshold(&mut self, value: f32) -> Result<()> {
        if !(0.0..=2.0).contains(&value) {
            Err(GamepadSettingsError::InvalidAxisSetting(
                "threshold must be between 0.0 and 2.0, inclusive".to_owned(),
            ))
        } else {
            self.threshold = value;
            Ok(())
        }
    }

    /// Try to set the minimum value by which input must change before the changes will be applied.
    /// If the value passed is not within [0.0, 2.0], the value will not be changed.
    ///
    /// Returns the new value of threshold.
    pub fn set_threshold(&mut self, value: f32) -> f32 {
        self.try_set_threshold(value).ok();
        self.threshold
    }

    fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value =
            if new_value <= self.deadzone_upperbound && new_value >= self.deadzone_lowerbound {
                0.0
            } else if new_value >= self.livezone_upperbound {
                1.0
            } else if new_value <= self.livezone_lowerbound {
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

#[derive(Debug, Clone)]
pub struct ButtonAxisSettings {
    pub high: f32,
    pub low: f32,
    pub threshold: f32,
}

impl Default for ButtonAxisSettings {
    fn default() -> Self {
        ButtonAxisSettings {
            high: 0.95,
            low: 0.05,
            threshold: 0.01,
        }
    }
}

impl ButtonAxisSettings {
    fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value = if new_value <= self.low {
            0.0
        } else if new_value >= self.high {
            1.0
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

/// Monitors gamepad connection and disconnection events, updating the [`Gamepads`] resource accordingly
///
/// By default, runs during `CoreStage::PreUpdate` when added via [`InputPlugin`](crate::InputPlugin).
pub fn gamepad_connection_system(
    mut gamepads: ResMut<Gamepads>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.iter() {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                gamepads.register(*gamepad);
                info!("{:?} Connected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                gamepads.deregister(gamepad);
                info!("{:?} Disconnected", gamepad);
            }
            _ => (),
        }
    }
}

pub fn gamepad_event_system(
    mut button_input: ResMut<Input<GamepadButton>>,
    mut axis: ResMut<Axis<GamepadAxis>>,
    mut button_axis: ResMut<Axis<GamepadButton>>,
    mut raw_events: EventReader<GamepadEventRaw>,
    mut events: EventWriter<GamepadEvent>,
    settings: Res<GamepadSettings>,
) {
    button_input.clear();
    for event in raw_events.iter() {
        let (gamepad, event) = (event.0, &event.1);
        match event {
            GamepadEventType::Connected => {
                events.send(GamepadEvent(gamepad, event.clone()));
                for button_type in ALL_BUTTON_TYPES.iter() {
                    let gamepad_button = GamepadButton(gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.set(gamepad_button, 0.0);
                }
                for axis_type in ALL_AXIS_TYPES.iter() {
                    axis.set(GamepadAxis(gamepad, *axis_type), 0.0);
                }
            }
            GamepadEventType::Disconnected => {
                events.send(GamepadEvent(gamepad, event.clone()));
                for button_type in ALL_BUTTON_TYPES.iter() {
                    let gamepad_button = GamepadButton(gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.remove(gamepad_button);
                }
                for axis_type in ALL_AXIS_TYPES.iter() {
                    axis.remove(GamepadAxis(gamepad, *axis_type));
                }
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let gamepad_axis = GamepadAxis(gamepad, *axis_type);
                if let Some(filtered_value) = settings
                    .get_axis_settings(gamepad_axis)
                    .filter(*value, axis.get(gamepad_axis))
                {
                    axis.set(gamepad_axis, filtered_value);
                    events.send(GamepadEvent(
                        gamepad,
                        GamepadEventType::AxisChanged(*axis_type, filtered_value),
                    ))
                }
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let gamepad_button = GamepadButton(gamepad, *button_type);
                if let Some(filtered_value) = settings
                    .get_button_axis_settings(gamepad_button)
                    .filter(*value, button_axis.get(gamepad_button))
                {
                    button_axis.set(gamepad_button, filtered_value);
                    events.send(GamepadEvent(
                        gamepad,
                        GamepadEventType::ButtonChanged(*button_type, filtered_value),
                    ))
                }

                let button_property = settings.get_button_settings(gamepad_button);
                if button_input.pressed(gamepad_button) {
                    if button_property.is_released(*value) {
                        button_input.release(gamepad_button);
                    }
                } else if button_property.is_pressed(*value) {
                    button_input.press(gamepad_button);
                }
            }
        }
    }
}

const ALL_BUTTON_TYPES: [GamepadButtonType; 19] = [
    GamepadButtonType::South,
    GamepadButtonType::East,
    GamepadButtonType::North,
    GamepadButtonType::West,
    GamepadButtonType::C,
    GamepadButtonType::Z,
    GamepadButtonType::LeftTrigger,
    GamepadButtonType::LeftTrigger2,
    GamepadButtonType::RightTrigger,
    GamepadButtonType::RightTrigger2,
    GamepadButtonType::Select,
    GamepadButtonType::Start,
    GamepadButtonType::Mode,
    GamepadButtonType::LeftThumb,
    GamepadButtonType::RightThumb,
    GamepadButtonType::DPadUp,
    GamepadButtonType::DPadDown,
    GamepadButtonType::DPadLeft,
    GamepadButtonType::DPadRight,
];

const ALL_AXIS_TYPES: [GamepadAxisType; 8] = [
    GamepadAxisType::LeftStickX,
    GamepadAxisType::LeftStickY,
    GamepadAxisType::LeftZ,
    GamepadAxisType::RightStickX,
    GamepadAxisType::RightStickY,
    GamepadAxisType::RightZ,
    GamepadAxisType::DPadX,
    GamepadAxisType::DPadY,
];

#[cfg(test)]
mod tests {
    use super::{AxisSettings, ButtonAxisSettings, ButtonSettings};

    fn test_button_axis_settings_filter(
        settings: ButtonAxisSettings,
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
    fn test_button_axis_settings_default_filter() {
        let cases = [
            (1.0, None, Some(1.0)),
            (0.99, None, Some(1.0)),
            (0.96, None, Some(1.0)),
            (0.95, None, Some(1.0)),
            (0.9499, None, Some(0.9499)),
            (0.84, None, Some(0.84)),
            (0.43, None, Some(0.43)),
            (0.05001, None, Some(0.05001)),
            (0.05, None, Some(0.0)),
            (0.04, None, Some(0.0)),
            (0.01, None, Some(0.0)),
            (0.0, None, Some(0.0)),
        ];

        for (new_value, old_value, expected) in cases {
            let settings = ButtonAxisSettings::default();
            test_button_axis_settings_filter(settings, new_value, old_value, expected);
        }
    }

    #[test]
    fn test_button_axis_settings_default_filter_with_old_value() {
        let cases = [
            (0.43, Some(0.44001), Some(0.43)),
            (0.43, Some(0.44), None),
            (0.43, Some(0.43), None),
            (0.43, Some(0.41999), Some(0.43)),
            (0.43, Some(0.17), Some(0.43)),
            (0.43, Some(0.84), Some(0.43)),
            (0.05, Some(0.055), Some(0.0)),
            (0.95, Some(0.945), Some(1.0)),
        ];

        for (new_value, old_value, expected) in cases {
            let settings = ButtonAxisSettings::default();
            test_button_axis_settings_filter(settings, new_value, old_value, expected);
        }
    }

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
