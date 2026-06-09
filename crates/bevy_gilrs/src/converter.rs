use bevy_input::gamepad::{GamepadAxis, GamepadButton};

pub fn convert_button(button: gilrs::Button, code: gilrs::ev::Code) -> GamepadButton {
    match button {
        gilrs::Button::South => GamepadButton::South,
        gilrs::Button::East => GamepadButton::East,
        gilrs::Button::North => GamepadButton::North,
        gilrs::Button::West => GamepadButton::West,
        gilrs::Button::C => GamepadButton::C,
        gilrs::Button::Z => GamepadButton::Z,
        gilrs::Button::LeftTrigger => GamepadButton::LeftTrigger,
        gilrs::Button::LeftTrigger2 => GamepadButton::LeftTrigger2,
        gilrs::Button::RightTrigger => GamepadButton::RightTrigger,
        gilrs::Button::RightTrigger2 => GamepadButton::RightTrigger2,
        gilrs::Button::Select => GamepadButton::Select,
        gilrs::Button::Start => GamepadButton::Start,
        gilrs::Button::Mode => GamepadButton::Mode,
        gilrs::Button::LeftThumb => GamepadButton::LeftThumb,
        gilrs::Button::RightThumb => GamepadButton::RightThumb,
        gilrs::Button::DPadUp => GamepadButton::DPadUp,
        gilrs::Button::DPadDown => GamepadButton::DPadDown,
        gilrs::Button::DPadLeft => GamepadButton::DPadLeft,
        gilrs::Button::DPadRight => GamepadButton::DPadRight,
        gilrs::Button::Unknown => GamepadButton::Other(code.into_u32()),
    }
}

pub fn convert_axis(axis: gilrs::Axis, code: gilrs::ev::Code) -> Option<GamepadAxis> {
    match axis {
        gilrs::Axis::LeftStickX => Some(GamepadAxis::LeftStickX),
        gilrs::Axis::LeftStickY => Some(GamepadAxis::LeftStickY),
        gilrs::Axis::LeftZ => Some(GamepadAxis::LeftZ),
        gilrs::Axis::RightStickX => Some(GamepadAxis::RightStickX),
        gilrs::Axis::RightStickY => Some(GamepadAxis::RightStickY),
        gilrs::Axis::RightZ => Some(GamepadAxis::RightZ),
        gilrs::Axis::Unknown => Some(GamepadAxis::Other(code.into_u32())),
        // The `axis_dpad_to_button` gilrs filter should filter out all DPadX and DPadY events. If
        // it doesn't then we probably need an entry added to the following repo and an update to
        // GilRs to use the updated database: https://github.com/gabomdq/SDL_GameControllerDB
        gilrs::Axis::DPadX | gilrs::Axis::DPadY => None,
    }
}
