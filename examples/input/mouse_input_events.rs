//! Prints all mouse events to the console.

use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        touchpad::{TouchpadMagnify, TouchpadRotate},
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
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut touchpad_magnify_events: EventReader<TouchpadMagnify>,
    mut touchpad_rotate_events: EventReader<TouchpadRotate>,
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
    for event in touchpad_magnify_events.read() {
        info!("{:?}", event);
    }

    // This event will only fire on macOS
    for event in touchpad_rotate_events.read() {
        info!("{:?}", event);
    }
}
