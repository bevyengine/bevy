//! Shows handling of gamepad input, connections, and disconnections.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, gamepad_system)
        .run();
}

fn gamepad_system(gamepads: Query<&Gamepad>) {
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::South) {
            info!("{:?} just pressed South", gamepad.id());
        } else if gamepad.just_released(GamepadButton::South) {
            info!("{:?} just released South", gamepad.id());
        }

        let right_trigger = gamepad.get(GamepadButton::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            info!(
                "{:?} RightTrigger2 value is {}",
                gamepad.id(),
                right_trigger
            );
        }

        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{:?} LeftStickX value is {}", gamepad.id(), left_stick_x);
        }
    }
}
