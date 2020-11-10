use bevy::{
    input::gamepad::{GamepadEvent, GamepadEventType},
    prelude::*,
};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(gamepad_events.system())
        .run();
}

fn gamepad_events(
    mut event_reader: Local<EventReader<GamepadEvent>>,
    gamepad_event: Res<Events<GamepadEvent>>,
) {
    for event in event_reader.iter(&gamepad_event) {
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
