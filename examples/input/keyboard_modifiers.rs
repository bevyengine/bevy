//! Demonstrates using key modifiers (ctrl, shift).

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, keyboard_input_system)
        .run();
}

/// This system prints when `Ctrl + Shift + A` is pressed
fn keyboard_input_system(input: Res<ButtonInput<PhysicalKey>>) {
    let shift = input.any_pressed([PhysicalKey::ShiftLeft, PhysicalKey::ShiftRight]);
    let ctrl = input.any_pressed([PhysicalKey::ControlLeft, PhysicalKey::ControlRight]);

    if ctrl && shift && input.just_pressed(PhysicalKey::KeyA) {
        info!("Just pressed Ctrl + Shift + A!");
    }
}
