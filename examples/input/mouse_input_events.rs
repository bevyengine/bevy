//! Prints all mouse events to the console.

use bevy::{
    input::{
        gestures::*,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, print_mouse_events_system)
        .run();
}

/// This system prints out all mouse events as they come in
fn print_mouse_events_system(
    mut mouse_button_input_reader: MessageReader<MouseButtonInput>,
    mut mouse_motion_reader: MessageReader<MouseMotion>,
    mut cursor_moved_reader: MessageReader<CursorMoved>,
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    mut pinch_gesture_reader: MessageReader<PinchGesture>,
    mut rotation_gesture_reader: MessageReader<RotationGesture>,
    mut double_tap_gesture_reader: MessageReader<DoubleTapGesture>,
) {
    for mouse_button_input in mouse_button_input_reader.read() {
        info!("{:?}", mouse_button_input);
    }

    for mouse_motion in mouse_motion_reader.read() {
        info!("{:?}", mouse_motion);
    }

    for cursor_moved in cursor_moved_reader.read() {
        info!("{:?}", cursor_moved);
    }

    for mouse_wheel in mouse_wheel_reader.read() {
        info!("{:?}", mouse_wheel);
    }

    // This event will only fire on macOS
    for pinch_gesture in pinch_gesture_reader.read() {
        info!("{:?}", pinch_gesture);
    }

    // This event will only fire on macOS
    for rotation_gesture in rotation_gesture_reader.read() {
        info!("{:?}", rotation_gesture);
    }

    // This event will only fire on macOS
    for double_tap_gesture in double_tap_gesture_reader.read() {
        info!("{:?}", double_tap_gesture);
    }
}
