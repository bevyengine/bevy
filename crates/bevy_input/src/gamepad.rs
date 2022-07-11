use crate::{Axis, Input};
use bevy_ecs::event::{EventReader, EventWriter};
use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{tracing::info, HashMap, HashSet};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Gamepad {
    pub id: usize,
}

impl Gamepad {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

#[derive(Default)]
/// Container of unique connected [`Gamepad`]s
///
/// [`Gamepad`]s are registered and deregistered in [`gamepad_connection_system`]
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
pub struct GamepadEvent {
    pub gamepad: Gamepad,
    pub event_type: GamepadEventType,
}

impl GamepadEvent {
    pub fn new(gamepad: Gamepad, event_type: GamepadEventType) -> Self {
        Self {
            gamepad,
            event_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEventRaw {
    pub gamepad: Gamepad,
    pub event_type: GamepadEventType,
}

impl GamepadEventRaw {
    pub fn new(gamepad: Gamepad, event_type: GamepadEventType) -> Self {
        Self {
            gamepad,
            event_type,
        }
    }
}

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
pub struct GamepadButton {
    pub gamepad: Gamepad,
    pub button_type: GamepadButtonType,
}

impl GamepadButton {
    pub fn new(gamepad: Gamepad, button_type: GamepadButtonType) -> Self {
        Self {
            gamepad,
            button_type,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadAxisType {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadAxis {
    pub gamepad: Gamepad,
    pub axis_type: GamepadAxisType,
}

impl GamepadAxis {
    pub fn new(gamepad: Gamepad, axis_type: GamepadAxisType) -> Self {
        Self { gamepad, axis_type }
    }
}

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
        match event.event_type {
            GamepadEventType::Connected => {
                gamepads.register(event.gamepad);
                info!("{:?} Connected", event.gamepad);
            }
            GamepadEventType::Disconnected => {
                gamepads.deregister(&event.gamepad);
                info!("{:?} Disconnected", event.gamepad);
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
        match event.event_type {
            GamepadEventType::Connected => {
                events.send(GamepadEvent::new(event.gamepad, event.event_type.clone()));
                for button_type in &ALL_BUTTON_TYPES {
                    let gamepad_button = GamepadButton::new(event.gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.set(gamepad_button, 0.0);
                }
                for axis_type in &ALL_AXIS_TYPES {
                    axis.set(GamepadAxis::new(event.gamepad, *axis_type), 0.0);
                }
            }
            GamepadEventType::Disconnected => {
                events.send(GamepadEvent::new(event.gamepad, event.event_type.clone()));
                for button_type in &ALL_BUTTON_TYPES {
                    let gamepad_button = GamepadButton::new(event.gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.remove(gamepad_button);
                }
                for axis_type in &ALL_AXIS_TYPES {
                    axis.remove(GamepadAxis::new(event.gamepad, *axis_type));
                }
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let gamepad_axis = GamepadAxis::new(event.gamepad, axis_type);
                if let Some(filtered_value) = settings
                    .get_axis_settings(gamepad_axis)
                    .filter(value, axis.get(gamepad_axis))
                {
                    axis.set(gamepad_axis, filtered_value);
                    events.send(GamepadEvent::new(
                        event.gamepad,
                        GamepadEventType::AxisChanged(axis_type, filtered_value),
                    ));
                }
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let gamepad_button = GamepadButton::new(event.gamepad, button_type);
                if let Some(filtered_value) = settings
                    .get_button_axis_settings(gamepad_button)
                    .filter(value, button_axis.get(gamepad_button))
                {
                    button_axis.set(gamepad_button, filtered_value);
                    events.send(GamepadEvent::new(
                        event.gamepad,
                        GamepadEventType::ButtonChanged(button_type, filtered_value),
                    ));
                }

                let button_property = settings.get_button_settings(gamepad_button);
                if button_input.pressed(gamepad_button) {
                    if button_property.is_released(value) {
                        button_input.release(gamepad_button);
                    }
                } else if button_property.is_pressed(value) {
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

const ALL_AXIS_TYPES: [GamepadAxisType; 6] = [
    GamepadAxisType::LeftStickX,
    GamepadAxisType::LeftStickY,
    GamepadAxisType::LeftZ,
    GamepadAxisType::RightStickX,
    GamepadAxisType::RightStickY,
    GamepadAxisType::RightZ,
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
