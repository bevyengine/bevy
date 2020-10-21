use bevy::prelude::*;
use bevy_input::gamepad::{GamepadEvent, GamepadEventType};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(gamepad_raw_events.system())
        .add_system(gamepad_raw_events.system())
        .run();
}

#[derive(Default)]
struct State {
    gamepad_event_reader: EventReader<GamepadEvent>,
}

fn gamepad_raw_events(mut state: Local<State>, gamepad_event: Res<Events<GamepadEvent>>) {
    for event in state.gamepad_event_reader.iter(&gamepad_event) {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                println!("{:?} Connected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                println!("{:?} Disconnected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::ButtonChanged(button_type, value)) => {
                println!("{:?} of {:?} is changed to {}", button_type, gamepad, value);
            }
            GamepadEvent(gamepad, GamepadEventType::AxisChanged(axis_type, value)) => {
                println!("{:?} of {:?} is changed to {}", axis_type, gamepad, value);
            }
        }
    }
}
