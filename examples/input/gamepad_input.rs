//! Shows handling of gamepad input, connections, and disconnections.

use bevy::prelude::*;
use bevy_internal::input::gamepad::{GamepadAnalogButtonsComponent, GamepadAxisComponent, GamepadDigitalButtonsComponent};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, gamepad_system)
        .run();
}

fn gamepad_system(
    gamepads: Query<(&Gamepad, &GamepadDigitalButtonsComponent, &GamepadAnalogButtonsComponent, &GamepadAxisComponent)>,
) {
    for (gamepad, digital, analog, axis) in gamepads.iter() {
        if digital.just_pressed(GamepadButtonType::South) {
            info!("{:?} just pressed South", gamepad);
        } else if digital.just_released(GamepadButtonType::South)
        {
            info!("{:?} just released South", gamepad);
        }

        let right_trigger = analog
            .get(GamepadButtonType::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            info!("{:?} RightTrigger2 value is {}", gamepad, right_trigger);
        }

        let left_stick_x = axis
            .get(GamepadAxisType::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{:?} LeftStickX value is {}", gamepad, left_stick_x);
        }
    }
}
