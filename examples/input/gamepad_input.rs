use bevy::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadButton, GamepadEvent, GamepadEventType};
use bevy_utils::HashSet;

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<GamepadLobby>()
        .add_startup_system(connection_system.system())
        .add_system(connection_system.system())
        .add_system(gamepad_system.system())
        .run();
}

#[derive(Default)]
struct GamepadLobby {
    gamepads: HashSet<Gamepad>,
    gamepad_event_reader: EventReader<GamepadEvent>,
}

fn connection_system(mut lobby: ResMut<GamepadLobby>, gamepad_event: Res<Events<GamepadEvent>>) {
    for event in lobby.gamepad_event_reader.iter(&gamepad_event) {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                lobby.gamepads.insert(*gamepad);
                println!("{:?} Connected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                lobby.gamepads.remove(gamepad);
                println!("{:?} Disconnected", gamepad);
            }
            _ => (),
        }
    }
}

fn gamepad_system(
    lobby: Res<GamepadLobby>,
    button_inputs: Res<Input<GamepadButton>>,
    button_axes: Res<Axis<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
) {
    for gamepad in lobby.gamepads.iter() {
        let south_button = GamepadButton(*gamepad, GamepadButtonType::South);
        if button_inputs.just_pressed(south_button) {
            println!(
                "{:?} of {:?} is just pressed",
                GamepadButtonType::South,
                gamepad
            );
        } else if button_inputs.just_released(south_button) {
            println!(
                "{:?} of {:?} is just released",
                GamepadButtonType::South,
                gamepad
            );
        }

        println!(
            "For {:?}: {:?} is {:.4}, {:?} is {:.4}",
            gamepad,
            GamepadButtonType::RightTrigger2,
            button_axes
                .get(GamepadButton(*gamepad, GamepadButtonType::RightTrigger2))
                .unwrap_or(0.0),
            GamepadAxisType::LeftStickX,
            axes.get(GamepadAxis(*gamepad, GamepadAxisType::LeftStickX))
                .unwrap_or(0.0)
        )
    }
}
