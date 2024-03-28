//! Shows handling of gamepad input, connections, and disconnections.

use bevy::prelude::*;
use bevy_internal::input::gamepad::{Gamepad, GamepadAxisComponent, GamepadButtons};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, gamepad_system)
        .run();
}

fn gamepad_system(
    gamepads: Query<(&Gamepad, &GamepadButtons, &GamepadAxisComponent)>,
) {
    for (gamepad, buttons, axis) in gamepads.iter() {
        if buttons.just_pressed(GamepadButtonType::South) {
            info!("{:?} just pressed South", gamepad.id());
        } else if buttons.just_released(GamepadButtonType::South)
        {
            info!("{:?} just released South", gamepad.id());
        }

        let right_trigger = buttons
            .get(GamepadButtonType::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            info!("{:?} RightTrigger2 value is {}", gamepad.id(), right_trigger);
        }

        let left_stick_x = axis
            .get(GamepadAxisType::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{:?} LeftStickX value is {}", gamepad.id(), left_stick_x);
        }
    }
}
