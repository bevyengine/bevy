use bevy_input::gamepad::{GamepadAxis, GamepadButton};

pub fn convert_button(button: gilrs::Button) -> Option<GamepadButton> {
    match button {
        gilrs::Button::South => Some(GamepadButton::South),
        gilrs::Button::East => Some(GamepadButton::East),
        gilrs::Button::North => Some(GamepadButton::North),
        gilrs::Button::West => Some(GamepadButton::West),
        gilrs::Button::C => Some(GamepadButton::C),
        gilrs::Button::Z => Some(GamepadButton::Z),
        gilrs::Button::LeftTrigger => Some(GamepadButton::LeftTrigger),
        gilrs::Button::LeftTrigger2 => Some(GamepadButton::LeftTrigger2),
        gilrs::Button::RightTrigger => Some(GamepadButton::RightTrigger),
        gilrs::Button::RightTrigger2 => Some(GamepadButton::RightTrigger2),
        gilrs::Button::Select => Some(GamepadButton::Select),
        gilrs::Button::Start => Some(GamepadButton::Start),
        gilrs::Button::Mode => Some(GamepadButton::Mode),
        gilrs::Button::LeftThumb => Some(GamepadButton::LeftThumb),
        gilrs::Button::RightThumb => Some(GamepadButton::RightThumb),
        gilrs::Button::DPadUp => Some(GamepadButton::DPadUp),
        gilrs::Button::DPadDown => Some(GamepadButton::DPadDown),
        gilrs::Button::DPadLeft => Some(GamepadButton::DPadLeft),
        gilrs::Button::DPadRight => Some(GamepadButton::DPadRight),
        gilrs::Button::Unknown => None,
    }
}

pub fn convert_axis(axis: gilrs::Axis) -> Option<GamepadAxis> {
    match axis {
        gilrs::Axis::LeftStickX => Some(GamepadAxis::LeftStickX),
        gilrs::Axis::LeftStickY => Some(GamepadAxis::LeftStickY),
        gilrs::Axis::LeftZ => Some(GamepadAxis::LeftZ),
        gilrs::Axis::RightStickX => Some(GamepadAxis::RightStickX),
        gilrs::Axis::RightStickY => Some(GamepadAxis::RightStickY),
        gilrs::Axis::RightZ => Some(GamepadAxis::RightZ),
        // The `axis_dpad_to_button` gilrs filter should filter out all DPadX and DPadY events. If
        // it doesn't then we probably need an entry added to the following repo and an update to
        // GilRs to use the updated database: https://github.com/gabomdq/SDL_GameControllerDB
        gilrs::Axis::Unknown | gilrs::Axis::DPadX | gilrs::Axis::DPadY => None,
    }
}
