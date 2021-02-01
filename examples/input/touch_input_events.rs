use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(touch_event_system.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<TouchInput>,
}

fn touch_event_system(mut state: Local<State>, touch_events: Res<Events<TouchInput>>) {
    for event in state.event_reader.iter(&touch_events) {
        println!("{:?}", event);
    }
}
