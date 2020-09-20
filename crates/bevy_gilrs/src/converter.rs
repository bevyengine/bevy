use bevy_input::gamepad::{Gamepad, GamepadAxisType, GamepadButtonType};

pub fn convert_gamepad_id(gamepad_id: gilrs::GamepadId) -> Gamepad {
    Gamepad(gamepad_id.into())
}

pub fn convert_button(button: gilrs::Button) -> Option<GamepadButtonType> {
    match button {
        gilrs::Button::South => Some(GamepadButtonType::South),
        gilrs::Button::East => Some(GamepadButtonType::East),
        gilrs::Button::North => Some(GamepadButtonType::North),
        gilrs::Button::West => Some(GamepadButtonType::West),
        gilrs::Button::C => Some(GamepadButtonType::C),
        gilrs::Button::Z => Some(GamepadButtonType::Z),
        gilrs::Button::LeftTrigger => Some(GamepadButtonType::LeftTrigger),
        gilrs::Button::LeftTrigger2 => Some(GamepadButtonType::LeftTrigger2),
        gilrs::Button::RightTrigger => Some(GamepadButtonType::RightTrigger),
        gilrs::Button::RightTrigger2 => Some(GamepadButtonType::RightTrigger2),
        gilrs::Button::Select => Some(GamepadButtonType::Select),
        gilrs::Button::Start => Some(GamepadButtonType::Start),
        gilrs::Button::Mode => Some(GamepadButtonType::Mode),
        gilrs::Button::LeftThumb => Some(GamepadButtonType::LeftThumb),
        gilrs::Button::RightThumb => Some(GamepadButtonType::RightThumb),
        gilrs::Button::DPadUp => Some(GamepadButtonType::DPadUp),
        gilrs::Button::DPadDown => Some(GamepadButtonType::DPadDown),
        gilrs::Button::DPadLeft => Some(GamepadButtonType::DPadLeft),
        gilrs::Button::DPadRight => Some(GamepadButtonType::DPadRight),
        gilrs::Button::Unknown => None,
    }
}

pub fn convert_axis(axis: gilrs::Axis) -> Option<GamepadAxisType> {
    match axis {
        gilrs::Axis::LeftStickX => Some(GamepadAxisType::LeftStickX),
        gilrs::Axis::LeftStickY => Some(GamepadAxisType::LeftStickY),
        gilrs::Axis::LeftZ => Some(GamepadAxisType::LeftZ),
        gilrs::Axis::RightStickX => Some(GamepadAxisType::RightStickX),
        gilrs::Axis::RightStickY => Some(GamepadAxisType::RightStickY),
        gilrs::Axis::RightZ => Some(GamepadAxisType::RightZ),
        gilrs::Axis::DPadX => Some(GamepadAxisType::DPadX),
        gilrs::Axis::DPadY => Some(GamepadAxisType::DPadY),
        gilrs::Axis::Unknown => None,
    }
}
