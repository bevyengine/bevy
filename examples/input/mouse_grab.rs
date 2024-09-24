//! Demonstrates how to grab and hide the mouse cursor.

use bevy::{prelude::*, render::view::cursor::CursorIcon, window::CursorGrabMode};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, grab_mouse)
        .run();
}

// This system grabs the mouse when the left mouse button is pressed
// and releases it when the escape key is pressed
fn grab_mouse(
    mut windows_and_cursors: Query<(&mut Window, &mut CursorIcon)>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    // There are cases where we can have multiple windows and cursors,
    // but not in this example. Keeping the naming convention since it
    // exists in other examples
    let Ok((mut window, mut cursor)) = windows_and_cursors.get_single_mut() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        *cursor = CursorIcon::Hidden;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    }

    if key.just_pressed(KeyCode::Escape) {
        *cursor = CursorIcon::default();
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}
