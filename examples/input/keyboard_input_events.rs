//! Prints out all keyboard events.

use bevy::{input::keyboard::KeyboardInput, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, print_keyboard_event_system)
        .run();
}

/// This system prints out all keyboard inputs as they come in
fn print_keyboard_event_system(mut keyboard_inputs: MessageReader<KeyboardInput>) {
    for keyboard_input in keyboard_inputs.read() {
        info!("{:?}", keyboard_input);
    }
}
