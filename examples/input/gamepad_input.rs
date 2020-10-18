use bevy::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadButton, GamepadEvent, GamepadEventType};
use bevy_utils::{HashMap, HashSet};

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<GamepadLobby>()
        .init_resource::<GamepadData>()
        .add_startup_system(connection_system.system())
        .add_system(connection_system.system())
        .add_system(button_system.system())
        .add_system(axis_system.system())
        .run();
}

#[derive(Default)]
struct GamepadLobby {
    gamepads: HashSet<Gamepad>,
    gamepad_event_reader: EventReader<GamepadEvent>,
}

#[derive(Default)]
struct GamepadData {
    axis: HashMap<GamepadAxis, f32>,
    button: HashMap<GamepadButton, f32>,
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

fn button_system(
    lobby: Res<GamepadLobby>,
    inputs: Res<Input<GamepadButton>>,
    button_axes: Res<Axis<GamepadButton>>,
    mut data: ResMut<GamepadData>,
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
            let gamepad_button = GamepadButton(*gamepad, *button_type);
            if inputs.just_pressed(gamepad_button) {
                println!("{:?} Pressed", gamepad_button);
            } else if inputs.just_released(GamepadButton(*gamepad, *button_type)) {
                println!("{:?} Released", gamepad_button);
            }
            if let Some(value) = button_axes.get(gamepad_button) {
                if !approx_eq(
                    data.button.get(&gamepad_button).copied().unwrap_or(0.0),
                    value,
                ) {
                    data.button.insert(gamepad_button, value);
                    println!("{:?} is {}", gamepad_button, value);
                }
            }
        }
    }
}

fn axis_system(
    lobby: Res<GamepadLobby>,
    axes: Res<Axis<GamepadAxis>>,
    mut data: ResMut<GamepadData>,
) {
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
            let gamepad_axis = GamepadAxis(*gamepad, *axis_type);
            if let Some(value) = axes.get(gamepad_axis) {
                if !approx_eq(data.axis.get(&gamepad_axis).copied().unwrap_or(0.0), value) {
                    data.axis.insert(gamepad_axis, value);
                    println!("{:?} is {}", gamepad_axis, value);
                }
            }
        }
    }
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON
}
