//! Demonstrates handling a key press/release.

use bevy::{input::keyboard::Key, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, keyboard_input_system)
        .run();
}

/// This system responds to certain key presses
fn keyboard_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    key_input: Res<ButtonInput<Key>>,
) {
    // KeyCode is used when you want the key location across different keyboard layouts
    // See https://w3c.github.io/uievents-code/#code-value-tables for the locations
    if keyboard_input.pressed(KeyCode::KeyA) {
        info!("'A' currently pressed");
    }

    if keyboard_input.just_pressed(KeyCode::KeyA) {
        info!("'A' just pressed");
    }
    if keyboard_input.just_released(KeyCode::KeyA) {
        info!("'A' just released");
    }

    // Key is used when you want a specific key, no matter where it is located.
    // This is useful for symbols that have a specific connotation, e.g. '?' for
    // a help menu or '+'/'-' for zoom
    let key = Key::Character("?".into());
    if key_input.pressed(key.clone()) {
        info!("'?' currently pressed");
    }
    if key_input.just_pressed(key.clone()) {
        info!("'?' just pressed");
    }
    if key_input.just_released(key) {
        info!("'?' just released");
    }
}
