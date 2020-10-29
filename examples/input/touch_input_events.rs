use bevy::{input::devices::touch::*, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(touch_event_system.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<TouchEvent>,
}

fn touch_event_system(mut state: Local<State>, touch_events: Res<Events<TouchEvent>>) {
    for event in state.event_reader.iter(&touch_events) {
        println!("{:?}", event);
    }
}
