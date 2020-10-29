use bevy::{input::prelude::*, prelude::*, window::CursorMoved};

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(print_mouse_events_system.system())
        .run();
}

#[derive(Default)]
struct State {
    mouse_button_event_reader: EventReader<MouseButtonEvent>,
    mouse_motion_event_reader: EventReader<MouseMotionEvent>,
    cursor_moved_event_reader: EventReader<CursorMoved>,
    mouse_wheel_event_reader: EventReader<MouseWheelEvent>,
}

/// This system prints out all mouse events as they come in
fn print_mouse_events_system(
    mut state: Local<State>,
    mouse_button_input_events: Res<Events<MouseButtonEvent>>,
    mouse_motion_events: Res<Events<MouseMotionEvent>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mouse_wheel_events: Res<Events<MouseWheelEvent>>,
) {
    for event in state
        .mouse_button_event_reader
        .iter(&mouse_button_input_events)
    {
        println!("{:?}", event);
    }

    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
        println!("{:?}", event);
    }

    for event in state.cursor_moved_event_reader.iter(&cursor_moved_events) {
        println!("{:?}", event);
    }

    for event in state.mouse_wheel_event_reader.iter(&mouse_wheel_events) {
        println!("{:?}", event);
    }
}
