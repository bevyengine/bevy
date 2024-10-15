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
    mut mouse_button_input_events: EventReader<'_, '_, MouseButtonInput>,
    mut mouse_motion_events: EventReader<'_, '_, MouseMotion>,
    mut cursor_moved_events: EventReader<'_, '_, CursorMoved>,
    mut mouse_wheel_events: EventReader<'_, '_, MouseWheel>,
    mut pinch_gesture_events: EventReader<'_, '_, PinchGesture>,
    mut rotation_gesture_events: EventReader<'_, '_, RotationGesture>,
    mut double_tap_gesture_events: EventReader<'_, '_, DoubleTapGesture>,
) {
    for event in mouse_button_input_events.read() {
        info!("{:?}", event);
    }

    for event in mouse_motion_events.read() {
        info!("{:?}", event);
    }

    for event in cursor_moved_events.read() {
        info!("{:?}", event);
    }

    for event in mouse_wheel_events.read() {
        info!("{:?}", event);
    }

    // This event will only fire on macOS
    for event in pinch_gesture_events.read() {
        info!("{:?}", event);
    }

    // This event will only fire on macOS
    for event in rotation_gesture_events.read() {
        info!("{:?}", event);
    }

    // This event will only fire on macOS
    for event in double_tap_gesture_events.read() {
        info!("{:?}", event);
    }
}
