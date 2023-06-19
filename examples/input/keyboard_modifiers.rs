//! Demonstrates using key modifiers (ctrl, shift).

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, keyboard_input_system)
        .run();
}

/// This system prints when `Ctrl + Shift + A` is pressed
fn keyboard_input_system(input: Res<Input<KeyCode>>) {
    let shift = input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);

    if ctrl && shift && input.just_pressed(KeyCode::A) {
        info!("Just pressed Ctrl + Shift + A!");
    }
}
