#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadAxisCode {
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
pub enum GamepadButtonCode {
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

pub(crate) const ALL_AXIS_TYPES: [GamepadAxisCode; 8] = [
    GamepadAxisCode::LeftStickX,
    GamepadAxisCode::LeftStickY,
    GamepadAxisCode::LeftZ,
    GamepadAxisCode::RightStickX,
    GamepadAxisCode::RightStickY,
    GamepadAxisCode::RightZ,
    GamepadAxisCode::DPadX,
    GamepadAxisCode::DPadY,
];

pub(crate) const ALL_BUTTON_TYPES: [GamepadButtonCode; 19] = [
    GamepadButtonCode::South,
    GamepadButtonCode::East,
    GamepadButtonCode::North,
    GamepadButtonCode::West,
    GamepadButtonCode::C,
    GamepadButtonCode::Z,
    GamepadButtonCode::LeftTrigger,
    GamepadButtonCode::LeftTrigger2,
    GamepadButtonCode::RightTrigger,
    GamepadButtonCode::RightTrigger2,
    GamepadButtonCode::Select,
    GamepadButtonCode::Start,
    GamepadButtonCode::Mode,
    GamepadButtonCode::LeftThumb,
    GamepadButtonCode::RightThumb,
    GamepadButtonCode::DPadUp,
    GamepadButtonCode::DPadDown,
    GamepadButtonCode::DPadLeft,
    GamepadButtonCode::DPadRight,
];
