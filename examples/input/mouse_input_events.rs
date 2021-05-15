use bevy::{input::{ElementState, mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel}}, prelude::*, window::{CursorMoved, WindowId}};

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
        info!("Button: {:?}", event.button);
        info!("State: {:?}", event.state);
    }

    for event in mouse_motion_events.iter() {
        info!("Mouse moved by ({}, {})", event.delta.x, event.delta.y);
    }

    for event in cursor_moved_events.iter() {
        info!("Window id: {:?}", event.id);
        info!("Cursor position: {:?}", event.position);
    }

    for event in mouse_wheel_events.iter() {
        info!("Unit: {:?}", event.unit);
        info!("Moved by {}, {}", event.x, event.y);
    }
}
