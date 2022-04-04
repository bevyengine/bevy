use bevy::{
    input::gamepad::{GamepadEvent, GamepadEventType},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(gamepad_events)
        .run();
}

fn gamepad_events(mut gamepad_event: EventReader<GamepadEvent>) {
    for event in gamepad_event.iter() {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                info!("{:?} Connected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                info!("{:?} Disconnected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::ButtonChanged(button_type, value)) => {
                info!("{:?} of {:?} is changed to {}", button_type, gamepad, value);
            }
            GamepadEvent(gamepad, GamepadEventType::AxisChanged(axis_type, value)) => {
                info!("{:?} of {:?} is changed to {}", axis_type, gamepad, value);
            }
        }
    }
}
