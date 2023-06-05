//! Iterates and prints gamepad input and connection events.

use bevy::{
    input::gamepad::{
        GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadConnectionEvent, GamepadEvent,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, (gamepad_events, gamepad_ordered_events))
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

// If you require in-frame relative event ordering, you can also read the `Gamepad` event
// stream directly. For standard use-cases, reading the events individually or using the
// `Input<T>` or `Axis<T>` resources is preferable.
fn gamepad_ordered_events(mut gamepad_events: EventReader<GamepadEvent>) {
    for gamepad_event in gamepad_events.iter() {
        match gamepad_event {
            GamepadEvent::Connection(connection_event) => info!("{:?}", connection_event),
            GamepadEvent::Button(button_event) => info!("{:?}", button_event),
            GamepadEvent::Axis(axis_event) => info!("{:?}", axis_event),
        }
    }
}
