//! Demonstrates how to grab and hide the mouse cursor.

use bevy::{prelude::*, window::PrimaryWindow};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(grab_mouse)
        .run();
}

// This system grabs the mouse when the left mouse button is pressed
// and releases it when the escape key is pressed
fn grab_mouse(
    mut commands: Commands,
    primary_window: Res<PrimaryWindow>,
    mouse: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
) {
    let mut window_commands =
        commands.window(primary_window.window.expect("Primary window should exist"));
    if mouse.just_pressed(MouseButton::Left) {
        window_commands.set_cursor_visibility(false);
        window_commands.set_cursor_lock_mode(true);
    }
    if key.just_pressed(KeyCode::Escape) {
        window_commands.set_cursor_visibility(true);
        window_commands.set_cursor_lock_mode(false);
    }
}
