use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(print_keyboard_event_system.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<KeyboardEvent>,
}

/// This system prints out all keyboard events as they come in
fn print_keyboard_event_system(
    mut state: Local<State>,
    keyboard_input_events: Res<Events<KeyboardEvent>>,
) {
    for event in state.event_reader.iter(&keyboard_input_events) {
        println!("{:?}", event);
    }
}
