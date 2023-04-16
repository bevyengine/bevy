//! Shows how to trigger force-feedback, making gamepads rumble when buttons are
//! pressed.

use bevy::{
    input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest},
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
    mut rumble_requests: EventWriter<GamepadRumbleRequest>,
) {
    for gamepad in gamepads.iter() {
        let button_pressed = |button| {
            button_inputs.just_pressed(GamepadButton {
                gamepad,
                button_type: button,
            })
        };
        if button_pressed(GamepadButtonType::South) {
            info!("(S) South face button: weak rumble for 1 second");
            // Use the simplified API provided by Bevy
            rumble_requests.send(GamepadRumbleRequest {
                gamepad,
                duration_seconds: 1.0,
                intensity: GamepadRumbleIntensity {
                    strong: 0.0,
                    weak: 0.25,
                },
                additive: true,
            });
        } else if button_pressed(GamepadButtonType::West) {
            info!("(W) West face button: maximum rumble for 5 second");
            rumble_requests.send(GamepadRumbleRequest {
                gamepad,
                intensity: GamepadRumbleIntensity {
                    strong: 1.0,
                    weak: 1.0,
                },
                duration_seconds: 5.0,
                additive: true,
            });
        } else if button_pressed(GamepadButtonType::North) {
            info!("(N) North face button: Interrupt the current rumble");
            rumble_requests.send(GamepadRumbleRequest::stop(gamepad));
        }
    }
}
