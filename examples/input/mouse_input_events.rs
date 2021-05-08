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
        let button: MouseButton = event.button;
        let state: ElementState = event.state;
        info!("Button: {:?}", button);
        info!("State: {:?}", state);
    }

    for event in mouse_motion_events.iter() {
        let movement: Vec2 = event.delta;
        info!("Mouse moved by ({}, {})", movement.x, movement.y);
    }

    for event in cursor_moved_events.iter() {
        let id: WindowId = event.id;
        let position: Vec2 = event.position;

        info!("Window id: {:?}", id);
        info!("Cursor position: {:?}", position);
    }

    for event in mouse_wheel_events.iter() {
        let unit: MouseScrollUnit = event.unit;
        let x: f32 = event.x;
        let y: f32 = event.y;
        info!("Unit: {:?}", unit);
        info!("Moved by {}, {}", x, y);
    }
}
