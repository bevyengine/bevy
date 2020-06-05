use bevy::{
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
    window::CursorMoved,
};

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<State>()
        .add_system(print_mouse_events_system.system())
        .run();
}

#[derive(Default)]
struct State {
    mouse_button_event_reader: EventReader<MouseButtonInput>,
    mouse_motion_event_reader: EventReader<MouseMotion>,
    cursor_moved_event_reader: EventReader<CursorMoved>,
}

/// This system prints out all mouse events as they come in
fn print_mouse_events_system(
    mut state: ResMut<State>,
    mouse_button_input_events: Res<Events<MouseButtonInput>>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
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
}
