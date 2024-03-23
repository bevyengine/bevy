//! Shows handling of gamepad input, connections, and disconnections.

use bevy::prelude::*;
use bevy_internal::input::gamepad::GamepadsSystemParam;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, gamepad_system)
        .add_systems(Update, iter_gamepad_system)
        .add_systems(Update, convenient_gamepad_system)
        .run();
}

// Same as gamepad_input example
fn gamepad_system(gamepads: GamepadsSystemParam) {
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButtonType::South) {
            info!("{:?} just pressed South", gamepad.id);
        } else if gamepad.just_released(GamepadButtonType::South) {
            info!("{:?} just released South", gamepad.id);
        }

        let right_trigger = gamepad.get_analog_button(GamepadButtonType::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            info!("{:?} RightTrigger2 value is {}", gamepad.id, right_trigger);
        }

        let left_stick_x = gamepad.get_axis(GamepadAxisType::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{:?} LeftStickX value is {}", gamepad.id, left_stick_x);
        }
    }
}

// New
fn iter_gamepad_system(gamepads: GamepadsSystemParam) {
    if gamepads.iter().any(|gamepad| gamepad.pressed(GamepadButtonType::South)) {
        info!("Someone pressed South!")
    }
    if gamepads.iter().all(|gamepad| gamepad.pressed(GamepadButtonType::North)) {
        info!("Everyone pressed North!")
    }
}

fn convenient_gamepad_system(gamepads: GamepadsSystemParam) {
    for gamepad in gamepads.iter() {
        // Sticks as vec2
        info!("Right stick: {:}, Left stick: {:}", gamepad.right_stick(), gamepad.left_stick());
        // All button info in the same struct
        let button = gamepad.get_button(GamepadButtonType::LeftTrigger2).unwrap();
        info!("Button {:?}, just_pressed: {:}, just_released: {:}, analog_value: {:}", button.button_type, button.just_pressed, button.just_released, button.value)
    }
}
