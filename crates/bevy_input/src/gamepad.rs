use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Gamepad(pub usize);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GamepadEventType {
    Connected,
    Disconnected,
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

impl Default for GamepadButtonType {
    fn default() -> Self {
        GamepadButtonType::South
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GamepadButton(pub Gamepad, pub GamepadButtonType);

#[derive(Serialize, Deserialize)]
struct GamepadButtonHelper {
    Pad: usize,
    Button: GamepadButtonType,
}

impl Serialize for GamepadButton {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        GamepadButtonHelper { Pad: self.0.0, Button: self.1 }.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GamepadButton {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|GamepadButtonHelper { Pad, Button }
            | GamepadButton(Gamepad(Pad), Button))
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
    DPadX,
    DPadY,
}

impl Default for GamepadAxisType {
    fn default() -> Self {
        GamepadAxisType::LeftStickX
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GamepadAxis(pub Gamepad, pub GamepadAxisType);

#[derive(Serialize, Deserialize)]
struct GamepadAxisHelper {
    Pad: usize,
    Axis: GamepadAxisType,
}

impl Serialize for GamepadAxis {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        GamepadAxisHelper { Pad: self.0.0, Axis: self.1 }.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GamepadAxis {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|GamepadAxisHelper { Pad, Axis }
            | GamepadAxis(Gamepad(Pad), Axis))
    }
}