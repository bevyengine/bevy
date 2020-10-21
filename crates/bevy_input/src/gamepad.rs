use crate::{Axis, Input};
use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_utils::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Gamepad(pub usize);

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
pub struct GamepadSetting {
    pub default_button_setting: ButtonSetting,
    pub default_axis_setting: AxisSetting,
    pub default_button_axis_setting: ButtonAxisSetting,
    pub button_settings: HashMap<GamepadButton, ButtonSetting>,
    pub axis_settings: HashMap<GamepadAxis, AxisSetting>,
    pub button_axis_settings: HashMap<GamepadButton, ButtonAxisSetting>,
}

impl GamepadSetting {
    pub fn get_button_setting(&self, button: GamepadButton) -> &ButtonSetting {
        self.button_settings
            .get(&button)
            .unwrap_or(&self.default_button_setting)
    }

    pub fn get_axis_setting(&self, axis: GamepadAxis) -> &AxisSetting {
        self.axis_settings
            .get(&axis)
            .unwrap_or(&self.default_axis_setting)
    }

    pub fn get_button_axis_setting(&self, button: GamepadButton) -> &ButtonAxisSetting {
        self.button_axis_settings
            .get(&button)
            .unwrap_or(&self.default_button_axis_setting)
    }
}

#[derive(Debug, Clone)]
pub struct ButtonSetting {
    pub press: f32,
    pub release: f32,
}

impl Default for ButtonSetting {
    fn default() -> Self {
        ButtonSetting {
            press: 0.75,
            release: 0.65,
        }
    }
}

impl ButtonSetting {
    fn is_pressed(&self, value: f32) -> bool {
        value >= self.press
    }

    fn is_released(&self, value: f32) -> bool {
        value <= self.release
    }
}

#[derive(Debug, Clone)]
pub struct AxisSetting {
    pub positive_high: f32,
    pub positive_low: f32,
    pub negative_high: f32,
    pub negative_low: f32,
    pub threshold: f32,
}

impl Default for AxisSetting {
    fn default() -> Self {
        AxisSetting {
            positive_high: 0.95,
            positive_low: 0.05,
            negative_high: -0.95,
            negative_low: -0.05,
            threshold: 0.01,
        }
    }
}

impl AxisSetting {
    fn filter(&self, new_value: f32, old_value: Option<f32>) -> f32 {
        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return old_value;
            }
        }
        if new_value <= self.positive_low && new_value >= self.negative_low {
            return 0.0;
        }
        if new_value >= self.positive_high {
            return 1.0;
        }
        if new_value <= self.negative_high {
            return -1.0;
        }
        new_value
    }
}

#[derive(Debug, Clone)]
pub struct ButtonAxisSetting {
    pub high: f32,
    pub low: f32,
    pub threshold: f32,
}

impl Default for ButtonAxisSetting {
    fn default() -> Self {
        ButtonAxisSetting {
            high: 0.95,
            low: 0.05,
            threshold: 0.01,
        }
    }
}

impl ButtonAxisSetting {
    fn filter(&self, new_value: f32, old_value: Option<f32>) -> f32 {
        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return old_value;
            }
        }
        if new_value <= self.low {
            return 0.0;
        }
        if new_value >= self.high {
            return 1.0;
        }
        new_value
    }
}

#[derive(Default)]
pub struct GamepadEventState {
    gamepad_event_reader: EventReader<GamepadEvent>,
}

pub fn gamepad_event_system(
    mut state: Local<GamepadEventState>,
    mut button_input: ResMut<Input<GamepadButton>>,
    mut axis: ResMut<Axis<GamepadAxis>>,
    mut button_axis: ResMut<Axis<GamepadButton>>,
    events: Res<Events<GamepadEvent>>,
    settings: Res<GamepadSetting>,
) {
    button_input.update();
    for event in state.gamepad_event_reader.iter(&events) {
        let (gamepad, event) = (&event.0, &event.1);
        match event {
            GamepadEventType::Connected => {
                for button_type in ALL_BUTTON_TYPES.iter() {
                    let gamepad_button = GamepadButton(*gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.set(gamepad_button, 0.0);
                }
                for axis_type in ALL_AXIS_TYPES.iter() {
                    axis.set(GamepadAxis(*gamepad, *axis_type), 0.0);
                }
            }
            GamepadEventType::Disconnected => {
                for button_type in ALL_BUTTON_TYPES.iter() {
                    let gamepad_button = GamepadButton(*gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.remove(gamepad_button);
                }
                for axis_type in ALL_AXIS_TYPES.iter() {
                    axis.remove(GamepadAxis(*gamepad, *axis_type));
                }
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let gamepad_axis = GamepadAxis(*gamepad, *axis_type);
                let value = settings
                    .get_axis_setting(gamepad_axis)
                    .filter(*value, axis.get(gamepad_axis));
                axis.set(gamepad_axis, value);
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let gamepad_button = GamepadButton(*gamepad, *button_type);
                let filtered_value = settings
                    .get_button_axis_setting(gamepad_button)
                    .filter(*value, button_axis.get(gamepad_button));
                button_axis.set(gamepad_button, filtered_value);

                let button_property = settings.get_button_setting(gamepad_button);
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
