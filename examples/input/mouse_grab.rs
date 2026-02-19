//! Demonstrates how to grab and hide the mouse cursor.

use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, grab_mouse)
        .run();
}

// This system grabs the mouse when the left mouse button is pressed
// and releases it when the escape key is pressed
fn grab_mouse(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }

    if key.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}
