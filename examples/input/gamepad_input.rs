//! Shows handling of gamepad input, connections, and disconnections.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, gamepad_system)
        .run();
}

fn gamepad_system(gamepads: Query<(Entity, &Gamepad)>) {
    for (entity, gamepad) in &gamepads {
        if gamepad.just_pressed(GamepadButton::South) {
            info!("{} just pressed South", entity);
        } else if gamepad.just_released(GamepadButton::South) {
            info!("{} just released South", entity);
        }

        let right_trigger = gamepad.get(GamepadButton::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            info!("{} RightTrigger2 value is {}", entity, right_trigger);
        }

        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{} LeftStickX value is {}", entity, left_stick_x);
        }
    }
}
