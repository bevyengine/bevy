use bevy::{
    gilrs::{ff, RumbleIntensity, RumbleRequest},
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
            rumble_requests.send(RumbleRequest::with_intensity(
                RumbleIntensity::Weak,
                3.0,
                gamepad,
            ));
        } else if button_pressed(GamepadButtonType::West) {
            info!("(W) West face button: strong rumble for 10 second");
            rumble_requests.send(RumbleRequest::with_intensity(
                RumbleIntensity::Strong,
                10.0,
                gamepad,
            ));
        } else if button_pressed(GamepadButtonType::East) {
            info!("(E) East face button: alternating for 5 seconds");
            // Use the gilrs::ff more complex but feature-complete effect
            let duration = ff::Ticks::from_ms(800);
            let mut effect = ff::EffectBuilder::new();
            effect
                .add_effect(ff::BaseEffect {
                    kind: ff::BaseEffectType::Strong { magnitude: 60_000 },
                    scheduling: ff::Replay {
                        play_for: duration,
                        with_delay: duration * 3,
                        ..Default::default()
                    },
                    envelope: Default::default(),
                })
                .add_effect(ff::BaseEffect {
                    kind: ff::BaseEffectType::Weak { magnitude: 60_000 },
                    scheduling: ff::Replay {
                        after: duration * 2,
                        play_for: duration,
                        with_delay: duration * 3,
                    },
                    ..Default::default()
                });
            let request = RumbleRequest {
                pad: gamepad,
                gilrs_effect: effect,
                duration_seconds: 5.0,
            };
            rumble_requests.send(request);
        } else if button_pressed(GamepadButtonType::North) {
            info!("(N) North face button: Interupt the current rumble");
            rumble_requests.send(RumbleRequest::stop(gamepad));
        }
    }
}
