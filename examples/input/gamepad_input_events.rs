//! Iterates and prints gamepad input and connection events.

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
        match event.event_type {
            GamepadEventType::Connected(_) => {
                info!("{:?} Connected", event.gamepad);
            }
            GamepadEventType::Disconnected => {
                info!("{:?} Disconnected", event.gamepad);
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                info!(
                    "{:?} of {:?} is changed to {}",
                    button_type, event.gamepad, value
                );
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                info!(
                    "{:?} of {:?} is changed to {}",
                    axis_type, event.gamepad, value
                );
            }
        }
    }
}
