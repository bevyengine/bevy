//! Iterates and prints gamepad input and connection events.

use bevy::{
    input::gamepad::{GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadConnectionEvent},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(gamepad_events)
        .run();
}

fn gamepad_events(
    mut gamepad_connection_events: EventReader<GamepadConnectionEvent>,
    mut gamepad_axis_events: EventReader<GamepadAxisChangedEvent>,
    mut gamepad_button_events: EventReader<GamepadButtonChangedEvent>,
) {
    for connection_event in gamepad_connection_events.iter() {
        info!("{:?}", connection_event);
    }
    for axis_event in gamepad_axis_events.iter() {
        info!(
            "{:?} of {:?} is changed to {}",
            axis_event.axis_type, axis_event.gamepad, axis_event.value
        );
    }
    for button_event in gamepad_button_events.iter() {
        info!(
            "{:?} of {:?} is changed to {}",
            button_event.button_type, button_event.gamepad, button_event.value
        );
    }
}
