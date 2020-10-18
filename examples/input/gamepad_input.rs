use bevy::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadButton, GamepadEvent, GamepadEventType};
use std::collections::HashSet;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(connection_system.system())
        .add_system(connection_system.system())
        .add_system(button_system.system())
        .add_system(axis_system.system())
        .init_resource::<GamepadLobby>()
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
                println!("Connected {:?}", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                lobby.gamepads.remove(gamepad);
                println!("Disconnected {:?}", gamepad);
            }
        }
    }
}

fn button_system(
    lobby: Res<GamepadLobby>,
    inputs: Res<Input<GamepadButton>>,
    button_axes: Res<Axis<GamepadButton>>,
) {
    let button_types = [
        GamepadButtonType::South,
        GamepadButtonType::East,
        GamepadButtonType::North,
        GamepadButtonType::West,
        GamepadButtonType::C,
        GamepadButtonType::Z,
        GamepadButtonType::LeftTrigger,
        GamepadButtonType::LeftTrigger2,
        GamepadButtonType::RightTrigger,
        GamepadButtonType::RightTrigger2,
        GamepadButtonType::Select,
        GamepadButtonType::Start,
        GamepadButtonType::Mode,
        GamepadButtonType::LeftThumb,
        GamepadButtonType::RightThumb,
        GamepadButtonType::DPadUp,
        GamepadButtonType::DPadDown,
        GamepadButtonType::DPadLeft,
        GamepadButtonType::DPadRight,
    ];
    for gamepad in lobby.gamepads.iter() {
        for button_type in button_types.iter() {
            if inputs.just_pressed(GamepadButton(*gamepad, *button_type)) {
                println!("Pressed {:?}", GamepadButton(*gamepad, *button_type));
            } else if inputs.just_released(GamepadButton(*gamepad, *button_type)) {
                println!("Released {:?}", GamepadButton(*gamepad, *button_type));
            }
            if let Some(value) = button_axes.get(&GamepadButton(*gamepad, *button_type)) {
                if value_check(value) {
                    println!(
                        "Button as Axis {:?} is {}",
                        GamepadButton(*gamepad, *button_type),
                        value
                    );
                }
            }
        }
    }
}

fn axis_system(lobby: Res<GamepadLobby>, axes: Res<Axis<GamepadAxis>>) {
    let axis_types = [
        GamepadAxisType::LeftStickX,
        GamepadAxisType::LeftStickY,
        GamepadAxisType::LeftZ,
        GamepadAxisType::RightStickX,
        GamepadAxisType::RightStickY,
        GamepadAxisType::RightZ,
        GamepadAxisType::DPadX,
        GamepadAxisType::DPadY,
    ];
    for gamepad in lobby.gamepads.iter() {
        for axis_type in axis_types.iter() {
            if let Some(value) = axes.get(&GamepadAxis(*gamepad, *axis_type)) {
                if value_check(value) {
                    println!("Axis {:?} is {}", GamepadAxis(*gamepad, *axis_type), value);
                }
            }
        }
    }
}

fn value_check(value: f32) -> bool {
    let value = value.abs();
    value > 0.1f32 && value < 0.9f32
}
