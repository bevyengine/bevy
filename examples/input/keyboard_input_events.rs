use bevy::{input::keyboard::KeyboardInput, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(print_keyboard_event_system.system())
        .run();
}

/// This system prints out all keyboard events as they come in
fn print_keyboard_event_system(mut keyboard_input_events: EventReader<KeyboardInput>) {
    for event in keyboard_input_events.iter() {
        println!("{:?}", event);
    }
}
