//! Prints out all chars as they are inputted.

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, print_char_event_system)
        .run();
}

/// This system prints out all char events as they come in
fn print_char_event_system(mut char_input_events: EventReader<KeyboardInput>) {
    for event in char_input_events.read() {
        // Only check for characters when the key is pressed
        if event.state == ButtonState::Released {
            continue;
        }
        if let Key::Character(character) = &event.logical_key {
            info!("{:?}: '{}'", event, character);
        }
    }
}
