use bevy::{prelude::*, window::ReceivedCharacter};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(print_char_event_system)
        .run();
}

/// This system prints out all char events as they come in
fn print_char_event_system(mut char_input_events: EventReader<ReceivedCharacter>) {
    for event in char_input_events.iter() {
        info!("{:?}: '{}'", event, event.char);
    }
}
