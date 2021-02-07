use bevy::{
    input::mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    prelude::*,
    window::CursorMoved,
};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(print_mouse_events_system.system())
        .run();
}

/// This system prints out all mouse events as they come in
fn print_mouse_events_system(
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
) {
    for event in mouse_button_input_events.iter() {
        println!("{:?}", event);
    }

    for event in mouse_motion_events.iter() {
        println!("{:?}", event);
    }

    for event in cursor_moved_events.iter() {
        println!("{:?}", event);
    }

    for event in mouse_wheel_events.iter() {
        println!("{:?}", event);
    }
}
