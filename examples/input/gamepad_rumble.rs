//! Shows how to trigger force-feedback, making gamepads rumble when buttons are
//! pressed.

use bevy::{
    input::gamepad::{RumbleIntensity, RumbleRequest},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, gamepad_system)
        .run();
}

fn gamepad_system(
    gamepads: Res<Gamepads>,
    button_inputs: Res<Input<GamepadButton>>,
    mut rumble_requests: EventWriter<RumbleRequest>,
) {
    for gamepad in gamepads.iter() {
        let button_pressed = |button| {
            button_inputs.just_pressed(GamepadButton {
                gamepad,
                button_type: button,
            })
        };
        if button_pressed(GamepadButtonType::South) {
            info!("(S) South face button: weak rumble for 3 second");
            // Use the simplified API provided by bevy
            rumble_requests.send(RumbleRequest {
                gamepad,
                duration_seconds: 3.0,
                intensity: RumbleIntensity::Weak,
            });
        } else if button_pressed(GamepadButtonType::West) {
            info!("(W) West face button: strong rumble for 10 second");
            rumble_requests.send(RumbleRequest {
                gamepad,
                intensity: RumbleIntensity::Strong,
                duration_seconds: 10.0,
            });
        } else if button_pressed(GamepadButtonType::North) {
            info!("(N) North face button: Interrupt the current rumble");
            rumble_requests.send(RumbleRequest::stop(gamepad));
        }
    }
}
