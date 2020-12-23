use bevy::{input::keyboard::KeyboardInput, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(print_keyboard_event_system.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<KeyboardInput>,
}

/// This system prints out all keyboard events as they come in
fn print_keyboard_event_system(
    mut state: Local<State>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
) {
    for event in state.event_reader.iter(&keyboard_input_events) {
        println!("{:?}", event);
    }
}
