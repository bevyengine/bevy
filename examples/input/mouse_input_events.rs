//! Prints all mouse events to the console.

use bevy::{
    input::mouse::{Magnify, MouseButtonInput, MouseMotion, MouseWheel, Rotate},
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
    mut magnify_events: EventReader<Magnify>,
    mut rotate_events: EventReader<Rotate>,
) {
    for event in mouse_button_input_events.iter() {
        info!("{:?}", event);
    }

    for event in mouse_motion_events.iter() {
        info!("{:?}", event);
    }

    for event in cursor_moved_events.iter() {
        info!("{:?}", event);
    }

    for event in mouse_wheel_events.iter() {
        info!("{:?}", event);
    }

    // This event will only fire on macOS
    for event in magnify_events.iter() {
        info!("{:?}", event);
    }

    // This event will only fire on macOS
    for event in rotate_events.iter() {
        info!("{:?}", event);
    }
}
