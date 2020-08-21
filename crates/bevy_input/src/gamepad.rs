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
pub struct GamepadButton(pub Gamepad, pub GamepadButtonType);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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
pub struct GamepadAxis(pub Gamepad, pub GamepadAxisType);
