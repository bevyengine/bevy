use bevy::{input::gamepad::GamepadButton, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(gamepad_system)
        .run();
}

fn gamepad_system(gamepads: Res<Gamepads>) {
    for (gamepad, button_input) in gamepads.buttons.iter() {
        if button_input.just_pressed(GamepadButton::South) {
            info!("{:?} just pressed South", gamepad);
        } else if button_input.just_released(GamepadButton::South) {
            info!("{:?} just released South", gamepad);
        }

        let right_trigger2 = button_input.value(GamepadButton::RightTrigger2);
        if right_trigger2 > 0.01 {
            info!("{:?} RightTrigger2 value is {}", gamepad, right_trigger2);
        }
    }

    for (gamepad, axes) in gamepads.axes.iter() {
        let left_stick_x = axes.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("{:?} LeftStickX value is {}", gamepad, left_stick_x);
        }
    }
}
