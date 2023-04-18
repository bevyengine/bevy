//! Shows how to trigger force-feedback, making gamepads rumble when buttons are
//! pressed.

use bevy::{
    input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest},
    prelude::*,
    utils::Duration,
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
            info!("(S) South face button: weak rumble for 0.5 seconds");
            rumble_requests.send(GamepadRumbleRequest::Add {
                gamepad,
                duration: Duration::from_secs_f32(0.5),
                intensity: GamepadRumbleIntensity {
                    strong: 0.0,
                    weak: 0.25,
                },
            });
        } else if button_pressed(GamepadButtonType::West) {
            info!("(W) West face button: maximum rumble for 5 second");
            rumble_requests.send(GamepadRumbleRequest::Add {
                gamepad,
                intensity: GamepadRumbleIntensity {
                    strong: 1.0,
                    weak: 1.0,
                },
                duration: Duration::from_secs(5),
            });
        } else if button_pressed(GamepadButtonType::North) {
            info!(
                "(N) North face button: Low-intensity, strong (low-frequency)
                rumble for 5 seconds. Press multiple times for increased
                intensity."
            );
            rumble_requests.send(GamepadRumbleRequest::Add {
                gamepad,
                intensity: GamepadRumbleIntensity {
                    strong: 0.1,
                    weak: 0.0,
                },
                duration: Duration::from_secs(5),
            });
        } else if button_pressed(GamepadButtonType::East) {
            info!("(E) East face button: Interrupt the current rumble");
            rumble_requests.send(GamepadRumbleRequest::Stop { gamepad });
        }
    }
}
