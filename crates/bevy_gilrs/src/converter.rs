use bevy_input::devices::gamepad::*;

pub fn convert_gamepad_id(gamepad_id: gilrs::GamepadId) -> Gamepad {
    Gamepad(gamepad_id.into())
}

pub fn convert_button(button: gilrs::Button) -> Option<GamepadButtonCode> {
    match button {
        gilrs::Button::South => Some(GamepadButtonCode::South),
        gilrs::Button::East => Some(GamepadButtonCode::East),
        gilrs::Button::North => Some(GamepadButtonCode::North),
        gilrs::Button::West => Some(GamepadButtonCode::West),
        gilrs::Button::C => Some(GamepadButtonCode::C),
        gilrs::Button::Z => Some(GamepadButtonCode::Z),
        gilrs::Button::LeftTrigger => Some(GamepadButtonCode::LeftTrigger),
        gilrs::Button::LeftTrigger2 => Some(GamepadButtonCode::LeftTrigger2),
        gilrs::Button::RightTrigger => Some(GamepadButtonCode::RightTrigger),
        gilrs::Button::RightTrigger2 => Some(GamepadButtonCode::RightTrigger2),
        gilrs::Button::Select => Some(GamepadButtonCode::Select),
        gilrs::Button::Start => Some(GamepadButtonCode::Start),
        gilrs::Button::Mode => Some(GamepadButtonCode::Mode),
        gilrs::Button::LeftThumb => Some(GamepadButtonCode::LeftThumb),
        gilrs::Button::RightThumb => Some(GamepadButtonCode::RightThumb),
        gilrs::Button::DPadUp => Some(GamepadButtonCode::DPadUp),
        gilrs::Button::DPadDown => Some(GamepadButtonCode::DPadDown),
        gilrs::Button::DPadLeft => Some(GamepadButtonCode::DPadLeft),
        gilrs::Button::DPadRight => Some(GamepadButtonCode::DPadRight),
        gilrs::Button::Unknown => None,
    }
}

pub fn convert_axis(axis: gilrs::Axis) -> Option<GamepadAxisCode> {
    match axis {
        gilrs::Axis::LeftStickX => Some(GamepadAxisCode::LeftStickX),
        gilrs::Axis::LeftStickY => Some(GamepadAxisCode::LeftStickY),
        gilrs::Axis::LeftZ => Some(GamepadAxisCode::LeftZ),
        gilrs::Axis::RightStickX => Some(GamepadAxisCode::RightStickX),
        gilrs::Axis::RightStickY => Some(GamepadAxisCode::RightStickY),
        gilrs::Axis::RightZ => Some(GamepadAxisCode::RightZ),
        gilrs::Axis::DPadX => Some(GamepadAxisCode::DPadX),
        gilrs::Axis::DPadY => Some(GamepadAxisCode::DPadY),
        gilrs::Axis::Unknown => None,
    }
}
