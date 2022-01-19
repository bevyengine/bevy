use strum_macros::EnumIter;

/// A type of a [`GamepadButton`](crate::gamepad::GamepadButton).
///
/// ## Usage
///
/// This is used to determine which button has changed its value when receiving a
/// [`GamepadEventType::ButtonChanged`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadButtonType {
    /// The bottom action button of the action pad (i.e. PS: Cross, Xbox: A).
    South,
    /// The right action button of the action pad (i.e. PS: Circle, Xbox: B).
    East,
    /// The upper action button of the action pad (i.e. PS: Triangle, Xbox: Y).
    North,
    /// The left action button of the action pad (i.e. PS: Square, Xbox: X).
    West,

    /// The C button.
    C,
    /// The Z button.
    Z,

    /// The first left trigger.
    LeftTrigger,
    /// The second left trigger.
    LeftTrigger2,
    /// The first right trigger.
    RightTrigger,
    /// The second right trigger.
    RightTrigger2,

    /// The select button.
    Select,
    /// The start button.
    Start,
    /// The mode button.
    Mode,

    /// The left thumb stick button.
    LeftThumb,
    /// The right thumb stick button.
    RightThumb,

    /// The up button of the D-Pad.
    DPadUp,
    /// The down button of the D-Pad.
    DPadDown,
    /// The left button of the D-Pad.
    DPadLeft,
    /// The right button of the D-Pad.
    DPadRight,
}

/// An type of a [`GamepadAxis`](crate::gamepad::GamepadAxis).
///
/// ## Usage
///
/// This is used to determine which axis has changed its value when receiving a
/// [`GamepadEventType::ButtonChanged`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadAxisType {
    /// The horizontal value of the left stick.
    LeftStickX,
    /// The vertical value of the left stick.
    LeftStickY,
    /// The value of the left `Z` button.
    LeftZ,

    /// The horizontal value of the right stick.
    RightStickX,
    /// The vertical value of the right stick.
    RightStickY,
    /// The value of the right `Z` button.
    RightZ,

    /// The horizontal value of the D-Pad.
    DPadX,
    /// The vertical value of the D-Pad.
    DPadY,
}

/// A type of a [`GamepadEvent`](crate::gamepad::GamepadEvent).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GamepadEventType {
    /// A [`Gamepad`](crate::gamepad::Gamepad) has been connected.
    Connected,
    /// A [`Gamepad`](crate::gamepad::Gamepad) has been disconnected.
    Disconnected,

    /// The value of a [`Gamepad`](crate::gamepad::Gamepad) button has changed.
    ButtonChanged(GamepadButtonType, f32),
    /// The value of a [`Gamepad`](crate::gamepad::Gamepad) axis has changed.
    AxisChanged(GamepadAxisType, f32),
}
