//! Demonstrates how to grab and hide the mouse cursor.

use bevy::{prelude::*, window::CursorGrabMode};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, grab_mouse)
        .run();
}

// This system grabs the mouse when the left mouse button is pressed
// and releases it when the escape key is pressed
fn grab_mouse(
    mut windows: Query<&mut Window>,
    mouse: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
) {
    let mut window = windows.single_mut();

    if mouse.just_pressed(MouseButton::Left) {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }

    if key.just_pressed(KeyCode::Escape) {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
    }
}
