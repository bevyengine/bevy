use bevy::{
    input::gamepad::{Gamepad, GamepadButton, GamepadEvent, GamepadEventType},
    prelude::*,
    utils::HashSet,
};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .init_resource::<GamepadLobby>()
        .add_system_to_stage(stage::PRE_UPDATE, connection_system.system())
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
    for gamepad in lobby.gamepads.iter().cloned() {
        if button_inputs.just_pressed(GamepadButton(gamepad, GamepadButtonType::South)) {
            println!("{:?} just pressed South", gamepad);
        } else if button_inputs.just_released(GamepadButton(gamepad, GamepadButtonType::South)) {
            println!("{:?} just released South", gamepad);
        }

        let right_trigger = button_axes
            .get(GamepadButton(gamepad, GamepadButtonType::RightTrigger2))
            .unwrap();
        if right_trigger.abs() > 0.01 {
            println!("{:?} RightTrigger2 value is {}", gamepad, right_trigger);
        }

        let left_stick_x = axes
            .get(GamepadAxis(gamepad, GamepadAxisType::LeftStickX))
            .unwrap();
        if left_stick_x.abs() > 0.01 {
            println!("{:?} LeftStickX value is {}", gamepad, left_stick_x);
        }
    }
}
