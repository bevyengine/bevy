use crate::{device_codes::*, devices::Gamepad};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadEventType {
    Connected,
    Disconnected,
    ButtonChanged(GamepadButtonCode, f32),
    AxisChanged(GamepadAxisCode, f32),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEvent(pub Gamepad, pub GamepadEventType);

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct GamepadEventRaw(pub Gamepad, pub GamepadEventType);
