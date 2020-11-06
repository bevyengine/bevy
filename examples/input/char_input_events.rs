use bevy::{input::text::CharInput, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(print_char_event_system.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<CharInput>,
}

/// This system prints out all char events as they come in
fn print_char_event_system(mut state: Local<State>, char_input_events: Res<Events<CharInput>>) {
    for event in state.event_reader.iter(&char_input_events) {
        println!("{:?}: '{}'", event, event.0);
    }
}
